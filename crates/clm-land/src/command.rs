//! `clm land` and `clm reconnect` as calls (`land/commands-in-crate`): their arguments, what
//! they print and their exit status.

use std::cell::RefCell;
use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::stderr::Stderr;
use crate::{Env, Error, Output};

/// The arguments of `clm land`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct LandArgs {
    /// The branch (default: the current one).
    pub branch: Option<String>,
    /// Run the land's check command and `clm check` on the working tree and land
    /// nothing: print `checks pass` (after `recorded <sha> as checked` when a clean
    /// committed head counts for a land) or name the command that failed.
    #[arg(long)]
    pub check_only: bool,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The arguments of `clm reconnect`.
#[derive(clap::Args, Debug)]
pub struct ReconnectArgs {
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The command line `land_command` parses: the command's own arguments, without its name.
#[derive(Parser)]
#[command(name = "clm land", bin_name = "clm land", no_binary_name = true)]
struct LandLine {
    #[command(flatten)]
    args: LandArgs,
}

/// The command line `reconnect_command` parses.
#[derive(Parser)]
#[command(
    name = "clm reconnect",
    bin_name = "clm reconnect",
    no_binary_name = true
)]
struct ReconnectLine {
    #[command(flatten)]
    args: ReconnectArgs,
}

/// `clm land` with `args`, the strings the command line gives after the command's name, in
/// `root` unless `--root` overrides it, with `env` as the environment. Writes what the command
/// prints to `out` and `err`, the check's output and the waiting lines as they come, and
/// returns its exit status: 0 when it landed or the checks passed, 1 when it refused, 2 when
/// git, the ledger or the settings failed, or for arguments it can't parse.
pub fn land_command<I, T>(
    args: I,
    root: &Path,
    env: &Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let LandLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let branch = args.branch.as_deref();
    let (done, err) = streamed(err, |stderr| match args.check_only {
        true => crate::check_only::check_only_to(root, branch, env, stderr),
        false => crate::land::land_to(root, branch, env, stderr),
    });
    report("land", done, out, err).unwrap_or(2)
}

/// `clm reconnect`, as `land_command` takes its arguments.
pub fn reconnect_command<I, T>(
    args: I,
    root: &Path,
    env: &Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let ReconnectLine { args } = match parse(args, out, err) {
        Ok(line) => line,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let (done, err) = streamed(err, |stderr| {
        crate::reconnect::reconnect_to(root, env, stderr)
    });
    report("reconnect", done, out, err).unwrap_or(2)
}

/// The parsed command line, or the exit status once the usage error (2, on `err`) or the help
/// (0, on `out`) is written.
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

/// What `run` returns, with what it writes on its stderr while it runs going to `err`, and
/// `err` back for what follows.
fn streamed<R>(err: &mut dyn Write, run: impl FnOnce(Stderr) -> R) -> (R, &mut dyn Write) {
    let cell = RefCell::new(err);
    let write = |bytes: &[u8]| {
        let _ = cell.borrow_mut().write_all(bytes);
    };
    let done = run(Stderr::to(&write));
    (done, cell.into_inner())
}

/// Writes the outcome as `clm <command>` prints it: each warning on `err`, then the lines on
/// `out`, and gives its exit status.
fn report(
    command: &str,
    done: Result<Output, Error>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match done {
        Ok(done) => {
            for warning in done.warnings {
                writeln!(err, "clm {command}: warning: {warning}")?;
            }
            for line in done.lines {
                writeln!(out, "{line}")?;
            }
            Ok(0)
        }
        Err(Error::Refused(why)) => writeln!(err, "clm {command}: {why}").map(|()| 1),
        Err(Error::Git(why)) => writeln!(err, "clm {command}: {why}").map(|()| 2),
    }
}
