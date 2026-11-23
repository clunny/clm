//! `clm build returned`: a run's return, recorded and finished (`return-rules`), and what a
//! stop comes to (`stop-causes`, `repeated-stops-escalate`).

use clm_git::{Failure, now};

use crate::Options;
use crate::cmd::{clm, say};
use crate::escalate;
use crate::state::{self, Batch, Escalation, Kind, LoopFile, Run, RunState};
use crate::stops::{self, CAUSES, Verdict};

/// What finishing a run came to.
struct Finished {
    landed: bool,
    /// The `say` actions for what the finish left behind.
    says: Vec<String>,
}

/// Finishes a build or draft with `clm run --finish` or `clm draft --finish`, and reads from it
/// whether the run landed: a closing line decides, and without one the finish does
/// (`unclosed-build-reads-finish`). Each `left` line it prints is said, or, for a draft, recorded
/// among the items drafts of the loop left. `returned` says the run's subagent has handed back,
/// so a build's finish passes `--returned` (`return-rules`).
fn finish(options: &Options, run: &Run, line: &str, returned: bool) -> Result<Finished, Failure> {
    let name = &run.name;
    let (finish_args, id) = match run.kind {
        Some(Kind::Build) => {
            let component = name.strip_prefix("build:").unwrap_or(name);
            let mut args = vec!["run", "--finish", component];
            if returned {
                args.push("--returned");
            }
            (args, component)
        }
        Some(Kind::Draft) => {
            let id = name.strip_prefix("draft:").unwrap_or(name);
            (vec!["draft", "--finish", id], id)
        }
        _ => (Vec::new(), name.as_str()),
    };
    let output = if finish_args.is_empty() {
        String::new()
    } else {
        let ran = clm(options, &finish_args)?;
        if ran.code == 2 {
            return Err(Failure::Git(format!(
                "clm {}: {}",
                finish_args.join(" "),
                ran.reason()
            )));
        }
        ran.stdout
    };
    let closing = line.starts_with("landed ")
        || line.starts_with("proposed ")
        || line.starts_with("stopped:");
    let landed = line.starts_with("landed ")
        || (!closing
            && run.kind == Some(Kind::Build)
            && output
                .lines()
                .any(|l| l.trim() == format!("landed impl/{id}")));
    let mut says = Vec::new();
    for left in output
        .lines()
        .filter_map(|l| l.trim().strip_prefix("left "))
    {
        if run.kind == Some(Kind::Draft) {
            if let Some((slug, _)) = left.split_once(':') {
                let slug = slug.to_string();
                state::update(&options.root, |f| {
                    if !f.drafted.contains(&slug) {
                        f.drafted.push(slug);
                    }
                    Ok(())
                })?;
            }
        } else {
            says.push(say(options, &format!("left {left}"))?);
        }
    }
    // What a build's `--returned` finish or a draft's finish keeps stays for the human; what a
    // build's finish without it keeps, the return's finish removes (`return-rules`).
    if run.kind == Some(Kind::Draft) || (run.kind == Some(Kind::Build) && returned) {
        says.extend(kept(options, &output)?);
    }
    Ok(Finished { landed, says })
}

/// The `say` actions for each line starting `kept ` that a finish printed (`return-rules`).
pub(crate) fn kept(options: &Options, output: &str) -> Result<Vec<String>, Failure> {
    output
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("kept "))
        .map(|l| say(options, l))
        .collect()
}

/// Finishes a build or draft as a run that returned with no closing line and no cause, and
/// records it `returned`, with no stop counted: whether it landed, and the `say` actions its
/// finish leaves (`return-rules`). No subagent has handed back, so a build's finish passes no
/// `--returned` (`claim-gone-ends-a-build`, `loop-start`).
pub(crate) fn finish_unclosed(
    options: &Options,
    name: &str,
) -> Result<(bool, Vec<String>), Failure> {
    let Some(run) = state::read(&options.root).and_then(|f| f.run(name).cloned()) else {
        return Ok((false, Vec::new()));
    };
    let finished = finish(options, &run, "", false)?;
    state::update(&options.root, |f| {
        if finished.landed {
            f.landed_since_verify = true;
        }
        if let Some(r) = f.run_mut(name) {
            r.state = RunState::Returned;
            r.landed = finished.landed;
            r.orphan = false;
        }
        Ok(())
    })?;
    Ok((finished.landed, finished.says))
}

/// Whether `line` is a `landed <sha>` report of an earlier run of a build than `run`: its commit
/// is one the clone holds, committed before `run` started, and no run lands a commit made before
/// it started (`return-rules`, `claim-gone-ends-a-build`).
fn late_report(options: &Options, run: &Run, line: &str) -> bool {
    if run.kind != Some(Kind::Build) {
        return false;
    }
    let Some(sha) = line
        .strip_prefix("landed ")
        .and_then(|rest| rest.split_whitespace().next())
    else {
        return false;
    };
    if !sha.bytes().all(|b| b.is_ascii_hexdigit()) {
        return false;
    }
    clm_git::git(
        &options.root,
        &["show", "-s", "--format=%ct", &format!("{sha}^{{commit}}")],
    )
    .ok()
    .and_then(|t| t.trim().parse::<u64>().ok())
    .is_some_and(|committed| committed < run.started)
}

/// Whether a build's branch has landed, as the place judges it: at least one of
/// `origin/impl/<component>` and the local branch exists, and each that exists is on
/// `origin/main`. It reads git alone, so a run that is to keep its place can be asked.
fn branch_landed(options: &Options, run: &Run) -> Result<bool, Failure> {
    let Some(component) = run.name.strip_prefix("build:") else {
        return Ok(false);
    };
    let mut any = false;
    for rf in [
        format!("refs/remotes/origin/impl/{component}"),
        format!("refs/heads/impl/{component}"),
    ] {
        if clm_git::git(&options.root, &["rev-parse", "--verify", "-q", &rf]).is_err() {
            continue;
        }
        if !clm_git::is_ancestor(&options.root, &rf, "refs/remotes/origin/main")? {
            return Ok(false);
        }
        any = true;
    }
    Ok(any)
}

/// A file holding a resume's message, which the `resume` action names.
pub(crate) fn message_file(
    options: &Options,
    name: &str,
    text: &str,
) -> Result<std::path::PathBuf, Failure> {
    let root = std::path::absolute(&options.root)
        .map_err(|e| Failure::Git(format!("{}: {e}", options.root.display())))?;
    let dir = root.join(".clm/prompts/loop");
    std::fs::create_dir_all(&dir).map_err(|e| Failure::Git(format!("{}: {e}", dir.display())))?;
    let stem = format!("resume-{}", name.replace(':', "-"));
    let path = (1..)
        .map(|n| dir.join(format!("{stem}-{n}.md")))
        .find(|p| !p.exists())
        .unwrap_or_else(|| dir.join(format!("{stem}.md")));
    std::fs::write(&path, text).map_err(|e| Failure::Git(format!("{}: {e}", path.display())))?;
    Ok(path)
}

/// `clm build returned <name> [--task <task-id>] --result <line> [--cause <cause> [--until
/// <time>]]`: records a run's return by the run's last line as it stands, or, for `verify`, by
/// its outcome line (the last line starting `verify `, or `none`), and prints the actions to take
/// now. `task` names the task the return came from.
pub fn returned(
    options: &Options,
    name: &str,
    task: Option<&str>,
    result: &str,
    cause: Option<&str>,
    until: Option<&str>,
) -> Result<Vec<String>, Failure> {
    if let Some(cause) = cause
        && !CAUSES.contains(&cause)
    {
        return Err(Failure::Refused(format!(
            "unknown cause `{cause}`: one of {}",
            CAUSES.join(", ")
        )));
    }
    let until =
        match until {
            None => None,
            Some(_) if cause != Some("limit") => {
                return Err(Failure::Refused(
                    "--until goes with --cause limit".to_string(),
                ));
            }
            Some(text) => Some(crate::time::unix(text).ok_or_else(|| {
                Failure::Refused(format!("--until `{text}` isn't an RFC 3339 time"))
            })?),
        };
    let notes = &state::notes(&options.root)?;
    clm_git::fetch(&options.root)?;
    // The late report of an earlier run of that name changes nothing but the lease: a return
    // from a task the loop keeps under the name, whether or not a run of it is listed
    // (`returned-ignores-an-earlier-task`), or a `landed` line older than the run (`return-rules`).
    let (known, earlier) = state::update(&options.root, |f| {
        f.lease = Some(now());
        let earlier = task.filter(|t| f.kept_earlier(name, t));
        Ok((f.run(name).cloned(), earlier))
    })?;
    if let Some(task) = earlier {
        eprintln!(
            "clm build: the return from task {task} is a late report of an earlier run of \
             {name}, which the loop has finished; it changed nothing"
        );
        return crate::step::next_actions(options, notes);
    }
    // `clm verify` prints notes on stderr after its outcome line, so a session reading both
    // streams as one may report a note: refused before every other rule (`plan-verify`).
    if name == "verify" {
        if known.is_none() {
            return Err(Failure::Refused(format!("the loop has no run {name}")));
        }
        let line = result.trim();
        if line.contains('\n') || (line != "none" && !line.starts_with("verify ")) {
            return Err(Failure::Refused(format!(
                "`{line}` isn't verify's outcome: report the line of the run's output that \
                 starts `verify `, or `none` when no line does"
            )));
        }
    }
    if let Some(known) = &known
        && late_report(options, known, result.trim())
    {
        return crate::step::next_actions(options, notes);
    }
    // A build the loop already finished, as `claim-gone-ends-a-build` may before its subagent
    // hands back: its return finishes it again with `--returned`, which removes the worktree the
    // first finish kept, and changes nothing else (`return-rules`).
    if let Some(known) = known
        .as_ref()
        .filter(|r| r.kind == Some(Kind::Build) && r.state == RunState::Returned)
    {
        let mut lines = Vec::new();
        if !result.trim().starts_with("waiting:") {
            lines.extend(finish(options, known, "", true)?.says);
        }
        lines.extend(crate::step::next_actions(options, notes)?);
        return Ok(lines);
    }
    let no_run = || Failure::Refused(format!("the loop has no run {name}"));
    let listed = known.ok_or_else(no_run)?;
    // A report repeated, such as after a timeout, of a run other than a build (whose returns
    // are handled above) finishes nothing twice and changes nothing but the lease.
    if listed.state == RunState::Returned {
        return crate::step::next_actions(options, notes);
    }
    // The `stop` a background run's `landed` return owes names its task: with none reported,
    // nothing changes (`return-rules`).
    let command = name.starts_with("land:") || name == "verify";
    if !command
        && cause == Some("running")
        && result.trim().starts_with("landed ")
        && listed.task_id.is_none()
    {
        return Err(no_task(name));
    }
    let (run, outstanding) = state::update(&options.root, |f| {
        f.lease = Some(now());
        let run = f.run_mut(name).ok_or_else(no_run)?;
        let outstanding = std::mem::take(&mut run.resumed);
        Ok((run.clone(), outstanding))
    })?;
    if name.starts_with("land:") {
        return land_returned(options, &run, result.trim(), notes);
    }
    if name == "verify" {
        return verify_returned(options, &run, result.trim(), notes);
    }
    let line = result.trim();
    // A run still waiting on a command stays running and frees nothing.
    if line.starts_with("waiting:") {
        return crate::step::next_actions(options, notes);
    }
    // A run the loop stopped keeps its claim, branch and place, and nothing is finished.
    if cause == Some("stopped") {
        state::update(&options.root, |f| {
            if let Some(r) = f.run_mut(name) {
                r.state = RunState::Stopped;
                *r.stops.entry("stopped".to_string()).or_insert(0) += 1;
            }
            Ok(())
        })?;
        return crate::step::next_actions(options, notes);
    }
    let landed_line = line.starts_with("landed ");
    // A task the agent harness still lists as running background work is left to run.
    if cause == Some("running") && !landed_line {
        return crate::step::next_actions(options, notes);
    }
    let mut lines = Vec::new();
    if landed_line && cause == Some("running") {
        let task = run.task_id.as_deref().ok_or_else(|| no_task(name))?;
        lines.push(format!("stop {name} {task}"));
    }
    let proposed = line
        .strip_prefix("proposed ")
        .and_then(|rest| rest.split_whitespace().next())
        .map(str::to_string);
    // What the stop comes to, when the run ended without a closing line of its own.
    let stop = match cause {
        Some(c) if !landed_line && proposed.is_none() => {
            let mut trial = run.clone();
            *trial.stops.entry(c.to_string()).or_insert(0) += 1;
            Some((c, stops::verdict(&trial, c, until, now(), outstanding)))
        }
        _ => None,
    };
    // A pause finishes nothing, so the run keeps its claim, branch and place.
    if let Some((c, Verdict::Pause(pending))) = &stop
        && !branch_landed(options, &run)?
    {
        let (c, pending) = (c.to_string(), pending.clone());
        let entry = format!("{c}: {line}");
        state::update(&options.root, |f| {
            let others = f
                .runs
                .iter()
                .any(|r| r.name != name && r.resume.as_ref().is_some_and(|p| p.cause == c));
            if matches!(c.as_str(), "limit" | "disk") && others {
                f.conditions.serial = true;
            }
            match c.as_str() {
                "disk" => f.conditions.disk = true,
                "limit" => {
                    f.conditions.limit_until = f.conditions.limit_until.max(pending.after);
                }
                _ => {}
            }
            if let Some(r) = f.run_mut(name) {
                r.state = RunState::Stopped;
                *r.stops.entry(c.clone()).or_insert(0) += 1;
                r.log.push(entry);
                r.resume = Some(pending);
            }
            Ok(())
        })?;
        lines.extend(crate::step::next_actions(options, notes)?);
        return Ok(lines);
    }
    let finished = finish(options, &run, line, true)?;
    lines.extend(finished.says);
    let landed = finished.landed;
    let (stopped_for, verdict) = match (&stop, landed) {
        (Some((c, v)), false) => (Some(c.to_string()), Some(v.clone())),
        _ => (None, None),
    };
    // The item an escalation files, built before the loop file changes.
    let escalation = match (&verdict, &stopped_for) {
        (Some(Verdict::Escalate), Some(c))
            if matches!(run.kind, Some(Kind::Build | Kind::Draft)) =>
        {
            let mut counted = run.clone();
            counted.log.push(format!("{c}: {line}"));
            Some(escalate::item(options, &counted, c)?)
        }
        _ => None,
    };
    let filed = escalation
        .as_ref()
        .map(|(slug, text)| (slug.clone(), escalate::land(options, slug, text).is_ok()));
    let say_fold = (run.kind == Some(Kind::Fold) && proposed.is_none() && !landed)
        .then(|| {
            let id = name.strip_prefix("fold:").unwrap_or(name);
            format!(
                "fold {id} returned without a batch ({line}); its drafts stay on their spec branches, which `clm proposals` lists and /clm:triage folds"
            )
        });
    if let Some(text) = &say_fold {
        lines.push(say(options, text)?);
    }
    state::update(&options.root, |f| {
        let session_model = f.session_model.clone();
        if let Some(id) = &proposed {
            match run.kind {
                Some(Kind::Fold) => f.batches.push(Batch {
                    id: id.clone(),
                    approved: false,
                    drafts: run.items.clone(),
                    since: now(),
                }),
                _ if !f.review.contains(id) => f.review.push(id.clone()),
                _ => {}
            }
        }
        if landed {
            f.landed_since_verify = true;
        }
        if let Some((slug, landed_it)) = &filed {
            if !landed_it && let Some((_, text)) = &escalation {
                f.escalations.push(Escalation {
                    slug: slug.clone(),
                    body: text.clone(),
                    since: now(),
                });
            }
            if run.kind == Some(Kind::Draft) {
                for item in &run.items {
                    if !f.drafted.contains(item) {
                        f.drafted.push(item.clone());
                    }
                }
            }
        }
        if let Some(r) = f.run_mut(name) {
            r.state = RunState::Returned;
            r.landed = landed;
            if let (Some(c), Some(verdict)) = (&stopped_for, &verdict) {
                *r.stops.entry(c.clone()).or_insert(0) += 1;
                r.log.push(format!("{c}: {line}"));
                if *verdict == Verdict::Rebuild {
                    // A new subagent, on the session's model, in the next round.
                    r.state = RunState::Stopped;
                    r.orphan = true;
                    r.round += 1;
                    r.model = session_model;
                    r.rebuilt_for = Some(c.clone());
                    r.task_id = None;
                    r.resume = None;
                }
            }
            if let Some((slug, _)) = &filed {
                r.escalated = Some(slug.clone());
            }
        }
        Ok(())
    })?;
    lines.extend(crate::step::next_actions(options, notes)?);
    Ok(lines)
}

/// The refusal of a `landed` return with `--cause running` from a run with no task ID, whose
/// `stop` would name none (`return-rules`).
fn no_task(name: &str) -> Failure {
    Failure::Refused(format!(
        "{name} has no task ID to stop: report it with `clm build launched {name} <task-id>`, \
         then report its return again"
    ))
}

/// The return of `land:<batch>`, a `clm land spec/<batch>` the session ran (`plan-fold`).
fn land_returned(
    options: &Options,
    run: &Run,
    line: &str,
    notes: &str,
) -> Result<Vec<String>, Failure> {
    let batch = run
        .name
        .strip_prefix("land:")
        .unwrap_or(&run.name)
        .to_string();
    let landed = line.starts_with("landed ");
    let drafts = state::read(&options.root)
        .and_then(|f| f.batches.iter().find(|b| b.id == batch).cloned())
        .map(|b| b.drafts)
        .unwrap_or_default();
    let mut lines = Vec::new();
    if landed {
        for id in &drafts {
            let ran = clm(options, &["draft", "--finish", id])?;
            if ran.code == 2 {
                return Err(Failure::Git(format!(
                    "clm draft --finish {id}: {}",
                    ran.reason()
                )));
            }
            for left in ran
                .stdout
                .lines()
                .filter_map(|l| l.trim().strip_prefix("left "))
            {
                lines.push(say(options, &format!("left {left}"))?);
            }
            lines.extend(kept(options, &ran.stdout)?);
        }
    } else if !line.contains("gate/approval") {
        lines.push(say(
            options,
            &format!("the land of fold batch {batch} failed ({line}); it is back in your queue"),
        )?);
    }
    state::update(&options.root, |f| {
        if landed {
            f.batches.retain(|b| b.id != batch);
            f.landed_since_verify = true;
        } else if let Some(b) = f.batches.iter_mut().find(|b| b.id == batch) {
            // Back in the human's queue.
            b.approved = false;
            b.since = now();
        }
        if let Some(r) = f.run_mut(&run.name) {
            r.state = RunState::Returned;
            r.landed = landed;
        }
        Ok(())
    })?;
    lines.extend(crate::step::next_actions(options, notes)?);
    Ok(lines)
}

/// The return of `verify`, a `clm verify --due` the session ran (`plan-verify`).
fn verify_returned(
    options: &Options,
    run: &Run,
    line: &str,
    notes: &str,
) -> Result<Vec<String>, Failure> {
    state::update(&options.root, |f: &mut LoopFile| {
        if line.starts_with("verify failed") && !f.red_verifies.iter().any(|r| r == line) {
            f.red_verifies.push(line.to_string());
        }
        f.landed_since_verify = false;
        if let Some(r) = f.run_mut(&run.name) {
            r.state = RunState::Returned;
        }
        Ok(())
    })?;
    crate::step::next_actions(options, notes)
}
