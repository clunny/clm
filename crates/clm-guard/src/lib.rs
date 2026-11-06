//! `clm hook guard` (`guard-matches-writes`, `guard-reads-quotes-and-wrappers`): judges
//! each segment of a `Bash` command, as the shell would split and run it, by every rule.

mod gh;
mod push;
mod release;
mod shell;

use std::path::Path;

use serde_json::Value;

use clm_hooks::settings;
use push::Reach;
use shell::command;
use shell::walk::{Judged, Stdin, walk};

/// What the guard answers a `PreToolUse` event with.
#[derive(Debug, PartialEq, Eq)]
pub enum Decision {
    /// Let the command run: exit 0, print nothing.
    Allow,
    /// Have the human confirm the command (`approve-in-session`), with the reason.
    Ask(String),
    /// Refuse the command, with the reason: exit 2.
    Block(String),
}

/// The `ask` permission decision Claude Code reads from a `PreToolUse` hook's stdout.
pub fn ask_json(reason: &str) -> String {
    serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "ask",
            "permissionDecisionReason": reason,
        }
    })
    .to_string()
}

/// `PreToolUse`: refuses a `Bash` command that would cut a release or move `main`, and
/// has the human confirm one that runs `clm approve` (`approve-in-session`), or files a CLM
/// defect as an issue. A block wins over an ask.
pub fn guard(event: &str) -> Decision {
    let Ok(event) = serde_json::from_str::<Value>(event) else {
        return Decision::Allow;
    };
    if event["tool_name"] != "Bash" {
        return Decision::Allow;
    }
    let Some(command) = event["tool_input"]["command"].as_str() else {
        return Decision::Allow;
    };
    let cwd = event["cwd"].as_str().map(Path::new);
    let judged = walk(command, cwd);
    let mut reasons: Vec<String> = Vec::new();
    // Re-running a release run would publish, merge or deploy again, and one that can't be
    // looked up might be one.
    let reruns: Vec<_> = judged
        .iter()
        .filter_map(|j| release::rerun(&j.tokens, &j.command(), j.checkout.as_deref()))
        .collect();
    if judged.iter().any(|j| release::cuts_release(&j.tokens)) || release::reruns_release(&reruns) {
        reasons.push(
            "clm: only a human cuts a release (`humans-cut-releases`). Ask the user to \
             dispatch the Release or Site workflow."
                .to_string(),
        );
    }
    let budget = push::Budget::default();
    let mut reaches: Vec<(Reach, Option<&str>)> = judged
        .iter()
        .filter_map(|j| {
            push::reach(
                &j.command(),
                j.checkout.as_deref(),
                j.lost.as_deref(),
                &budget,
            )
        })
        .collect();
    if judged.iter().any(|j| push::host_moves_main(&j.tokens)) {
        reaches.push((Reach::Explicit, None));
    }
    if !reaches.is_empty() {
        reasons.push(
            "clm: nothing moves `main` but `clm land` (`guard-blocks-push-to-main`). Push your \
             branch and land it with `clm land`."
                .to_string(),
        );
    }
    for why in reaches.iter().filter_map(|(r, lost)| r.why(*lost)) {
        if !reasons.contains(&why) {
            reasons.push(why);
        }
    }
    // A pipe hides a land's verdict (`guard-refuses-piped-land`).
    for j in judged.iter().filter(|j| j.piped && runs_land(&j.tokens)) {
        reasons.push(format!(
            "clm: `{plain}` piped to another command (`guard-refuses-piped-land`). A pipe \
             reports the last command's result, not the land's, and cuts its output, while the \
             land's last line is its verdict and Claude Code keeps a long output whole. Run \
             `{plain}` on its own.",
            plain = j.tokens.join(" ")
        ));
    }
    // `clm defect` reads `CLM_DEFECTS` from its own environment, which the segment can set.
    // Loaded once, and only when a segment runs `clm defect` without its own value.
    let asks_by_settings = std::cell::OnceCell::new();
    let files_issue = |j: &&Judged| {
        runs_defect(&j.tokens)
            && match j.command().assigned("CLM_DEFECTS") {
                Some(v) => !matches!(v, "backlog" | "off"),
                None => *asks_by_settings.get_or_init(|| {
                    settings(cwd).map_or(true, |s| s.defects.value == clm_config::Defects::Ask)
                }),
            }
    };
    // Each such segment's report is shown to the human when the guard can
    // (`defect-ask-shows-report`): a heredoc the shell sends as written.
    let filings: Vec<&Judged> = judged.iter().filter(|j| files_issue(j)).collect();
    if filings.iter().any(|j| shown_report(j).is_none()) {
        reasons.push(
            "clm: the guard can't show the human this `clm defect` report \
             (`defect-ask-shows-report`), and the report leaves the project as a public issue. \
             Send the report as the body of a `<<'EOF'` heredoc on the `clm defect` line, so \
             the human can read it before confirming."
                .to_string(),
        );
    }
    if !reasons.is_empty() {
        return Decision::Block(reasons.join("\n"));
    }
    let mut asks: Vec<String> = judged
        .iter()
        .filter(|j| approves(&j.tokens))
        .map(|j| {
            format!(
                "clm: `{}` records a human's review decision (a change's intent, or a \
                 capability's or architectural constraint's worth), and only a human makes it \
                 (`humans-review-intent`). \
                 Confirm only if you, the human, have just said yes to it.",
                j.text
            )
        })
        .collect();
    if !filings.is_empty() {
        let quoted: Vec<String> = filings
            .iter()
            .filter_map(|j| shown_report(j).map(|r| format!("{}\n{r}", j.text)))
            .collect();
        asks.push(format!(
            "clm: `clm defect` files its report as a public issue on `clunny/clm`, so the \
             report leaves this project (`defect-reports`). Confirm only if you, the human, have read \
             it and it holds no project content: no files, code, specs or output. When no one \
             can approve (an unattended session), list the defect under `## CLM defects` in \
             the pull request body instead. The command and its report:\n{}",
            quoted.join("\n\n")
        ));
    }
    if asks.is_empty() {
        Decision::Allow
    } else {
        Decision::Ask(asks.join("\n"))
    }
}

/// A segment's `clm` call (`approve-guard`), as every rule that looks for one reads it.
enum ClmCall<'a> {
    /// Its command is `clm`, or ends in `/clm`: `clm`'s arguments.
    Clm(&'a [String]),
    /// It is a `cargo run`: the tokens after the subcommand. The guard doesn't work out which
    /// package or binary runs, or which of these reach it (`guard-asks-any-cargo-run`).
    CargoRun(&'a [String]),
}

/// The `clm` call a segment with `tokens` makes, if any.
fn clm_call(tokens: &[String]) -> Option<ClmCall<'_>> {
    let command = command::parse(tokens);
    if command.is("clm") {
        return Some(ClmCall::Clm(command.args));
    }
    if !command.is("cargo") {
        return None;
    }
    let args = command.args;
    // The subcommand: the first word that isn't an option or a global option's value.
    let mut i = 0;
    while let Some(t) = args.get(i) {
        match t.as_str() {
            "--color" | "--config" | "-C" | "-Z" => i += 2,
            _ if t.starts_with(['-', '+']) => i += 1,
            _ => break,
        }
    }
    match args.get(i).map(String::as_str) {
        Some("run" | "r") => Some(ClmCall::CargoRun(&args[i + 1..])),
        _ => None,
    }
}

/// Whether a segment with `tokens` runs `clm defect`, or is a `cargo run` with a token
/// `defect`, and asks for no help: `--help` or `-h` files nothing.
fn runs_defect(tokens: &[String]) -> bool {
    let (runs, args) = match clm_call(tokens) {
        Some(ClmCall::Clm(args)) => (args.first().is_some_and(|a| a == "defect"), args),
        Some(ClmCall::CargoRun(args)) => (args.iter().any(|t| t == "defect"), args),
        None => return false,
    };
    runs && !args.iter().any(|t| t == "--help" || t == "-h")
}

/// The report a `clm defect` segment sends, when it is a heredoc the shell sends as written
/// (`defect-ask-shows-report`).
fn shown_report(j: &Judged) -> Option<&str> {
    match &j.stdin {
        Stdin::Heredoc {
            body,
            as_written: true,
        } => Some(body),
        _ => None,
    }
}

/// Whether a segment with `tokens` runs `clm land`, or is a `cargo run` whose `land` token is
/// `land`, and asks for no help after it: `--help` or `-h` lands nothing, so a pipe hides no verdict.
fn runs_land(tokens: &[String]) -> bool {
    let after_land = match clm_call(tokens) {
        Some(ClmCall::Clm(args)) if args.first().is_some_and(|a| a == "land") => &args[1..],
        // The binary's first argument is the token after the first `--`; with no `--`, the
        // guard can't tell cargo's tokens from the binary's, so the first `land` counts.
        Some(ClmCall::CargoRun(tokens)) => match tokens.iter().position(|t| t == "--") {
            Some(d) => match tokens.get(d + 1) {
                Some(t) if t == "land" => &tokens[d + 2..],
                _ => return false,
            },
            None => match tokens.iter().position(|t| t == "land") {
                Some(i) => &tokens[i + 1..],
                None => return false,
            },
        },
        _ => return false,
    };
    !after_land.iter().any(|t| t == "--help" || t == "-h")
}

/// Whether the arguments of `clm capabilities` or `clm architecture` run `decide`: its first positional argument, the
/// values of `--root` and `--format` skipped. Any later `decide` is an argument of `decide`
/// or a usage error.
fn reviews_decide(args: &[String]) -> bool {
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" | "--format" => {
                args.next();
            }
            a if a.starts_with('-') => {}
            a => return a == "decide",
        }
    }
    false
}

/// Whether a segment with `tokens` runs `clm approve`, `clm capabilities decide` or
/// `clm architecture decide`, a review decision (`approve-guard`). A `cargo run` does when
/// one of its tokens is `approve`, or one is `capabilities` or `architecture` and a later
/// one `decide`.
fn approves(tokens: &[String]) -> bool {
    match clm_call(tokens) {
        Some(ClmCall::Clm(args)) => match args.first().map(String::as_str) {
            Some("approve") => true,
            Some("capabilities" | "architecture") => reviews_decide(&args[1..]),
            _ => false,
        },
        Some(ClmCall::CargoRun(tokens)) => {
            tokens.iter().any(|t| t == "approve")
                || tokens
                    .iter()
                    .position(|t| t == "capabilities" || t == "architecture")
                    .is_some_and(|c| tokens[c + 1..].iter().any(|t| t == "decide"))
        }
        None => false,
    }
}
