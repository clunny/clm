//! `clm affected` as a call: its arguments, what it prints and its exit status
//! (`commands-in-crate`).

use std::ffi::OsString;
use std::fmt::Display;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{CommandFactory, FromArgMatches};

use crate::{Selection, affected, affected_overlap, affected_readers, why};

/// `clm affected`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Compare with the merge base of this ref and HEAD (default `origin/main`).
    #[arg(long)]
    pub base: Option<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Print only the `file` lines of the tests that read what changed, doc-only paths included.
    #[arg(long)]
    pub readers: bool,
    /// Print what changing both of two commits can break, each against their merge base.
    #[arg(long, num_args = 2, value_names = ["A", "B"])]
    pub overlap: Option<Vec<String>>,
    /// Print why the change selects this path: the chain from it to a changed file.
    #[arg(long, value_name = "PATH")]
    pub why: Option<String>,
}

/// The parser of `clm affected` alone.
#[derive(clap::Parser)]
#[command(
    no_binary_name = true,
    bin_name = "clm affected",
    about = "Print what a change can break: `node <id>` and `file <path>` lines, or `all <path>` when every test must run."
)]
struct Cli {
    #[command(flatten)]
    args: Args,
}

/// The settings `crates/clm` loads for a root: `config`'s `[affected] all` globs, or the
/// line naming why they can't be loaded.
pub type Settings<'a> =
    &'a dyn Fn(&Path, &dyn Fn(&str) -> Option<String>) -> Result<Vec<String>, String>;

/// `clm affected` with `args` as the command line gives them after the command's name, in
/// `root` unless `--root` overrides it, with `env` for the settings that `settings` loads.
/// Writes what the command prints to `out` and `err` and returns its exit status: 2 with the
/// usage error for arguments it can't parse.
pub fn command<I, T>(
    args: I,
    root: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    settings: Settings,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let parsed = Cli::command()
        .try_get_matches_from(args)
        .and_then(|matches| Cli::from_arg_matches(&matches));
    let args = match parsed {
        Ok(cli) => cli.args,
        Err(e) => {
            let text = e.render().to_string();
            if e.use_stderr() {
                let _ = write!(err, "{text}");
            } else {
                let _ = write!(out, "{text}");
            }
            return u8::try_from(e.exit_code()).unwrap_or(2);
        }
    };
    let root = args.root.as_deref().unwrap_or(root);
    match run(root, &args, env, settings, out) {
        Ok(()) => 0,
        Err(Failed::Own(code, message)) => {
            line(err, format!("clm affected: {message}"));
            code
        }
        Err(Failed::Settings(given)) => {
            line(err, given);
            1
        }
    }
}

/// Why the command failed: its own exit status and message, or the line the caller gave for
/// settings it couldn't load, which the command writes as given (`affected-base`).
enum Failed {
    Own(u8, String),
    Settings(String),
}

/// The command's lines on `out`, or why it failed.
fn run(
    root: &Path,
    args: &Args,
    env: &dyn Fn(&str) -> Option<String>,
    settings: Settings,
    out: &mut dyn Write,
) -> Result<(), Failed> {
    let refused = |message: String| Failed::Own(1, message);
    let failed = |e: clm_git::Failure| Failed::Own(e.exit_code(), e.to_string());
    if args.overlap.is_some() {
        for (given, flag) in [
            (args.base.is_some(), "--base"),
            (args.readers, "--readers"),
            (args.why.is_some(), "--why"),
        ] {
            if given {
                return Err(refused(format!(
                    "--overlap and {flag} can't be given together"
                )));
            }
        }
    }
    let base = args.base.as_deref().unwrap_or("origin/main");
    if args.readers && args.why.is_some() {
        return Err(refused(
            "--readers and --why can't be given together".to_string(),
        ));
    }
    if args.readers {
        for file in affected_readers(root, base).map_err(failed)? {
            line(out, format!("file {file}"));
        }
        return Ok(());
    }
    let all = settings(root, env).map_err(Failed::Settings)?;
    if let Some(path) = &args.why {
        for chain in why(root, base, path, &all).map_err(failed)? {
            line(out, chain);
        }
        return Ok(());
    }
    let selection = match &args.overlap {
        Some(revs) => affected_overlap(root, &revs[0], &revs[1], &all),
        None => affected(root, base, &all),
    };
    match selection.map_err(failed)? {
        Selection::All(path) => line(out, format!("all {path}")),
        Selection::Nodes { nodes, files } => {
            for node in nodes {
                line(out, format!("node {node}"));
            }
            for file in files {
                line(out, format!("file {file}"));
            }
        }
    }
    Ok(())
}

/// One line on a stream. The stream `crates/clm` passes for standard output fails as
/// `println!` does when its reader has gone (`closed-stdout`); a buffer never fails.
fn line(stream: &mut dyn Write, text: impl Display) {
    let _ = writeln!(stream, "{text}");
}
