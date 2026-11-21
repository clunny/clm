//! `clm ledger` as a call (`commands-in-crate`): its arguments, what it prints and its exit
//! status. Native only: the web app compiles this crate to WebAssembly without it.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use clm_gate::Violation;

use crate::{Error, Finding};

/// The arguments of `clm ledger`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct LedgerArgs {
    #[command(subcommand)]
    pub ledger: Ledger,
}

impl LedgerArgs {
    /// The `--root` the subcommand was given.
    pub fn root(&self) -> &Option<PathBuf> {
        match &self.ledger {
            Ledger::Compact { root } | Ledger::Verify { root, .. } | Ledger::Show { root } => root,
        }
    }
}

/// `clm ledger`'s subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Ledger {
    /// Rewrite your own ledger without approvals of landed diffs and checks older than the
    /// newest land, push it, and print how many records went.
    Compact {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Report each commit on `main` no `landed` record names, merge or not, and each land no
    /// `checked` record covers; exit 1 when there is one. Doesn't fetch.
    Verify {
        /// Audit only the commits after this one (default: from the first landed commit).
        #[arg(long)]
        since: Option<String>,
        /// Also recompute each signed land's kind and whether it needed review with the gate,
        /// and report a land whose record states less than the gate gives.
        #[arg(long)]
        review: bool,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Print every record, one per line in time order. Doesn't fetch.
    Show {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

// The command line `ledger_command` parses: the command's own arguments, without its name.
// A plain comment, since clap would print a doc comment as the help's description.
#[derive(Parser)]
#[command(
    name = "clm ledger",
    bin_name = "clm ledger",
    no_binary_name = true,
    about = "The ledger of approvals, checks and lands: print it, audit `main` against it, or \
             prune your own superseded records"
)]
struct Line {
    #[command(flatten)]
    args: LedgerArgs,
}

/// `clm ledger` with `args`, the strings the command line gives after the command's name, in
/// `root` unless `--root` overrides it, with `env` as the environment. Writes what the
/// command prints to `out` and `err` and returns its exit status: 0 when it's done, 1 on a
/// `verify` finding, 2 when git or the settings fail or for arguments it can't parse.
pub fn ledger_command<I, T>(
    args: I,
    root: &Path,
    env: &dyn Fn(&str) -> Option<String>,
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
        Ok(Line { args }) => {
            let root = args.root().as_deref().unwrap_or(root).to_path_buf();
            run(args.ledger, &root, env, out, err)
        }
    };
    written.unwrap_or(2)
}

/// Runs the subcommand and writes what it prints, giving its exit status.
fn run(
    ledger: Ledger,
    root: &Path,
    env: &dyn Fn(&str) -> Option<String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    let done = match ledger {
        Ledger::Compact { .. } => crate::compact(root, env).map(|done| {
            let warnings = done.warnings;
            (warnings, vec![format!("removed {}", done.removed)], 0)
        }),
        Ledger::Verify { since, review, .. } => {
            crate::verify(root, "main", since.as_deref(), review).map(|findings| {
                let code = if findings.is_empty() { 0 } else { 1 };
                let lines = findings.into_iter().map(|f| violation(f).to_string());
                (vec![], lines.collect(), code)
            })
        }
        Ledger::Show { .. } => crate::fold(root).map(|fold| {
            let lines = crate::show(&fold);
            (fold.warnings, lines, 0)
        }),
    };
    report(done, out, err)
}

/// Writes the warnings to `err` and the lines to `out`, or the error, and gives the status.
fn report(
    done: Result<(Vec<String>, Vec<String>, u8), Error>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match done {
        Ok((warnings, lines, code)) => {
            for warning in &warnings {
                writeln!(err, "warning: {warning}")?;
            }
            for line in &lines {
                writeln!(out, "{line}")?;
            }
            Ok(code)
        }
        Err(e) => writeln!(err, "clm ledger: {e}").map(|()| 2),
    }
}

/// A `clm ledger verify` finding as a violation against `main`.
fn violation(f: Finding) -> Violation {
    Violation {
        path: "main".into(),
        rule: f.rule,
        message: f.message,
    }
}

/// `clm ledger verify`'s findings when the checked-out branch is `main` and the repository
/// has a ledger branch, which `clm check` reports, so a push to `main` that didn't go through
/// `clm land` fails there (`no-silent-drift`).
pub fn on_main(root: &Path) -> Vec<Violation> {
    let on_main = matches!(clm_git::current_branch(root), Ok(Some(b)) if b == "main");
    if !on_main {
        return Vec::new();
    }
    let audit = crate::verify::has_ledger(root).and_then(|has| match has {
        // Without `--review`: recomputing runs the gate once per signed land, on every check of
        // `main`.
        true => crate::verify(root, "main", None, false),
        false => Ok(Vec::new()),
    });
    match audit {
        Ok(findings) => findings.into_iter().map(violation).collect(),
        // Not audited is not clean.
        Err(e) => vec![Violation {
            path: "main".into(),
            rule: "ledger/verify",
            message: format!("can't audit main against the ledger: {e}"),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(done: Result<(Vec<String>, Vec<String>, u8), Error>) -> (String, String, u8) {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = report(done, &mut out, &mut err).unwrap();
        (
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
            code,
        )
    }

    #[test]
    fn an_error_is_on_the_error_stream_with_2() {
        // req:ledger/commands-in-crate@61f9b60a
        let (out, err, code) = written(Err(Error::Writer("no writer".into())));
        assert_eq!(
            (out.as_str(), err.as_str(), code),
            ("", "clm ledger: no writer\n", 2)
        );
    }

    #[test]
    fn warnings_go_to_the_error_stream_and_lines_to_the_output() {
        // req:ledger/commands-in-crate@61f9b60a
        let done = Ok((vec!["w".into()], vec!["a".into(), "b".into()], 1));
        let (out, err, code) = written(done);
        assert_eq!(
            (out.as_str(), err.as_str(), code),
            ("a\nb\n", "warning: w\n", 1)
        );
    }

    #[test]
    fn a_finding_prints_as_the_models_violation_line() {
        // req:ledger/commands-in-crate@61f9b60a
        let v = violation(Finding {
            rule: "ledger/unrecorded-land",
            commit: "abc".into(),
            message: "abc".into(),
        });
        // `clm-model`'s line, `<path>: <rule>: <message>`, against `main`.
        assert_eq!(v.to_string(), "main: ledger/unrecorded-land: abc");
    }

    #[test]
    fn arguments_it_cannot_parse_return_2_with_the_usage_error() {
        // req:ledger/commands-in-crate@61f9b60a
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let code = ledger_command(
            ["show", "--no-such-option"],
            Path::new("."),
            &|_| None,
            &mut out,
            &mut err,
        );
        assert_eq!(code, 2);
        assert!(out.is_empty());
        let err = String::from_utf8(err).unwrap();
        assert!(err.contains("--no-such-option"), "{err}");
        assert!(err.contains("Usage: clm ledger show"), "{err}");
    }
}
