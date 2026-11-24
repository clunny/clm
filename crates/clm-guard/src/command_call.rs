//! `clm hook guard` as a call (`commands-in-crate`): its arguments, what it prints and its
//! exit status.

use std::ffi::OsString;
use std::io::{self, Write};

use clap::Parser;
use clm_agent_harness::AgentHarness;

use crate::{Env, guard_hook_in};

/// The arguments of `clm hook guard`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct GuardArgs {
    /// The agent harness running the hook: `claude-code` (the default), `codex` or `none`.
    #[arg(long = "agent-harness", default_value = "claude-code")]
    pub agent_harness: String,
}

/// The command line `guard_command` parses: the command's own arguments, without its name.
/// Its description is the one the root's parser gives `clm hook guard`, so the help reads the
/// same whichever prints it.
#[derive(Parser)]
#[command(
    name = "clm hook guard",
    bin_name = "clm hook guard",
    no_binary_name = true,
    about = "PreToolUse: exit 2 with the reason when a Bash command would cut a release or push \
             to `main`; print the `ask` decision when it would run `clm approve`, or file a CLM \
             defect as an issue",
    long_about = None
)]
struct Line {
    #[command(flatten)]
    args: GuardArgs,
}

/// `clm hook guard` with `args`, the strings the command line gives after the command's
/// name, judging `event`, the text on standard input, with `env` as the environment. Writes
/// what the command prints to `out` and `err` and returns its exit status: the agent
/// harness's rendering of the decision (`guard-per-agent-harness`), 1 for an agent harness
/// it doesn't know (`guard-fails-open`), and 2 for arguments it can't parse or when a write
/// to either stream fails. The guard places each command by its event's `cwd`, so it takes
/// no repository root.
pub fn guard_command<I, T>(
    args: I,
    event: &str,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let written = match Line::try_parse_from(args) {
        Err(e) if e.use_stderr() => write!(err, "{}", e.render()).map(|()| 2),
        Err(e) => write!(out, "{}", e.render()).map(|()| 0),
        Ok(Line { args }) => run(&args, event, env, out, err),
    };
    // A stream that fails leaves the answer unwritten: 2, so the call's caller can't take it
    // for an answer that let the command through (`commands-in-crate`).
    written.unwrap_or(2)
}

/// Judges `event` for the agent harness `args` names and writes what the hook prints.
fn run(
    args: &GuardArgs,
    event: &str,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    // Exit 1 is no block for any agent harness (`guard-fails-open`), its failed write
    // included.
    let Some(agent_harness) = AgentHarness::from_name(&args.agent_harness) else {
        let _ = writeln!(err, "unknown agent harness `{}`", args.agent_harness);
        return Ok(1);
    };
    let rendered = guard_hook_in(agent_harness, event, env);
    write!(out, "{}", rendered.stdout)?;
    if !rendered.stderr.is_empty() {
        writeln!(err, "{}", rendered.stderr)?;
    }
    Ok(u8::try_from(rendered.exit_code).unwrap_or(2))
}
