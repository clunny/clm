//! `clm build returned`: a run's return, recorded and finished (`return-rules`).

use std::fs;

use clm_git::{Failure, now};

use crate::Options;
use crate::cmd::clm;
use crate::state::{self, Kind, RunState};

/// `clm build returned <name> --result <line> [--cause <cause>]`: records a run's return by
/// the run's last line as it stands and prints the actions to take now.
pub fn returned(
    options: &Options,
    name: &str,
    result: &str,
    cause: Option<&str>,
) -> Result<Vec<String>, Failure> {
    clm_git::fetch(&options.root)?;
    let run = state::update(&options.root, |f| {
        f.lease = Some(now());
        f.run(name)
            .cloned()
            .ok_or_else(|| Failure::Refused(format!("the loop has no run {name}")))
    })?;
    let mut lines = Vec::new();
    let line = result.trim();
    // A run still waiting on a command stays running and frees nothing.
    if line.starts_with("waiting:") {
        lines.extend(crate::step::next_actions(options)?);
        return Ok(lines);
    }
    // A run the loop stopped keeps its claim, branch and place, and nothing is finished.
    if cause == Some("stopped") {
        state::update(&options.root, |f| {
            if let Some(r) = f.run_mut(name) {
                r.state = RunState::Stopped;
            }
            Ok(())
        })?;
        lines.extend(crate::step::next_actions(options)?);
        return Ok(lines);
    }
    if line.starts_with("landed ") && cause == Some("running") {
        let task = run.task_id.as_deref().ok_or_else(|| {
            Failure::Refused(format!("{name} has no task ID: report it with `launched`"))
        })?;
        lines.push(format!("stop {name} {task}"));
    }
    let (finish_args, id) = match run.kind {
        Some(Kind::Build) => {
            let component = name.strip_prefix("build:").unwrap_or(name);
            (vec!["run", "--finish", component], component)
        }
        Some(Kind::Draft) => {
            let id = name.strip_prefix("draft:").unwrap_or(name);
            (vec!["draft", "--finish", id], id)
        }
        _ => (Vec::new(), name),
    };
    let finished = if finish_args.is_empty() {
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
            && finished
                .lines()
                .any(|l| l.trim() == format!("landed impl/{id}")));
    for left in finished
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
            lines.push(say(options, &format!("left {left}"))?);
        }
    }
    let proposed = line
        .strip_prefix("proposed ")
        .and_then(|rest| rest.split_whitespace().next())
        .map(str::to_string);
    state::update(&options.root, |f| {
        if let Some(id) = proposed
            && !f.review.contains(&id)
        {
            f.review.push(id);
        }
        if let Some(r) = f.run_mut(name) {
            r.state = RunState::Returned;
            r.landed = landed;
            if !landed && let Some(cause) = cause {
                *r.stops.entry(cause.to_string()).or_insert(0) += 1;
            }
        }
        Ok(())
    })?;
    lines.extend(crate::step::next_actions(options)?);
    Ok(lines)
}

/// A `say` action naming a file that holds `text`.
fn say(options: &Options, text: &str) -> Result<String, Failure> {
    let root = std::path::absolute(&options.root)
        .map_err(|e| Failure::Git(format!("{}: {e}", options.root.display())))?;
    let dir = root.join(".clm/prompts/loop");
    fs::create_dir_all(&dir).map_err(|e| Failure::Git(format!("{}: {e}", dir.display())))?;
    let path = dir.join(format!("say-{}-{}.md", now(), std::process::id()));
    let path = unique(path);
    fs::write(&path, format!("{text}\n"))
        .map_err(|e| Failure::Git(format!("{}: {e}", path.display())))?;
    Ok(format!("say {}", path.display()))
}

/// `path`, or the first of `path` with `-2`, `-3`, ... before its extension that doesn't exist.
fn unique(path: std::path::PathBuf) -> std::path::PathBuf {
    if !path.exists() {
        return path;
    }
    let stem = path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    (2..)
        .map(|n| path.with_file_name(format!("{stem}-{n}.md")))
        .find(|p| !p.exists())
        .unwrap_or(path)
}
