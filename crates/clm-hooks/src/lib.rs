//! The rules behind the plugin's hooks (`hook-logic-in-clm`). Each takes the hook's JSON
//! event as Claude Code writes it to stdin, and treats input it can't read as nothing to do.

use std::path::Path;

use clm_agent_harness::{AgentHarness, parse_event};
use serde_json::Value;

mod defect;
mod upgrade;
pub use defect::defect;

/// `SessionStart`: `clm status` for the CLM repository holding the event's `cwd`, then the
/// rule for reporting CLM defects unless `defects` is `off` (or the settings' error), else "".
pub fn session_start(event: &str) -> String {
    match serde_json::from_str::<Value>(event) {
        Ok(event) => event["cwd"]
            .as_str()
            .map(|cwd| session_text(Path::new(cwd)))
            .unwrap_or_default(),
        Err(_) => String::new(),
    }
}

/// `session_start` for an agent harness (`hooks-per-agent-harness`): the event's `cwd` comes
/// from the agent harness's own parser, and the text is written the way that agent harness
/// adds it to the session's context. The exit code is 0.
pub fn session_start_hook(agent_harness: AgentHarness, event: &str) -> (String, i32) {
    let text = hook_cwd(agent_harness, event)
        .map(|cwd| session_text(Path::new(&cwd)))
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

/// `autosync` for an agent harness (`hooks-per-agent-harness`): silent, exit code 0.
pub fn autosync_hook(agent_harness: AgentHarness, event: &str) -> (String, i32) {
    if let Some(cwd) = hook_cwd(agent_harness, event) {
        autosync_in(Path::new(&cwd));
    }
    (String::new(), 0)
}

fn hook_cwd(agent_harness: AgentHarness, event: &str) -> Option<String> {
    parse_event(agent_harness, event).ok()?.cwd
}

fn session_text(cwd: &Path) -> String {
    let Some(root) = clm_root(cwd) else {
        return String::new();
    };
    let settings = settings(Some(root));
    // Oversized nodes marked as `clm status` marks them; without settings, none.
    let budgets = settings.as_ref().ok().map(|s| clm_view_repo::Budgets {
        spec_tokens: s.node_size.value.spec_tokens,
        code_tokens: s.node_size.value.code_tokens,
    });
    let skipped = settings
        .as_ref()
        .ok()
        .and_then(|s| s.skip_migrations.value.clone())
        .unwrap_or_default();
    let reminder = upgrade::upgrade_line(clm_upgrade::pending(root), &skipped);
    let Ok(status) = clm_view_repo::status(root, budgets) else {
        // The reminder stands even when the status can't be read.
        return reminder.map(|line| line + "\n").unwrap_or_default();
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

/// `Stop`: pushes a snapshot of the spec work in the CLM repository holding the event's
/// `cwd` (`autosync-policy`), unless its `autosync` setting is `false` or can't be loaded.
/// Silent whatever happens: it runs every turn.
pub fn autosync(event: &str) {
    if let Some(cwd) = serde_json::from_str::<Value>(event)
        .ok()
        .and_then(|e| e["cwd"].as_str().map(str::to_string))
    {
        autosync_in(Path::new(&cwd));
    }
}

fn autosync_in(cwd: &Path) {
    let Some(root) = clm_root(cwd) else {
        return;
    };
    if settings(Some(root)).is_ok_and(|s| s.autosync.value) {
        let _ = clm_sync::autosync(root);
    }
}

/// The settings for the CLM repository found from `cwd` (or none), read from this
/// process's environment. The hooks and `clm defect` all load them this way, so they agree
/// on the repository.
pub fn settings(cwd: Option<&Path>) -> Result<clm_config::Settings, clm_config::Error> {
    clm_config::load(cwd.and_then(clm_root), &clm_config::process_env)
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
