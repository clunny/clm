//! `clm build start`, `step` and `launched`: reading the snapshot, preparing the runs the plan
//! launches and printing the actions (`step-actions`, `loop-start`).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use clm_git::{Failure, now};
use clm_orchestrator::DEFAULT_EXPIRY_HOURS;

use crate::Options;
use crate::cmd::{clm, clm_in, clm_ok, say};
use crate::plan::{BacklogRow, Disk, Group, Launch, Snapshot, plan_of};
use crate::state::{self, Kind, LoopFile, Run, RunState, StopMode};
use crate::stops;

/// The prompt file of a run: `.clm/prompts/loop/<name>.md`, with `:` written `-`.
fn prompt_path(options: &Options, name: &str) -> Result<PathBuf, Failure> {
    let root = std::path::absolute(&options.root)
        .map_err(|e| Failure::Git(format!("{}: {e}", options.root.display())))?;
    let dir = root.join(".clm/prompts/loop");
    fs::create_dir_all(&dir).map_err(|e| Failure::Git(format!("{}: {e}", dir.display())))?;
    Ok(dir.join(format!("{}.md", name.replace(':', "-"))))
}

/// Writes a run's prompt file: the line `loop run <name>`, then the launcher as its command
/// printed it, then the human's notes for the loop when they hold more than white space
/// (`notes-reach-every-launch`).
fn write_prompt(
    options: &Options,
    name: &str,
    launcher: &str,
    notes: &str,
) -> Result<String, Failure> {
    let path = prompt_path(options, name)?;
    let abilities = clm_agent_harness::abilities(options.agent_harness);
    let mut text = format!("loop run {name}\n{launcher}");
    if !text.ends_with('\n') {
        text.push('\n');
    }
    if !abilities.subagents {
        text.push_str(
            "\nThis agent harness has no subagents: run the prompt above in the session itself.\n",
        );
    }
    if !abilities.isolated_worktrees {
        let worktree = add_worktree(options, name)?;
        text.push_str(&format!(
            "\nThis agent harness makes no worktree for a subagent: work in {}, a worktree of origin/main the loop added for this run.\n",
            worktree.display()
        ));
    }
    if !notes.trim().is_empty() {
        text.push_str("\n## The human's notes for this loop\n\n");
        text.push_str(notes);
    }
    fs::write(&path, text).map_err(|e| Failure::Git(format!("{}: {e}", path.display())))?;
    Ok(path.display().to_string())
}

/// The worktree of a run at `.claude/worktrees/agent-<name>`, `:` written `-`, added from
/// `origin/main` unless it stands (`loop-follows-agent-harness`).
fn add_worktree(options: &Options, name: &str) -> Result<PathBuf, Failure> {
    let root = std::path::absolute(&options.root)
        .map_err(|e| Failure::Git(format!("{}: {e}", options.root.display())))?;
    let path = root
        .join(".claude/worktrees")
        .join(format!("agent-{}", name.replace(':', "-")));
    if !path.exists() {
        let dir = path.display().to_string();
        clm_git::git(
            &options.root,
            &["worktree", "add", "--detach", &dir, "origin/main"],
        )?;
    }
    Ok(path)
}

/// `clm next`'s components, in its order.
fn read_ready(options: &Options) -> Result<Vec<String>, Failure> {
    Ok(clm_ok(options, &["next"])?
        .lines()
        .filter_map(|l| l.split_whitespace().next().map(str::to_string))
        .collect())
}

/// Each component's priority from `clm view flight --at origin/main --format json`.
fn read_priorities(options: &Options) -> Result<BTreeMap<String, String>, Failure> {
    let json = clm_ok(
        options,
        &["view", "flight", "--at", "origin/main", "--format", "json"],
    )?;
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| Failure::Git(format!("clm view flight: unreadable JSON: {e}")))?;
    let nodes = value.get("nodes").and_then(|n| n.as_array());
    Ok(nodes
        .into_iter()
        .flatten()
        .filter_map(|n| {
            let node = n.get("node")?.as_str()?;
            let priority = n.get("priority")?.as_str()?;
            Some((node.to_string(), priority.to_string()))
        })
        .collect())
}

/// The `group` lines of `clm triage`: `group <key> [<components>] <P> <n> <slug>...`.
fn read_groups(options: &Options) -> Result<Vec<Group>, Failure> {
    Ok(clm_ok(options, &["triage"])?
        .lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            (words.next()? == "group").then_some(())?;
            let key = words.next()?.to_string();
            let _components = words.next()?;
            let priority = words.next()?.to_string();
            let _releases = words.next()?;
            let items: Vec<String> = words.map(str::to_string).collect();
            Some(Group {
                key,
                priority,
                items,
            })
        })
        .collect())
}

/// Reads what the plan is computed from, after the step's fetch.
fn read_snapshot(options: &Options, file: LoopFile, wrap_up: &str) -> Result<Snapshot, Failure> {
    let main = clm_git::git(&options.root, &["rev-parse", "origin/main"])?;
    let held = held_places(options)?.len() as u64;
    let ready = read_ready(options)?;
    let priorities = if ready.is_empty() {
        BTreeMap::new()
    } else {
        read_priorities(options)?
    };
    let abilities = clm_agent_harness::abilities(options.agent_harness);
    // Where the agent harness can't choose a subagent's model, every run is on the session's.
    let build_model = match abilities.build_model {
        Some(model) if abilities.model_choice => model.to_string(),
        _ => file.session_model.clone(),
    };
    Ok(Snapshot {
        now: now(),
        main,
        free_places: options.max_builds.saturating_sub(held),
        ready,
        priorities,
        groups: read_groups(options)?,
        build_model,
        one_at_a_time: !abilities.subagents,
        file,
        backlog: read_backlog(options)?,
        unsafe_work: unsafe_work(wrap_up),
        upgrades: read_upgrades(options)?,
        complete: wrap_up
            .lines()
            .filter_map(|l| Some(l.trim().strip_prefix("complete ")?.trim().to_string()))
            .collect(),
        archive_branches: archive_branches(options)?,
        disk: read_disk(options),
        verify: None,
        left: wrap_up
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("left "))
            .map(str::to_string)
            .collect(),
    })
}

/// The items of `origin/main`'s backlog the plan reads: those that block requirements or took
/// in a red verify, with, for a question or human task, when it was added.
fn read_backlog(options: &Options) -> Result<Vec<BacklogRow>, Failure> {
    let blobs = clm_git::tree_blobs(&options.root, "origin/main", &|path| {
        path.starts_with("backlog/") && path.ends_with(".md")
    })?;
    let mut rows = Vec::new();
    for (path, bytes) in blobs {
        let slug = path
            .trim_start_matches("backlog/")
            .trim_end_matches(".md")
            .to_string();
        let Ok(item) = clm_model::parse_backlog_item(&slug, &String::from_utf8_lossy(&bytes))
        else {
            continue;
        };
        let fm = item.frontmatter;
        let human = matches!(
            fm.kind,
            clm_model::BacklogKind::Question | clm_model::BacklogKind::Human
        );
        if !(human || !fm.verify_failures.is_empty()) {
            continue;
        }
        let since = clm_git::git(
            &options.root,
            &[
                "log",
                "--diff-filter=A",
                "--format=%ct",
                "origin/main",
                "--",
                &path,
            ],
        )?
        .lines()
        .last()
        .and_then(|t| t.parse().ok())
        .unwrap_or_else(now);
        rows.push(BacklogRow {
            slug,
            kind: fm.kind.to_string(),
            blocks: fm.blocks,
            verify_failures: fm.verify_failures,
            since,
        });
    }
    Ok(rows)
}

/// The lines of `clm wrap-up` about the caller's own work that isn't safe and the ledger's
/// findings; the lines about other checkouts' work (`elsewhere`) and runs' (`open`) are left out.
fn unsafe_work(wrap_up: &str) -> Vec<String> {
    wrap_up
        .lines()
        .map(str::trim)
        .filter(|l| {
            l.starts_with("dirty ")
                || l.starts_with("unpushed ")
                || l.split(": ").any(|part| part.starts_with("ledger/"))
        })
        .map(str::to_string)
        .collect()
}

/// The migrations `clm upgrade` lists, by ID.
fn read_upgrades(options: &Options) -> Result<Vec<String>, Failure> {
    let ran = clm(options, &["upgrade"])?;
    if ran.code != 0 {
        return Ok(Vec::new());
    }
    Ok(ran
        .stdout
        .lines()
        .filter_map(|l| {
            let mut words = l.split_whitespace();
            let id = words.next()?;
            matches!(words.next()?, "required" | "advised").then(|| id.to_string())
        })
        .collect())
}

/// The disk from `clm machine --format json`, or none when it fails or prints what doesn't parse,
/// so a reading that fails holds nothing back (`plan-order`).
fn read_disk(options: &Options) -> Option<Disk> {
    let ran = clm(options, &["machine", "--format", "json"]).ok()?;
    if ran.code != 0 {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(&ran.stdout).ok()?;
    let disk = value.get("disk")?;
    let room = match disk.get("room")? {
        serde_json::Value::Null => None,
        room => Some(room.as_u64()?),
    };
    Some(Disk {
        room,
        state: disk.get("state")?.as_str()?.to_string(),
    })
}

/// The IDs of the `impl/archive/*` branches on `origin`.
fn archive_branches(options: &Options) -> Result<Vec<String>, Failure> {
    Ok(clm_git::git(
        &options.root,
        &[
            "for-each-ref",
            "--format=%(refname)",
            "refs/remotes/origin/impl/archive/",
        ],
    )?
    .lines()
    .filter_map(|r| r.strip_prefix("refs/remotes/origin/impl/archive/"))
    .map(str::to_string)
    .collect())
}

/// The prompt of the subagent that archives the complete change `id` (`plan-archive`).
fn archive_prompt(id: &str) -> String {
    format!(
        "Archive the complete change `{id}`, in your own checkout.\n\n\
1. Cut the branch: `git switch -c impl/archive/{id} origin/main`. When `origin` already has \
`impl/archive/{id}`, continue it instead: `git switch -c impl/archive/{id} origin/impl/archive/{id}`.\n\
2. Run `clm rebase` until its last line is `rebase ok`, resolving what it reports and \
committing each fix.\n\
3. Push with `clm branch push`. When the push is refused because the branch moved on `origin`, \
another session is archiving it: leave it to that session and stop.\n\
4. Open the pull request with `clm pr open`, then land with `clm land`.\n\n\
End with one line: `landed <sha>` or `stopped: <reason>`.\n"
    )
}

/// What preparing one launch came to.
enum Prepared {
    /// A run, planned, to record.
    Run(Box<Run>),
    /// An orphan to drop for the plan to compute again.
    Drop(String),
    /// Nothing: the command refused, or the orphan needs no preparing.
    Nothing,
    /// The clone has no free place: launch nothing more this step.
    NoPlace,
    /// An orphan whose relaunch the command refused, with what it printed on stderr.
    Refused(String, String),
}

fn planned(name: &str, kind: Option<Kind>, model: &str, prompt: Option<String>) -> Run {
    Run {
        name: name.to_string(),
        kind,
        model: model.to_string(),
        state: RunState::Planned,
        task_id: None,
        started: now(),
        round: 1,
        stops: BTreeMap::new(),
        orphan: false,
        items: Vec::new(),
        prompt,
        command: None,
        landed: false,
        resume: None,
        resumed: false,
        serial: false,
        rebuilt_for: None,
        log: Vec::new(),
        escalated: None,
        hold: None,
    }
}

/// Runs `clm run` or `clm draft` for a run and writes its prompt file.
fn launch_run(
    options: &Options,
    mut run: Run,
    args: &[&str],
    relaunch: bool,
    notes: &str,
) -> Result<Prepared, Failure> {
    let ran = clm(options, args)?;
    match ran.code {
        0 => {
            if run.kind == Some(Kind::Draft) && !relaunch {
                // A new draft is named by the change ID `clm draft` chose.
                let id = ran
                    .stderr
                    .lines()
                    .rev()
                    .find_map(|l| l.trim().strip_prefix("draft "))
                    .ok_or_else(|| {
                        Failure::Git("clm draft: no `draft <id>` line on stderr".to_string())
                    })?;
                run.name = format!("draft:{id}");
            }
            run.prompt = Some(write_prompt(options, &run.name, &ran.stdout, notes)?);
            Ok(Prepared::Run(Box::new(run)))
        }
        3 => Ok(Prepared::NoPlace),
        // An orphan the command refuses has no subagent and no way to get one (`loop-start`).
        1 if relaunch => Ok(Prepared::Refused(run.name, ran.reason())),
        1 => {
            eprintln!("clm build: skipped {}: {}", run.name, ran.reason());
            Ok(Prepared::Nothing)
        }
        _ => Err(Failure::Git(format!(
            "clm {}: {}",
            args.join(" "),
            ran.reason()
        ))),
    }
}

fn prepare(
    options: &Options,
    file: &LoopFile,
    launch: &Launch,
    notes: &str,
) -> Result<Prepared, Failure> {
    match launch {
        Launch::Build { component, model } => {
            let run = planned(
                &format!("build:{component}"),
                Some(Kind::Build),
                model,
                None,
            );
            launch_run(
                options,
                run,
                &["run", component, "--review-model", &file.session_model],
                false,
                notes,
            )
        }
        Launch::Draft { key, items, model } => {
            // Named by its group until `clm draft` names its change.
            let mut run = planned(&format!("draft:{key}"), Some(Kind::Draft), model, None);
            run.items = items.clone();
            let mut args = vec!["draft"];
            args.extend(items.iter().map(String::as_str));
            launch_run(options, run, &args, false, notes)
        }
        Launch::Archive { id, model } => {
            let name = format!("archive:{id}");
            let prompt = write_prompt(options, &name, &archive_prompt(id), notes)?;
            Ok(Prepared::Run(Box::new(planned(
                &name,
                Some(Kind::Archive),
                model,
                Some(prompt),
            ))))
        }
        Launch::Fold { ids, model } => {
            let first = ids.first().cloned().unwrap_or_default();
            let mut run = planned(&format!("fold:{first}"), Some(Kind::Fold), model, None);
            run.items = ids.clone();
            let mut args = vec!["draft", "--fold"];
            args.extend(ids.iter().map(String::as_str));
            launch_run(options, run, &args, false, notes)
        }
        Launch::Land { batch } => {
            let mut run = planned(&format!("land:{batch}"), None, "", None);
            run.command = Some(format!("clm land spec/{batch}"));
            Ok(Prepared::Run(Box::new(run)))
        }
        Launch::Verify { work_done } => {
            let mut run = planned("verify", None, "", None);
            run.command = Some(if *work_done {
                "clm verify --due --event work-done".to_string()
            } else {
                "clm verify --due".to_string()
            });
            Ok(Prepared::Run(Box::new(run)))
        }
        Launch::Relaunch(name) => {
            let Some(orphan) = file.run(name) else {
                return Ok(Prepared::Nothing);
            };
            let mut run = orphan.clone();
            run.state = RunState::Planned;
            run.orphan = false;
            run.task_id = None;
            match (run.kind, &run.command) {
                (Some(Kind::Build), _) => {
                    let component = name.strip_prefix("build:").unwrap_or(name);
                    launch_run(
                        options,
                        run,
                        &["run", component, "--review-model", &file.session_model],
                        true,
                        notes,
                    )
                }
                (Some(Kind::Draft), _) => {
                    let id = name.strip_prefix("draft:").unwrap_or(name);
                    launch_run(options, run, &["draft", "--resume", id], true, notes)
                }
                (Some(Kind::Fold | Kind::Archive), _) => Ok(Prepared::Drop(name.clone())),
                (None, _) => Ok(Prepared::Run(Box::new(run))),
            }
        }
    }
}

/// Whether a build's claim has left `origin`, read on the remote-tracking refs as they stand: no
/// `origin/impl/<component>`, or one whose tip is on `origin/main` (`claim-gone-ends-a-build`).
fn claim_gone(options: &Options, component: &str) -> bool {
    let branch = format!("refs/remotes/origin/impl/{component}");
    if clm_git::git(&options.root, &["rev-parse", "--verify", "-q", &branch]).is_err() {
        return true;
    }
    clm_git::is_ancestor(&options.root, &branch, "refs/remotes/origin/main").unwrap_or(false)
}

/// The running builds of the loop file whose claim has left `origin`, as run names.
pub(crate) fn gone_builds(options: &Options, file: &LoopFile) -> Vec<String> {
    file.runs
        .iter()
        .filter(|r| r.kind == Some(Kind::Build) && r.state == RunState::Running)
        .filter_map(|r| {
            let component = r.name.strip_prefix("build:")?;
            claim_gone(options, component).then(|| r.name.clone())
        })
        .collect()
}

/// Finishes each running build whose claim has left `origin`, as a run that returned with no
/// closing line; the `say` actions its finishes leave (`claim-gone-ends-a-build`).
fn finish_gone_builds(options: &Options) -> Result<Vec<String>, Failure> {
    let Some(file) = state::read(&options.root) else {
        return Ok(Vec::new());
    };
    let mut says = Vec::new();
    for name in gone_builds(options, &file) {
        says.extend(crate::returned::finish_unclosed(options, &name)?.1);
    }
    Ok(says)
}

/// The drafts of the loop file no session holds (planned, or orphans) that name items, none of
/// which `origin/main`'s backlog holds, and that pushed no `spec/<change-id>`, as change IDs
/// (`draft-of-gone-items-dropped`).
fn gone_drafts(options: &Options, file: &LoopFile) -> Result<Vec<String>, Failure> {
    let candidates: Vec<&Run> = file
        .runs
        .iter()
        .filter(|r| r.kind == Some(Kind::Draft) && !r.items.is_empty())
        .filter(|r| r.state == RunState::Planned || r.orphan)
        .collect();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let backlog = clm_git::git(
        &options.root,
        &[
            "ls-tree",
            "-r",
            "--name-only",
            "refs/remotes/origin/main",
            "--",
            "backlog/",
        ],
    )?;
    let held: Vec<&str> = backlog.lines().collect();
    let mut gone = Vec::new();
    for r in candidates {
        let items_gone = r
            .items
            .iter()
            .all(|slug| !held.contains(&format!("backlog/{slug}.md").as_str()));
        let Some(id) = r.name.strip_prefix("draft:").filter(|_| items_gone) else {
            continue;
        };
        // Asked of `origin` itself: the fetch doesn't prune, so a remote-tracking ref may name a
        // branch since deleted. A failure to ask keeps the draft.
        let spec = format!("refs/heads/spec/{id}");
        if clm_git::git(&options.root, &["ls-remote", "--heads", "origin", &spec])?.is_empty() {
            gone.push(id.to_string());
        }
    }
    Ok(gone)
}

/// Finishes each draft whose items are gone with `clm draft --finish` and takes it off the loop
/// file's list, recording none of its items as left; the `say` actions for the `left <path>`
/// lines its finish prints (`draft-of-gone-items-dropped`).
fn finish_gone_drafts(options: &Options) -> Result<Vec<String>, Failure> {
    let Some(file) = state::read(&options.root) else {
        return Ok(Vec::new());
    };
    let mut says = Vec::new();
    for id in gone_drafts(options, &file)? {
        let ran = clm(options, &["draft", "--finish", &id])?;
        if ran.code == 2 {
            return Err(Failure::Git(format!(
                "clm draft --finish {id}: {}",
                ran.reason()
            )));
        }
        for left in ran.stdout.lines().map(str::trim) {
            // A path, never an item: `left <slug>:` lines name items, which aren't recorded.
            if let Some(rest) = left.strip_prefix("left ")
                && rest.split(':').next().is_some_and(|p| p.contains('/'))
            {
                says.push(say(options, left)?);
            }
        }
        let name = format!("draft:{id}");
        // `update` keeps the run's task ID under its name.
        state::update(&options.root, |f| {
            f.runs.retain(|r| r.name != name);
            Ok(())
        })?;
    }
    Ok(says)
}

/// Runs the housekeeping and reads the snapshot, after a fetch the caller made.
pub(crate) fn fresh_snapshot(options: &Options) -> Result<Snapshot, Failure> {
    // `clm wrap-up` removes landed worktrees and branches and frees the places that don't
    // count; its report is for the housekeeping requirements, which come later.
    let wrap_up = clm_in(options, &options.cwd, &["wrap-up"])?;
    if wrap_up.code == 2 {
        return Err(Failure::Git(format!("clm wrap-up: {}", wrap_up.reason())));
    }
    let file = state::read(&options.root).unwrap_or_default();
    let mut snapshot = read_snapshot(options, file, &wrap_up.stdout)?;
    snapshot.verify = verify_due(options, &snapshot)?;
    Ok(snapshot)
}

/// Whether a verify run is due, asked after a run of the loop landed and when the loop is idle,
/// and whether it was asked with the idle event (`plan-verify`).
fn verify_due(options: &Options, s: &Snapshot) -> Result<Option<bool>, Failure> {
    let file = &s.file;
    if file.stop == Some(StopMode::Now)
        || file
            .runs
            .iter()
            .any(|r| r.name == "verify" && r.state != RunState::Returned)
    {
        return Ok(None);
    }
    let idle = !file.runs.iter().any(|r| {
        matches!(r.state, RunState::Planned | RunState::Running) || r.resume.is_some() || r.orphan
    }) && plan_of(s).launches.is_empty();
    if !idle && !file.landed_since_verify {
        return Ok(None);
    }
    let mut args = vec!["verify", "--due", "--dry-run"];
    if idle {
        args.extend(["--event", "work-done"]);
    }
    let ran = clm(options, &args)?;
    Ok((ran.code == 0 && ran.stdout.trim_start().starts_with("verify due")).then_some(idle))
}

/// How long an orphan's branch must stay still before a session taken over is judged gone.
const STILL_SECONDS: u64 = 15 * 60;

/// The branch an orphan's work is on, on `origin`.
fn run_branch(run: &Run) -> Option<String> {
    match run.kind? {
        Kind::Build => Some(format!("impl/{}", run.name.strip_prefix("build:")?)),
        Kind::Draft => Some(format!("spec/{}", run.name.strip_prefix("draft:")?)),
        Kind::Fold | Kind::Archive => None,
    }
}

/// The tip of `origin/<branch>` and when it was committed, Unix seconds; none when it isn't
/// there.
fn tip(options: &Options, branch: &str) -> Option<(String, u64)> {
    let found = clm_git::git(
        &options.root,
        &[
            "log",
            "-1",
            "--format=%H %ct",
            &format!("refs/remotes/origin/{branch}"),
        ],
    )
    .ok()?;
    let (sha, time) = found.split_once(' ')?;
    Some((sha.to_string(), time.parse().ok()?))
}

/// Keeps each hold up to date: a branch that moved restarts its quiet time, and one that stayed
/// at the same commit for 15 minutes lets its orphan go (`take-over-holds-moving-orphans`).
pub(crate) fn refresh_holds(options: &Options) -> Result<(), Failure> {
    let Some(file) = state::read(&options.root) else {
        return Ok(());
    };
    let seen: Vec<(String, Option<(String, u64)>)> = file
        .runs
        .iter()
        .filter(|r| r.hold.is_some())
        .map(|r| (r.name.clone(), run_branch(r).and_then(|b| tip(options, &b))))
        .collect();
    if seen.is_empty() {
        return Ok(());
    }
    let at = now();
    state::update(&options.root, |f| {
        for (name, tip) in seen {
            let Some(run) = f.run_mut(&name) else {
                continue;
            };
            let Some(hold) = run.hold.as_mut() else {
                continue;
            };
            match tip {
                Some((sha, _)) if sha != hold.tip => {
                    hold.tip = sha;
                    hold.since = at;
                }
                Some(_) if at.saturating_sub(hold.since) < STILL_SECONDS => {}
                _ => run.hold = None,
            }
        }
        Ok(())
    })
}

/// Lands again each escalation an earlier land of its item failed; true when one landed.
fn retry_escalations(options: &Options) -> Result<bool, Failure> {
    let Some(file) = state::read(&options.root) else {
        return Ok(false);
    };
    let mut landed = Vec::new();
    for escalation in &file.escalations {
        if crate::escalate::land(options, &escalation.slug, &escalation.body).is_ok() {
            landed.push(escalation.slug.clone());
        }
    }
    if landed.is_empty() {
        return Ok(false);
    }
    state::update(&options.root, |f| {
        f.escalations.retain(|e| !landed.contains(&e.slug));
        Ok(())
    })?;
    Ok(true)
}

/// `clm build approved <batch>`: the human approved a fold batch through `clm approve`, so the
/// session may land it.
pub fn approved(options: &Options, batch: &str) -> Result<Vec<String>, Failure> {
    let notes = state::notes(&options.root)?;
    clm_git::fetch(&options.root)?;
    state::update(&options.root, |f| {
        f.lease = Some(now());
        let found = f
            .batches
            .iter_mut()
            .find(|b| b.id == batch)
            .ok_or_else(|| Failure::Refused(format!("the loop has no fold batch {batch}")))?;
        found.approved = true;
        Ok(())
    })?;
    actions(options, &notes)
}

/// `clm build cleared disk`: the human freed the disk, so the runs it stopped may resume.
pub fn cleared(options: &Options, what: &str) -> Result<Vec<String>, Failure> {
    if what != "disk" {
        return Err(Failure::Refused(format!(
            "`{what}` isn't a condition the human clears: say `disk`"
        )));
    }
    let notes = state::notes(&options.root)?;
    clm_git::fetch(&options.root)?;
    state::update(&options.root, |f| {
        f.lease = Some(now());
        f.conditions.disk = false;
        Ok(())
    })?;
    actions(options, &notes)
}

/// The names of the clone's places that count.
pub(crate) fn held_places(options: &Options) -> Result<Vec<String>, Failure> {
    Ok(
        clm_orchestrator::places(&options.root, DEFAULT_EXPIRY_HOURS)
            .map_err(|e| Failure::Git(format!("places: {e:?}")))?
            .into_iter()
            .filter(|p| p.counts)
            .map(|p| p.name)
            .collect(),
    )
}

/// The actions to take now: fetch, do the housekeeping, read the snapshot, compute the plan,
/// prepare each run it launches, record them and print one line per action.
fn actions(options: &Options, notes: &str) -> Result<Vec<String>, Failure> {
    clm_git::fetch(&options.root)?;
    state::update(&options.root, |f| {
        f.lease = Some(now());
        Ok(())
    })?;
    if retry_escalations(options)? {
        clm_git::fetch(&options.root)?;
    }
    refresh_holds(options)?;
    // Before the housekeeping, which removes a landed build's local branch.
    let mut say_lines = finish_gone_builds(options)?;
    say_lines.extend(finish_gone_drafts(options)?);
    let snapshot = fresh_snapshot(options)?;
    let plan = plan_of(&snapshot);
    // Each `left` line of the wrap-up is said once.
    let mut said = Vec::new();
    for left in &snapshot.left {
        if !snapshot.file.said.contains(left) && !said.contains(left) {
            said.push(left.clone());
        }
    }
    for left in &said {
        say_lines.push(say(options, left)?);
    }
    let mut changes = Vec::new();
    let mut stored_file = snapshot.file.clone();
    for launch in &plan.launches {
        match prepare(options, &stored_file, launch, notes)? {
            Prepared::NoPlace => break,
            Prepared::Nothing => {}
            Prepared::Refused(name, reason) => {
                let (landed, says) = crate::returned::finish_unclosed(options, &name)?;
                // Its task is an earlier run's now, which `update` keeps under its name.
                state::update(&options.root, |f| {
                    if let Some(r) = f.run_mut(&name) {
                        r.task_id = None;
                    }
                    Ok(())
                })?;
                say_lines.extend(says);
                if !landed {
                    say_lines.push(say(
                        options,
                        &format!("{name} wasn't relaunched: {reason}"),
                    )?);
                }
                stored_file.runs.retain(|r| r.name != name);
            }
            Prepared::Run(run) => {
                // Later launches of this step see the runs this one made.
                stored_file.runs.retain(|r| r.name != run.name);
                stored_file.runs.push((*run).clone());
                changes.push(Prepared::Run(run));
            }
            Prepared::Drop(name) => {
                stored_file.runs.retain(|r| r.name != name);
                changes.push(Prepared::Drop(name));
            }
        }
    }
    let main = snapshot.main.clone();
    let next = plan.next.clone();
    let humans = plan.humans.clone();
    let no_launches = plan.launches.is_empty();
    let (lines, ended, file) = state::update(&options.root, |f| {
        for change in changes {
            match change {
                Prepared::Run(run) => {
                    if run.kind == Some(Kind::Fold) {
                        // The changes the fold takes leave the fold list.
                        f.review.retain(|id| !run.items.contains(id));
                    }
                    match f.run_mut(&run.name) {
                        Some(existing) => *existing = *run,
                        None => f.runs.push(*run),
                    }
                }
                Prepared::Drop(name) => f.runs.retain(|r| r.name != name),
                Prepared::Nothing | Prepared::NoPlace | Prepared::Refused(..) => {}
            }
        }
        f.said.extend(said);
        // The runs a stop paused go on once what stopped them has cleared, which is also what a
        // plan reads from the loop file as it stands.
        let at = now();
        if f.conditions.limit_until.is_some_and(|until| at >= until) {
            f.conditions.limit_until = None;
        }
        let mut resumes = Vec::new();
        if f.stop != Some(StopMode::Now) {
            let serial = f.conditions.serial;
            for name in stops::resumable(f, at) {
                let Some(run) = f.run_mut(&name) else {
                    continue;
                };
                let (Some(pending), Some(task)) = (run.resume.take(), run.task_id.clone()) else {
                    continue;
                };
                let message = crate::returned::message_file(
                    options,
                    &name,
                    &stops::resume_message(run, &pending),
                )?;
                run.state = RunState::Running;
                run.resumed = true;
                run.serial = serial && matches!(pending.cause.as_str(), "limit" | "disk");
                resumes.push(format!("resume {name} {task} {}", message.display()));
            }
        }
        if f.conditions.serial && !stops::serial_hold(f) {
            f.conditions.serial = false;
        }
        f.plan = Some(state::StoredPlan {
            main,
            at: now(),
            next,
            humans,
        });
        let mut lines: Vec<String> = if f.stop == Some(StopMode::Now) {
            // A run never launched is stopped with its claim, branch and place kept.
            for run in f.runs.iter_mut().filter(|r| r.state == RunState::Planned) {
                run.state = RunState::Stopped;
            }
            f.runs
                .iter()
                .filter(|r| r.state == RunState::Running)
                .filter_map(|r| Some(format!("stop {} {}", r.name, r.task_id.as_ref()?)))
                .collect()
        } else {
            f.runs
                .iter()
                .filter(|r| r.state == RunState::Planned)
                .filter_map(action_line)
                .collect::<Vec<_>>()
        };
        lines.extend(resumes);
        let busy = f.runs.iter().any(|r| {
            matches!(r.state, RunState::Planned | RunState::Running)
                || r.resume.is_some()
                || r.orphan
        });
        let ended = f.stop.is_some() && !busy && no_launches;
        if ended {
            // The loop is over: the next `start` takes the lease at once.
            f.lease = None;
        }
        Ok((lines, ended, f.clone()))
    })?;
    if ended {
        // The notes were the loop's: the next one starts with none.
        state::remove_notes(&options.root)?;
        let text = crate::summary::summary(options, &file, &plan.view)?;
        let mut out = say_lines;
        out.push(say(options, &text)?);
        out.push("end".to_string());
        return Ok(out);
    }
    let mut out = say_lines;
    out.extend(lines);
    if out.is_empty() {
        Ok(vec!["wait".to_string()])
    } else {
        Ok(out)
    }
}

/// The action a planned run is printed as.
fn action_line(run: &Run) -> Option<String> {
    match (run.kind, &run.command, &run.prompt) {
        (Some(kind), _, Some(prompt)) => Some(format!(
            "launch {} {} {} {prompt}",
            run.name,
            kind.word(),
            run.model
        )),
        (None, Some(command), _) => Some(format!("run {} {command}", run.name)),
        _ => None,
    }
}

/// The actions a `returned` or `release` prints after it has recorded what it was given, with
/// the notes it read before it recorded anything.
pub(crate) fn next_actions(options: &Options, notes: &str) -> Result<Vec<String>, Failure> {
    actions(options, notes)
}

/// `clm build step`: the actions to take now.
pub fn step(options: &Options) -> Result<Vec<String>, Failure> {
    let notes = state::notes(&options.root)?;
    actions(options, &notes)
}

/// `clm build start --session-model <alias> [--take-over]`: begins or adopts the loop, keeping
/// its notes as they are.
pub fn start(
    options: &Options,
    session_model: &str,
    take_over: bool,
) -> Result<Vec<String>, Failure> {
    let notes = state::notes(&options.root)?;
    begin(options, session_model, take_over, &notes, false)
}

/// `clm build start --session-model <alias> [--take-over] --notes <file>`: begins or adopts the
/// loop with the human's notes in `notes`, a path taken against `options.cwd`, replacing any it
/// held without reading them (`notes-reach-every-launch`).
pub fn start_with_notes(
    options: &Options,
    session_model: &str,
    take_over: bool,
    notes: &std::path::Path,
) -> Result<Vec<String>, Failure> {
    let path = options.cwd.join(notes);
    let text = fs::read(&path)
        .map_err(|e| e.to_string())
        .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "not UTF-8 text".to_string()))
        .map_err(|why| Failure::Refused(format!("--notes {}: {why}", notes.display())))?;
    begin(options, session_model, take_over, &text, true)
}

/// `start`'s body: `keep` writes `notes` as the loop's once the lease is taken.
fn begin(
    options: &Options,
    session_model: &str,
    take_over: bool,
    notes: &str,
    keep: bool,
) -> Result<Vec<String>, Failure> {
    clm_git::fetch(&options.root)?;
    let main = clm_git::git(&options.root, &["rev-parse", "origin/main"])?;
    let at = now();
    // A session taken over may still run its subagents: an orphan whose branch advanced within
    // the 15 minutes before the fetch is held until the branch has stayed still that long.
    let mut holds = BTreeMap::new();
    if take_over && let Some(file) = state::read(&options.root) {
        for run in file.runs.iter().filter(|r| {
            matches!(
                r.state,
                RunState::Planned | RunState::Running | RunState::Stopped
            )
        }) {
            if let Some((sha, committed)) = run_branch(run).and_then(|b| tip(options, &b))
                && at.saturating_sub(committed) < STILL_SECONDS
            {
                holds.insert(
                    run.name.clone(),
                    state::Hold {
                        tip: sha,
                        since: committed,
                    },
                );
            }
        }
    }
    state::update(&options.root, |f| {
        if f.held(at) && !take_over {
            let ago = at.saturating_sub(f.lease.unwrap_or(at)) / 60;
            return Err(Failure::Refused(format!(
                "a loop holds this clone: its lease was last renewed {ago}m ago; \
                 `clm view queue` shows it, and --take-over takes it when its session is gone"
            )));
        }
        f.lease = Some(at);
        f.started_main = Some(main);
        f.session_model = session_model.to_string();
        f.stop = None;
        // The summary tells what the runs landed since this start.
        f.runs.retain(|r| r.state != RunState::Returned);
        for run in &mut f.runs {
            if matches!(
                run.state,
                RunState::Planned | RunState::Running | RunState::Stopped
            ) {
                run.orphan = true;
                run.hold = holds.remove(&run.name);
            }
        }
        Ok(())
    })?;
    if keep {
        state::set_notes(&options.root, notes)?;
    }
    actions(options, notes)
}

/// `clm build launched <name> <task-id>`: records the task ID of a run the session launched.
pub fn launched(options: &Options, name: &str, task_id: &str) -> Result<(), Failure> {
    state::update(&options.root, |f| {
        let listed = f
            .run(name)
            .ok_or_else(|| Failure::Refused(format!("the loop has no run {name}")))?;
        if listed.state == RunState::Returned {
            return Err(Failure::Refused(format!(
                "the loop has finished run {name}: it returned, and no launch of it is waiting"
            )));
        }
        if listed.task_id.as_deref() != Some(task_id)
            && let Some((owner, earlier)) = f.task_owner(task_id)
        {
            let new_run = match listed.kind {
                Some(_) => "to be started as a new subagent",
                None => "to be run as a new command",
            };
            let whose = if earlier && owner == name {
                format!(
                    "task {task_id} was an earlier run of {name}'s, which the loop has finished; \
                     a late `landed` report of it changes nothing"
                )
            } else if earlier {
                format!("task {task_id} was an earlier run of {owner}'s")
            } else {
                format!("task {task_id} is the task of {owner}")
            };
            return Err(Failure::Refused(format!(
                "{whose}. The action printed for {name} is a new run, {new_run} and reported \
                 with its own task ID"
            )));
        }
        f.lease = Some(now());
        let run = f.run_mut(name).expect("listed above");
        run.state = RunState::Running;
        // A task reported over another: `update` keeps the other under the run's name.
        run.task_id = Some(task_id.to_string());
        Ok(())
    })
}

/// `clm build stop --drain|--now`: records the request. It fetches nothing.
pub fn stop(options: &Options, mode: StopMode) -> Result<(), Failure> {
    state::update(&options.root, |f| {
        f.lease = Some(now());
        f.stop = Some(mode);
        Ok(())
    })
}
