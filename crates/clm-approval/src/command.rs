//! `clm approve` as a call: its arguments, what it prints and its exit status
//! (`commands-in-crate`).

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::{Env, Error, approve};

/// Record your approval of a branch's current diff in the ledger.
#[derive(clap::Args, Debug)]
pub struct ApproveArgs {
    /// The branch (default: the current one).
    branch: Option<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    root: Option<PathBuf>,
}

impl ApproveArgs {
    /// The `--root` the command line gives.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }
}

/// The parser of the command alone: its own arguments, without its name.
#[derive(Parser)]
#[command(name = "clm approve", no_binary_name = true)]
struct ApproveLine {
    #[command(flatten)]
    args: ApproveArgs,
}

/// `clm approve` with `args` as the command line gives them after the command's name, in
/// `root` unless `--root` overrides it, a relative `--root` taken from `cwd`. Writes what the
/// command prints to `out` and `err` and returns its exit status: 0 when it approved, 1 when
/// it refused, 2 for a usage error or when git, the ledger or the settings failed.
pub fn approve_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    env: &Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let ApproveLine { args } = match ApproveLine::try_parse_from(args) {
        Ok(line) => line,
        Err(e) => {
            // Usage errors on `err` with 2, help and the version on `out` with 0.
            let written = match e.use_stderr() {
                true => write!(err, "{}", e.render()).map(|()| 2),
                false => write!(out, "{}", e.render()).map(|()| 0),
            };
            return written.unwrap_or(2);
        }
    };
    let root = args
        .root
        .map_or_else(|| root.to_path_buf(), |r| cwd.join(r));
    let written = match approve(&root, args.branch.as_deref(), env) {
        // `approve` warns of nothing: its `Output` holds the lines alone.
        Ok(done) => done
            .lines
            .iter()
            .try_for_each(|line| writeln!(out, "{line}"))
            .map(|()| 0),
        Err(e) => writeln!(err, "clm approve: {e}").map(|()| exit_code(&e)),
    };
    written.unwrap_or(2)
}

fn exit_code(e: &Error) -> u8 {
    match e {
        Error::Refused(_) => 1,
        Error::Git(_) => 2,
    }
}
