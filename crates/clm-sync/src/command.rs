//! `clm rebase` and `clm autosync` as calls (`commands-in-crate`): their arguments, what they
//! print and their exit status. The rebase's call writes each warning as `clm rebase: <text>`
//! (`warnings-name-the-command`).

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

/// The name `clm rebase` is run by, which its usage, warnings and outcome line carry.
const REBASE: &str = "rebase";

/// The arguments of `clm rebase`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct RebaseArgs {
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The arguments of `clm autosync`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct AutosyncArgs {
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

// The command line `rebase_command` parses: the command's own arguments, without its name.
#[derive(Parser)]
#[command(
    no_binary_name = true,
    about = "Rebase the branch onto origin/main, regenerate applied specs and run `clm check`; \
             exit 1 with what is left to resolve"
)]
struct RebaseLine {
    #[command(flatten)]
    args: RebaseArgs,
}

// The command line `autosync_command` parses: the command's own arguments, without its name.
#[derive(Parser)]
#[command(
    no_binary_name = true,
    about = "Push a snapshot of the spec work on a `spec/*` branch to origin as `autosync/<branch>`"
)]
struct AutosyncLine {
    #[command(flatten)]
    args: AutosyncArgs,
}

/// Parses `args` as `P`, the command `usage` names, or writes the usage error (or the help)
/// and gives its status.
fn parsed<P, I, T>(args: I, usage: &str, out: &mut dyn Write, err: &mut dyn Write) -> Result<P, u8>
where
    P: Parser,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let command = P::command()
        .name(usage.to_string())
        .bin_name(usage.to_string());
    let matched = command.try_get_matches_from(args);
    matched.and_then(|m| P::from_arg_matches(&m)).map_err(|e| {
        let written = if e.use_stderr() {
            write!(err, "{}", e.render()).map(|()| 2)
        } else {
            write!(out, "{}", e.render()).map(|()| 0)
        };
        written.unwrap_or(2)
    })
}

/// `clm rebase` with `args`, the strings the command line gives after the command's name, in
/// `root` unless `--root` overrides it, with `env` as the environment. `settings` loads the
/// `health` settings from the CLM root it is given, or says why they can't be loaded, as
/// `sync` takes it. Writes what the command prints to `out` and `err`, each warning and error
/// on `err` as `clm rebase: <text>`, and returns its exit status: 0 when the branch is up to
/// date and passes, 1 when something is left, 2 when git fails or for arguments it can't
/// parse. The last line on `out` states the outcome, `rebase ok`, `rebase incomplete` or
/// `rebase failed` (`outcome-line`).
pub fn rebase_command<I, T>(
    args: I,
    root: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
    settings: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match parsed::<RebaseLine, _, _>(args, &format!("clm {REBASE}"), out, err) {
        Err(code) => code,
        Ok(RebaseLine { args }) => {
            let root = args.root.as_deref().unwrap_or(root);
            rebase(root, env, out, err, settings).unwrap_or(2)
        }
    }
}

fn rebase(
    root: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
    settings: impl FnOnce(&Path) -> Result<clm_health::Settings, String>,
) -> io::Result<u8> {
    let (code, outcome) = match crate::sync(root, env, settings) {
        Ok(report) => {
            for warning in &report.warnings {
                writeln!(err, "clm {REBASE}: {warning}")?;
            }
            for line in &report.lines {
                writeln!(out, "{line}")?;
            }
            if report.ok {
                (0, "ok")
            } else {
                (1, "incomplete")
            }
        }
        Err(e) => {
            writeln!(err, "clm {REBASE}: {e}")?;
            match e.exit_code() {
                1 => (1, "incomplete"),
                code => (code, "failed"),
            }
        }
    };
    writeln!(out, "{REBASE} {outcome}")?;
    Ok(code)
}

/// `clm autosync` with `args`, the strings the command line gives after the command's name,
/// in `root` unless `--root` overrides it. Writes what the command prints to `out` and `err`
/// and returns its exit status: 0 when it pushed or had nothing to push, the failure's status
/// when git fails, 2 for arguments it can't parse.
pub fn autosync_command<I, T>(
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
    match parsed::<AutosyncLine, _, _>(args, "clm autosync", out, err) {
        Err(code) => code,
        Ok(AutosyncLine { args }) => match crate::autosync(args.root.as_deref().unwrap_or(root)) {
            Ok(()) => 0,
            Err(e) => writeln!(err, "clm autosync: {e}").map_or(2, |()| e.exit_code()),
        },
    }
}
