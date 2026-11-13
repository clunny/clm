//! `clm build start`, `step` and `launched`: reading the snapshot, preparing the runs the plan
//! launches and printing the actions (`step-actions`, `loop-start`).

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use clm_git::{Failure, now};
use clm_orchestrator::DEFAULT_EXPIRY_HOURS;

use crate::Options;
use crate::cmd::{clm, clm_ok, say};
use crate::plan::{Group, Launch, Snapshot, plan_of};
use crate::state::{self, Kind, LoopFile, Run, RunState, StopMode};

/// The prompt file of a run: `.clm/prompts/loop/<name>.md`, with `:` written `-`.
fn prompt_path(options: &Options, name: &str) -> Result<PathBuf, Failure> {
    let root = std::path::absolute(&options.root)
        .map_err(|e| Failure::Git(format!("{}: {e}", options.root.display())))?;
    let dir = root.join(".clm/prompts/loop");
    fs::create_dir_all(&dir).map_err(|e| Failure::Git(format!("{}: {e}", dir.display())))?;
    Ok(dir.join(format!("{}.md", name.replace(':', "-"))))
}

/// Writes a run's prompt file: the line `loop run <name>`, then the launcher as its command
/// printed it.
fn write_prompt(options: &Options, name: &str, launcher: &str) -> Result<String, Failure> {
    let path = prompt_path(options, name)?;
    let text = format!("loop run {name}\n{launcher}");
    fs::write(&path, text).map_err(|e| Failure::Git(format!("{}: {e}", path.display())))?;
    Ok(path.display().to_string())
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
fn read_snapshot(options: &Options, file: LoopFile) -> Result<Snapshot, Failure> {
    let main = clm_git::git(&options.root, &["rev-parse", "origin/main"])?;
    let held = clm_orchestrator::places(&options.root, DEFAULT_EXPIRY_HOURS)
        .map_err(|e| Failure::Git(format!("places: {e:?}")))?
        .iter()
        .filter(|p| p.counts)
        .count() as u64;
    let ready = read_ready(options)?;
    let priorities = if ready.is_empty() {
        BTreeMap::new()
    } else {
        read_priorities(options)?
    };
    let build_model = clm_agent_harness::abilities(options.agent_harness)
        .build_model
        .map_or_else(|| file.session_model.clone(), str::to_string);
    Ok(Snapshot {
        now: now(),
        main,
        free_places: options.max_builds.saturating_sub(held),
        ready,
        priorities,
        groups: read_groups(options)?,
        build_model,
        file,
    })
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
    }
}

/// Runs `clm run` or `clm draft` for a run and writes its prompt file.
fn launch_run(options: &Options, mut run: Run, args: &[&str]) -> Result<Prepared, Failure> {
    let ran = clm(options, args)?;
    match ran.code {
        0 => {
            if run.kind == Some(Kind::Draft) && run.name.is_empty() {
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
            run.prompt = Some(write_prompt(options, &run.name, &ran.stdout)?);
            Ok(Prepared::Run(Box::new(run)))
        }
        3 => Ok(Prepared::NoPlace),
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

fn prepare(options: &Options, file: &LoopFile, launch: &Launch) -> Result<Prepared, Failure> {
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
            )
        }
        Launch::Draft { items, model, .. } => {
            let mut run = planned("", Some(Kind::Draft), model, None);
            run.items = items.clone();
            let mut args = vec!["draft"];
            args.extend(items.iter().map(String::as_str));
            launch_run(options, run, &args)
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
                    )
                }
                (Some(Kind::Draft), _) => {
                    let id = name.strip_prefix("draft:").unwrap_or(name);
                    launch_run(options, run, &["draft", "--resume", id])
                }
                (Some(Kind::Fold | Kind::Archive), _) => Ok(Prepared::Drop(name.clone())),
                (None, _) => Ok(Prepared::Run(Box::new(run))),
            }
        }
    }
}

/// The actions to take now: fetch, do the housekeeping, read the snapshot, compute the plan,
/// prepare each run it launches, record them and print one line per action.
fn actions(options: &Options) -> Result<Vec<String>, Failure> {
    clm_git::fetch(&options.root)?;
    state::update(&options.root, |f| {
        f.lease = Some(now());
        Ok(())
    })?;
    // `clm wrap-up` removes landed worktrees and branches and frees the places that don't
    // count; its report is for the housekeeping requirements, which come later.
    let wrap_up = clm(options, &["wrap-up"])?;
    if wrap_up.code == 2 {
        return Err(Failure::Git(format!("clm wrap-up: {}", wrap_up.reason())));
    }
    let file = state::read(&options.root).unwrap_or_default();
    let snapshot = read_snapshot(options, file)?;
    let plan = plan_of(&snapshot);
    let mut changes = Vec::new();
    let mut stored_file = snapshot.file.clone();
    for launch in &plan.launches {
        match prepare(options, &stored_file, launch)? {
            Prepared::NoPlace => break,
            Prepared::Nothing => {}
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
    let no_launches = plan.launches.is_empty();
    let (lines, ended, file) = state::update(&options.root, |f| {
        for change in changes {
            match change {
                Prepared::Run(run) => match f.run_mut(&run.name) {
                    Some(existing) => *existing = *run,
                    None => f.runs.push(*run),
                },
                Prepared::Drop(name) => f.runs.retain(|r| r.name != name),
                Prepared::Nothing | Prepared::NoPlace => {}
            }
        }
        f.plan = Some(state::StoredPlan {
            main,
            at: now(),
            next,
        });
        let lines = if f.stop == Some(StopMode::Now) {
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
        let busy = f
            .runs
            .iter()
            .any(|r| matches!(r.state, RunState::Planned | RunState::Running));
        let ended = f.stop.is_some() && !busy && no_launches;
        if ended {
            // The loop is over: the next `start` takes the lease at once.
            f.lease = None;
        }
        Ok((lines, ended, f.clone()))
    })?;
    if ended {
        let text = crate::summary::summary(options, &file, &plan.view)?;
        return Ok(vec![say(options, &text)?, "end".to_string()]);
    }
    if lines.is_empty() {
        Ok(vec!["wait".to_string()])
    } else {
        Ok(lines)
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

/// The actions a `returned` prints after it has recorded a run's return.
pub(crate) fn next_actions(options: &Options) -> Result<Vec<String>, Failure> {
    actions(options)
}

/// `clm build step`: the actions to take now.
pub fn step(options: &Options) -> Result<Vec<String>, Failure> {
    actions(options)
}

/// `clm build start --session-model <alias> [--take-over]`: begins or adopts the loop.
pub fn start(
    options: &Options,
    session_model: &str,
    take_over: bool,
) -> Result<Vec<String>, Failure> {
    clm_git::fetch(&options.root)?;
    let main = clm_git::git(&options.root, &["rev-parse", "origin/main"])?;
    let at = now();
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
            }
        }
        Ok(())
    })?;
    actions(options)
}

/// `clm build launched <name> <task-id>`: records the task ID of a run the session launched.
pub fn launched(options: &Options, name: &str, task_id: &str) -> Result<(), Failure> {
    state::update(&options.root, |f| {
        f.lease = Some(now());
        let run = f
            .run_mut(name)
            .ok_or_else(|| Failure::Refused(format!("the loop has no run {name}")))?;
        run.state = RunState::Running;
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
