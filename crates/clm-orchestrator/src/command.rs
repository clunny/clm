//! `clm next`, `clm claim` and `clm proposals` as calls (`commands-in-crate`): their
//! arguments, what they print and their exit status. Native only: the web app reads the
//! queue through `queue_from_refs`.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::{DEFAULT_EXPIRY_HOURS, Error};

/// The environment, as a lookup.
type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// `clm next`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct NextArgs {
    /// Hours without a commit after which a claim lapses.
    #[arg(long, default_value_t = DEFAULT_EXPIRY_HOURS)]
    pub expiry: u64,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `clm claim`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct ClaimArgs {
    /// A component ID, as `clm next` prints it, or `backlog/<slug>`; with `--fix`, any
    /// component ID.
    #[arg(value_name = "COMPONENT")]
    pub node: String,
    /// Claim the component for a code-only fix of this bug or deferred item (`backlog/<slug>`),
    /// whatever its state.
    #[arg(long, value_name = "backlog/<slug>")]
    pub fix: Option<String>,
    /// Hours without a commit after which a claim lapses.
    #[arg(long, default_value_t = DEFAULT_EXPIRY_HOURS)]
    pub expiry: u64,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `clm proposals`' arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct ProposalsArgs {
    /// Print only lines whose components include this one (repeatable).
    #[arg(long = "component", value_name = "COMPONENT")]
    pub nodes: Vec<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    name = "clm next",
    no_binary_name = true,
    about = "List the components ready to build from origin/main, most urgent first."
)]
struct NextLine {
    #[command(flatten)]
    args: NextArgs,
}

#[derive(Parser)]
#[command(
    name = "clm claim",
    no_binary_name = true,
    about = "Claim a component by pushing its impl/<component> branch, or a backlog item by \
             pushing propose/<slug>."
)]
struct ClaimLine {
    #[command(flatten)]
    args: ClaimArgs,
}

#[derive(Parser)]
#[command(
    name = "clm proposals",
    no_binary_name = true,
    about = "List the changes that hold or propose backlog items, and the live backlog claims."
)]
struct ProposalsLine {
    #[command(flatten)]
    args: ProposalsArgs,
}

/// `args` parsed as `L`, or the status after writing clap's message: 2 with the usage error
/// on `err`, 0 with help or the version on `out`.
fn parse<L, I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> Result<L, u8>
where
    L: Parser,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    L::try_parse_from(args).map_err(|e| {
        let written = if e.use_stderr() {
            write!(err, "{}", e.render()).map(|()| 2)
        } else {
            write!(out, "{}", e.render()).map(|()| 0)
        };
        written.unwrap_or(2)
    })
}

/// `clm next` with `args` as the command line gives them after the command's name, in `root`
/// unless `--root` overrides it, with `max_builds` as `crates/clm` loaded it from the
/// settings, or the reason they didn't load. Writes what the command prints to `out` and
/// `err` and returns its exit status: 0 with the ready components, 1 when the settings or the
/// specs don't load, 2 when git fails or for arguments it can't parse. The environment
/// lookup `_env` is taken as every command's call takes it; this command reads none of it.
pub fn next_command<I, T>(
    args: I,
    root: &Path,
    _env: Env,
    max_builds: Result<u64, String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<NextLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let result = max_builds
        .map_err(|e| Error::Refused(format!("settings: {e}")))
        .and_then(|cap| crate::next_look(root, args.expiry, cap));
    let written = match result {
        Ok(look) => look
            .no_place
            .iter()
            .try_for_each(|line| writeln!(err, "{line}"))
            .and_then(|()| report("next", Ok(look.components.join("\n")), out, err)),
        Err(e) => report("next", Err(e), out, err),
    };
    written.unwrap_or(2)
}

/// `clm claim`, as [`next_command`] takes its arguments, with no settings: 0 with the branch
/// it claimed, 1 when it refuses or the specs don't load, 2 when git fails or for arguments
/// it can't parse.
pub fn claim_command<I, T>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<ClaimLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let result = match &args.fix {
        Some(item) => crate::claim_fix(root, &args.node, item, args.expiry),
        None => crate::claim(root, &args.node, args.expiry),
    };
    report("claim", result, out, err).unwrap_or(2)
}

/// `clm proposals`, as [`claim_command`] takes its arguments: 0 with its lines, each warning
/// on `err` first, 1 when the specs don't load, 2 when git fails or for arguments it can't
/// parse.
pub fn proposals_command<I, T>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<ProposalsLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let written = match crate::proposals(root, &args.nodes) {
        Ok(p) => p
            .warnings
            .iter()
            .try_for_each(|w| writeln!(err, "clm proposals: warning: {w}"))
            .and_then(|()| report("proposals", Ok(p.lines.join("\n")), out, err)),
        Err(e) => report("proposals", Err(e), out, err),
    };
    written.unwrap_or(2)
}

/// Writes an orchestrator result and gives its exit status: the text on `out` and 0; a
/// refusal as `clm <command>: <why>` on `err` and 1; the specs' violations on `out`, one line
/// each, and 1; git's failure as `clm <command>: <error>` on `err` and 2.
fn report(
    command: &str,
    result: Result<String, Error>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match result {
        Ok(text) if text.is_empty() => Ok(0),
        Ok(text) => writeln!(out, "{text}").map(|()| 0),
        Err(Error::Refused(why)) => writeln!(err, "clm {command}: {why}").map(|()| 1),
        Err(Error::Spec(violations)) => {
            for v in &violations {
                writeln!(out, "{}", clm_check::format(v))?;
            }
            Ok(1)
        }
        Err(Error::Git(e)) => writeln!(err, "clm {command}: {e}").map(|()| 2),
    }
}
