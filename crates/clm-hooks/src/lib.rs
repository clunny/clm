//! The rules behind the plugin's hooks (`hook-logic-in-clm`). Each takes the hook's JSON
//! event as its agent harness writes it to stdin, and treats input it can't read as nothing
//! to do.

use std::io::Write;
use std::path::Path;

use clm_agent_harness::{AgentHarness, parse_event};
use clm_view_repo::NodeSize;

mod command;
mod defect;
mod upgrade;
pub use command::{DefectArgs, HookArgs, autosync_command, defect_command, session_start_command};

/// The environment as a lookup, as each command's call takes it.
type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// `SessionStart` for an agent harness (`session-status`, `hooks-per-agent-harness`): the
/// event's `cwd` comes from the agent harness's own parser, and the text is written the way
/// that agent harness adds it to the session's context. The settings are read through `env`,
/// the status is `clm status`'s given `budgets`, and what that writes to standard error goes
/// to `err`. The exit code is 0.
fn session_start_in(
    agent_harness: AgentHarness,
    event: &str,
    env: Env,
    err: &mut dyn Write,
    budgets: NodeSize,
) -> (String, i32) {
    let text = hook_cwd(agent_harness, event)
        .map(|cwd| session_text(Path::new(&cwd), env, err, budgets))
        .unwrap_or_default();
    if text.is_empty() || agent_harness != AgentHarness::Codex {
        return (text, 0);
    }
    // Codex reads a session start's `additionalContext` as context for the session.
    let out = serde_json::json!({"hookSpecificOutput": {
        "hookEventName": "SessionStart",
        "additionalContext": text,
    }});
    (format!("{out}\n"), 0)
}

/// `Stop` for an agent harness (`autosync-hook`, `hooks-per-agent-harness`), with the settings
/// and the environment git is given read through `env`: silent, exit code 0.
fn autosync_hook_in(agent_harness: AgentHarness, event: &str, env: Env) -> (String, i32) {
    if let Some(cwd) = hook_cwd(agent_harness, event) {
        autosync_in(Path::new(&cwd), env);
    }
    (String::new(), 0)
}

fn hook_cwd(agent_harness: AgentHarness, event: &str) -> Option<String> {
    parse_event(agent_harness, event).ok()?.cwd
}

/// `clm status` for the CLM repository holding `cwd`, then the upgrade line and the rule for
/// reporting CLM defects unless `defects` is `off` (or the settings' error), else "".
fn session_text(cwd: &Path, env: Env, err: &mut dyn Write, budgets: NodeSize) -> String {
    let Some(root) = clm_root(cwd) else {
        return String::new();
    };
    let settings = clm_config::load(Some(root), env);
    let skipped = settings
        .as_ref()
        .ok()
        .and_then(|s| s.skip_migrations.value.clone())
        .unwrap_or_default();
    let (list, pairs) = (clm_init::migrations(), clm_init::supersedes());
    let reminder = upgrade::upgrade_line(clm_upgrade::pending_in(&list, &pairs, root), &skipped);
    // `clm status` as it prints, kept only when it succeeds.
    let mut printed = Vec::new();
    let code = clm_view_repo::status_command(
        std::iter::empty::<&str>(),
        root,
        env,
        &mut printed,
        err,
        budgets,
    );
    let status = match (code, String::from_utf8(printed)) {
        (0, Ok(status)) => status,
        // The reminder stands even when the status can't be read.
        _ => return reminder.map(|line| line + "\n").unwrap_or_default(),
    };
    let mut text = format!("CLM state (`clm status`) of this repository:\n{status}");
    if let Some(line) = reminder {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&line);
        text.push('\n');
    }
    match settings {
        Ok(s) if s.defects.value == clm_config::Defects::Off => {}
        Ok(s) => {
            text.push_str(if text.ends_with('\n') { "\n" } else { "\n\n" });
            text.push_str(DEFECT_RULE);
            text.push_str(if s.defects.value == clm_config::Defects::Backlog {
                BACKLOG_RULE
            } else {
                ASK_RULE
            });
            text.push('\n');
        }
        Err(e) => {
            if !text.ends_with('\n') {
                text.push('\n');
            }
            text.push_str(&format!("clm: settings: {e}\n"));
        }
    }
    text
}

/// What `session-start` tells every session about CLM defects (`defect-reports`).
const DEFECT_RULE: &str = "CLM defects: when CLM itself gets in the way, report it with \
`clm defect --title <title> [--component <component>]...`, the report on stdin, when it happens. A CLM \
defect is anything CLM itself (its CLI, CI, gate rules, plugin, skills or method) would have \
to change so it doesn't happen again: a gate or check that misfires, a rule you must work \
around, a wrong or missing instruction, a decision that fails in practice. A mistake of your \
own that CLM caught, or that you fixed, is not one. A report describes only the process: the \
CLM step or command, what happened, what was expected, and the rule or decision involved. It \
never includes the project's files, code, specs or output. A session that can't ask the human \
(an unattended one) lists its defects under `## CLM defects` in its PR body instead of filing \
issues.";

/// What `session-start` adds when `defects` is `backlog`: where reports go, and fix-first
/// (`defect-reports`).
const BACKLOG_RULE: &str = " In this repository `clm defect` writes each report to \
`backlog/` as a `kind: bug` item. Fix a small defect that needs no spec change in the moment \
rather than leaving it filed: on an `impl/*` branch whose components hold the fix, fold it into \
that branch and don't report it (`fold-small-defer-large`); otherwise report it with \
`clm defect`, naming with `--component` the component the fix lies in, and once the current branch has \
landed and brought the item to `main`, fix it before the session ends on its own branch with \
`clm claim <component> --fix backlog/<slug>` (`code-fix-resolves-bug`). Leave in the backlog only \
a defect that is a large amount of work or needs a spec change. An unattended session folds \
what fits and reports the rest.";

/// What `session-start` adds when `defects` is `ask`: the one form of `clm defect` the guard
/// shows the human before the report leaves the project (`defect-ask-shows-report`).
const ASK_RULE: &str = " Send the report as the body of a `<<'EOF'` heredoc on the \
`clm defect` line, the one form the guard shows the human before the report leaves the project. When `clm defect` keeps a report it couldn't \
file (`defect-issue`), list that report under `## CLM defects` in your PR body with the kept \
file's path, for the human to file by hand.";

/// Pushes a snapshot of the spec work in the CLM repository holding `cwd` (`autosync-policy`)
/// with `sync`'s call of `clm autosync`, given `env`, unless the repository's `autosync`
/// setting is `false` or can't be loaded. Silent whatever happens: it runs every turn.
fn autosync_in(cwd: &Path, env: Env) {
    let Some(root) = clm_root(cwd) else {
        return;
    };
    if clm_config::load(Some(root), env).is_ok_and(|s| s.autosync.value) {
        let (mut out, mut err) = (std::io::sink(), std::io::sink());
        let _ =
            clm_sync::autosync_command(std::iter::empty::<&str>(), root, env, &mut out, &mut err);
    }
}

/// The first of `cwd` and its ancestors with `specs/SPEC.md`, looking no higher than the
/// first that holds `.git` (a directory, or a worktree's file).
pub fn clm_root(cwd: &Path) -> Option<&Path> {
    for dir in cwd.ancestors() {
        if dir.join("specs/SPEC.md").is_file() {
            return Some(dir);
        }
        if dir.join(".git").exists() {
            return None;
        }
    }
    None
}
