//! `clm gate` as a call: its arguments, what it prints and its exit status
//! (`commands-in-crate`). Native only: the web app calls `evaluate_at`.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

/// `clm gate`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Base ref the PR merges into.
    #[arg(long)]
    pub base: String,
    /// The ledger holds an approval of this diff that the policy accepts. `clm land` sets it
    /// from the ledger; passing it by hand only skips `gate/approval`.
    #[arg(long)]
    pub approved: bool,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The parser of `clm gate` alone.
#[derive(Parser)]
#[command(
    name = "clm gate",
    no_binary_name = true,
    about = "Decide whether HEAD may merge into a base ref; exit 1 to block."
)]
struct Line {
    #[command(flatten)]
    args: Args,
}

/// `clm gate` with `args` as the command line gives them after the command's name, in `root`
/// unless `--root` overrides it. Writes what the command prints to `out` and `err` and
/// returns its exit status: 0 to allow, 1 to block, 2 when git fails or for arguments it
/// can't parse. The environment lookup `_env` is taken as every command's call takes it;
/// this command reads none of it.
pub fn command<I, T>(
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
    let written = match Line::try_parse_from(args) {
        Err(e) if e.use_stderr() => write!(err, "{}", e.render()).map(|()| 2),
        Err(e) => write!(out, "{}", e.render()).map(|()| 0),
        Ok(Line { args }) => run(&args, args.root.as_deref().unwrap_or(root), out, err),
    };
    written.unwrap_or(2)
}

fn run(args: &Args, root: &Path, out: &mut dyn Write, err: &mut dyn Write) -> io::Result<u8> {
    let outcome = match crate::evaluate_at(root, &args.base, "HEAD", args.approved) {
        Ok(outcome) => outcome,
        Err(e) => return writeln!(err, "clm gate: {e}").map(|()| 2),
    };
    if outcome.review {
        writeln!(out, "{} review", outcome.kind)?;
    } else {
        writeln!(out, "{}", outcome.kind)?;
    }
    for v in &outcome.violations {
        writeln!(out, "{v}")?;
    }
    Ok(u8::from(!outcome.violations.is_empty()))
}
