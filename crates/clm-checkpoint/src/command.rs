//! `clm wrap-up` as a call (`commands-in-crate`): its arguments, what it prints and its exit
//! status.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::{Error, Report};

/// The arguments of `clm wrap-up`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct WrapUpArgs {
    /// Repository root, or any worktree of it (default: the CLM repository holding the
    /// current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The command line `wrap_up_command` parses: the command's own arguments, without its name.
#[derive(Parser)]
#[command(name = "clm wrap-up", bin_name = "clm wrap-up", no_binary_name = true)]
struct WrapUpLine {
    #[command(flatten)]
    args: WrapUpArgs,
}

/// `clm wrap-up` with `args`, the strings the command line gives after the command's name, in
/// `root` unless `--root` overrides it. Writes what the command prints to `out` and `err` and
/// returns its exit status: 0 with `wrap-up ok`, 1 with `wrap-up incomplete` or when it
/// refused, 2 when git failed or for arguments it can't parse. Checkpoint reads no variable
/// of its own: `git`, which it runs, inherits the process's environment, so `_env` is the
/// lookup every command's call takes.
pub fn wrap_up_command<I, T>(
    args: I,
    root: &Path,
    _env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let WrapUpLine { args } = match WrapUpLine::try_parse_from(args) {
        Ok(line) => line,
        Err(e) => {
            let written = match e.use_stderr() {
                true => write!(err, "{}", e.render()).map(|()| 2),
                false => write!(out, "{}", e.render()).map(|()| 0),
            };
            return written.unwrap_or(2);
        }
    };
    let root = args.root.as_deref().unwrap_or(root);
    report(crate::checkpoint(root), out, err).unwrap_or(2)
}

/// Writes the outcome as `clm wrap-up` prints it, and gives its exit status.
fn report(done: Result<Report, Error>, out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    match done {
        Ok(report) => {
            for line in &report.lines {
                writeln!(out, "{line}")?;
            }
            Ok(if report.ok { 0 } else { 1 })
        }
        Err(Error::Refused(why)) => writeln!(err, "clm wrap-up: {why}").map(|()| 1),
        Err(Error::Spec(violations)) => {
            for v in &violations {
                writeln!(out, "{}", clm_check::format(v))?;
            }
            Ok(1)
        }
        Err(Error::Git(why)) => writeln!(err, "clm wrap-up: {why}").map(|()| 2),
    }
}
