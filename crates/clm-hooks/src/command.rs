//! `clm defect`, `clm hook session-start` and `clm hook autosync` as calls
//! (`commands-in-crate`): their arguments, what they print and their exit status.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;

use clap::Parser;
use clm_agent_harness::AgentHarness;

use crate::Env;

/// The arguments of `clm defect`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct DefectArgs {
    /// The report's title.
    #[arg(long)]
    pub title: String,
    /// A CLM component the defect concerns; repeat for several.
    #[arg(long = "component", value_name = "COMPONENT")]
    pub components: Vec<String>,
}

/// The arguments of `clm hook session-start` and `clm hook autosync`: the agent harness whose
/// event the hook reads (`hook-agent-harness-flag`).
#[derive(clap::Args, Debug)]
pub struct HookArgs {
    /// The agent harness running the hook: `claude-code` (the default), `codex` or `none`.
    #[arg(long = "agent-harness", default_value = "claude-code")]
    pub agent_harness: String,
}

#[derive(Parser)]
#[command(name = "clm defect", bin_name = "clm defect", no_binary_name = true)]
struct DefectLine {
    #[command(flatten)]
    args: DefectArgs,
}

#[derive(Parser)]
#[command(
    name = "clm hook session-start",
    bin_name = "clm hook session-start",
    no_binary_name = true
)]
struct SessionStartLine {
    #[command(flatten)]
    args: HookArgs,
}

#[derive(Parser)]
#[command(
    name = "clm hook autosync",
    bin_name = "clm hook autosync",
    no_binary_name = true
)]
struct AutosyncLine {
    #[command(flatten)]
    args: HookArgs,
}

/// Parses `args` as `P`, or writes clap's message (usage errors to `err`, `--help` to `out`)
/// and gives the status to return: 2 for an error, 0 for help.
fn parse<P: Parser, I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> Result<P, u8>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    P::try_parse_from(args).map_err(|e| {
        let written = match e.use_stderr() {
            true => write!(err, "{}", e.render()).map(|()| 2),
            false => write!(out, "{}", e.render()).map(|()| 0),
        };
        written.unwrap_or(2)
    })
}

/// `clm defect` with `args`, the strings after the command's name, the report as `stdin`.
/// `root` is the CLM repository `crates/clm` found from `cwd` (none outside one): its settings,
/// read through `env`, say where the report goes, and a backlog item is written there; an
/// issue is filed, or kept, from `cwd`. Writes the backlog item's path, the issue's URL or
/// that reporting is off to `out`, and returns 0; writes why it failed to `err`, prefixed
/// `clm defect: `, and returns 1; returns 2 for arguments it can't parse.
pub fn defect_command<I, T>(
    args: I,
    root: Option<&Path>,
    cwd: &Path,
    stdin: &str,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let DefectLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let written: io::Result<u8> =
        match crate::defect::defect(root, cwd, &args.title, &args.components, stdin, env) {
            Ok(filed) => writeln!(out, "{filed}").map(|()| 0),
            Err(why) => writeln!(err, "clm defect: {why}").map(|()| 1),
        };
    written.unwrap_or(2)
}

/// `clm hook session-start` with `args`, the strings after the hook's name, and the agent
/// harness's event as `stdin`: writes what `session_start_hook` gives to `out`, with the
/// settings read through `env`, and returns 0. The hook finds its repository from the event's
/// `cwd`, not from `_root` or `_cwd`, which it takes as every command's call does. An agent
/// harness CLM doesn't know gives nothing to do; arguments it can't parse return 2.
pub fn session_start_command<I, T>(
    args: I,
    _root: Option<&Path>,
    _cwd: &Path,
    stdin: &str,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let SessionStartLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let Some(agent_harness) = AgentHarness::from_name(&args.agent_harness) else {
        return 0;
    };
    let (text, code) = crate::session_start_in(agent_harness, stdin, env);
    write!(out, "{text}").map_or(2, |()| u8::try_from(code).unwrap_or(2))
}

/// `clm hook autosync` with `args`, the strings after the hook's name, and the agent harness's
/// event as `stdin`: autosyncs the repository holding the event's `cwd` as `autosync_hook`
/// does, with the settings read through `env`, writes nothing and returns 0, whatever
/// happens; arguments it can't parse return 2. `_root` and `_cwd` are taken as for
/// `session_start_command`.
pub fn autosync_command<I, T>(
    args: I,
    _root: Option<&Path>,
    _cwd: &Path,
    stdin: &str,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let AutosyncLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let Some(agent_harness) = AgentHarness::from_name(&args.agent_harness) else {
        return 0;
    };
    let (text, code) = crate::autosync_hook_in(agent_harness, stdin, env);
    write!(out, "{text}").map_or(2, |()| u8::try_from(code).unwrap_or(2))
}
