//! `clm rename` as a call: its arguments, what it prints and its exit status
//! (`commands-in-crate`).

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

/// `clm rename`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct RenameArgs {
    /// Change ID (directory name under changes/).
    pub change: String,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    name = "clm rename",
    no_binary_name = true,
    about = "Carry out the renames a proposed change records: its deltas, applied, with every \
             reference re-pointed, and the change archived as done."
)]
struct RenameLine {
    #[command(flatten)]
    args: RenameArgs,
}

/// `clm rename` with `args` as the command line gives them after the command's name, in `root`
/// unless `--root` overrides it. Writes what the command prints to `out` and `err` and returns
/// its exit status: 0 with one line per rename, rebased element and leftover on `out`; the
/// refusal's or git's message on `err` with 1 or 2 (`Failure::exit_code`); 2 with the usage
/// error on `err` for arguments it can't parse, 0 with `--help` on `out`. The environment
/// lookup `_env` is taken as every command's call takes it; this command reads none of it.
pub fn rename_command<I, T>(
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
    let written: io::Result<u8> = match RenameLine::try_parse_from(args) {
        Err(e) if e.use_stderr() => write!(err, "{}", e.render()).map(|()| 2),
        Err(e) => write!(out, "{}", e.render()).map(|()| 0),
        Ok(RenameLine { args }) => {
            let root = args.root.as_deref().unwrap_or(root);
            match crate::rename(root, &args.change) {
                Ok(lines) => lines
                    .iter()
                    .try_for_each(|line| writeln!(out, "{line}"))
                    .map(|()| 0),
                Err(failure) => writeln!(err, "{failure}").map(|()| failure.exit_code()),
            }
        }
    };
    written.unwrap_or(2)
}
