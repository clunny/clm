//! `clm check`, `clm size`, `clm coverage` and `clm backlog` as calls (`commands-in-crate`):
//! each parses the command's arguments from the strings the command line gives, writes what
//! the command prints to the streams it is given and returns the exit status. `crates/clm`
//! routes each command here and adds nothing to what it writes.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Args, FromArgMatches};

use crate::{Budgets, Detector, Violation};

/// Run every deterministic check; exit 1 on any violation.
#[derive(Args, Debug)]
pub struct CheckArgs {
    /// Run no dead-code or unused-dependency detector, for a caller that knows no code
    /// changed.
    #[arg(long)]
    pub no_detectors: bool,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// List every requirement as current, behind or unbuilt (with citing markers), then the
/// obsolete ones.
#[derive(Args, Debug)]
pub struct CoverageArgs {
    /// Print, in place of the coverage, the tests a removed requirement leaves with
    /// nothing to cite.
    #[arg(long, conflicts_with = "deleted_tests")]
    pub obsolete_tests: bool,
    /// Print, in place of the coverage, the citing tests of this revision that the
    /// working tree no longer holds.
    #[arg(long, value_name = "BASE")]
    pub deleted_tests: Option<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Report each component's spec and code size in tokens against its budget (advisory).
#[derive(Args, Debug)]
pub struct SizeArgs {
    /// Component ID; every component when omitted.
    #[arg(value_name = "COMPONENT")]
    pub node: Option<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// List backlog items: ideas, deferred work, questions and bugs.
#[derive(Args, Debug)]
pub struct BacklogArgs {
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// What `clm check` reads from other components for the root it checks: the detectors the
/// settings name, or why the settings can't be loaded, and the violations of rules other
/// components decide, such as the ledger's audit of `main`.
pub struct CheckInputs<'a> {
    pub detectors: &'a dyn Fn(&Path) -> Result<Vec<Detector>, Violation>,
    pub more: &'a dyn Fn(&Path) -> Vec<Violation>,
}

/// The environment, as a lookup of a variable's value.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// `clm check`: every deterministic rule, the import rule, the detectors unless
/// `--no-detectors`, then `inputs.more`; one line per violation on `out`, 1 when there is one.
pub fn check_command<I, S>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
    inputs: &CheckInputs,
) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: CheckArgs = match parse("check", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let mut violations = crate::run(root);
    match clm_init::import_edges(root) {
        Ok(edges) => {
            let (graph, _) = clm_model::load(root);
            violations.extend(crate::undeclared_imports(&graph, &edges));
        }
        Err(e) => violations.push(Violation {
            path: root.to_path_buf(),
            rule: "check/undeclared-imports",
            message: e,
        }),
    }
    if !args.no_detectors {
        match (inputs.detectors)(root) {
            Ok(list) => {
                // A detector whose `requires` names nothing here is skipped, and named.
                let list: Vec<_> = list
                    .into_iter()
                    .filter(|d| match &d.requires {
                        Some(path) if !root.join(path).exists() => {
                            let _ = writeln!(err, "{} skipped: no {path}", d.name);
                            false
                        }
                        _ => true,
                    })
                    .collect();
                violations.extend(crate::detectors(root, &list, err));
            }
            Err(v) => violations.push(v),
        }
    }
    violations.extend((inputs.more)(root));
    report(&violations, out);
    u8::from(!violations.is_empty())
}

/// `clm coverage`: the coverage lines, or one of the two lists in their place; the violations
/// and 1 when the graph doesn't load or the base is no commit.
pub fn coverage_command<I, S>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: CoverageArgs = match parse("coverage", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let result = match (&args.deleted_tests, args.obsolete_tests) {
        (Some(base), _) => crate::lists::deleted_tests_lines(root, base),
        (None, true) => crate::lists::obsolete_tests_lines(root),
        (None, false) => crate::lists::coverage_lines(root),
    };
    lines(result, out)
}

/// `clm size [<component>]`: each component's line, or only the one named, with `budgets` as
/// the budgets a component doesn't set; 1 for an unknown component or budgets that can't be
/// loaded.
pub fn size_command<I, S>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
    budgets: &dyn Fn(&Path) -> Result<Budgets, Violation>,
) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: SizeArgs = match parse("size", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let node = args.node.as_deref();
    let defaults = match budgets(root) {
        Ok(defaults) => defaults,
        Err(v) => return report(&[v], out),
    };
    // Advisory: the sizes of the nodes that loaded, whatever `clm check` would report.
    let (graph, _) = clm_model::load(root);
    if let Some(node) = node.filter(|n| !graph.nodes.contains_key(*n)) {
        return report(
            &[Violation {
                path: Path::new("specs")
                    .join(node.replace('.', "/"))
                    .join("SPEC.md"),
                rule: "check/node-size",
                message: format!("unknown component `{node}`"),
            }],
            out,
        );
    }
    match crate::size(root, &graph, &crate::scan(root).req, defaults) {
        Ok(sizes) => {
            for s in sizes.iter().filter(|s| node.is_none_or(|n| s.node == n)) {
                let _ = writeln!(out, "{s}");
            }
            0
        }
        Err(e) => {
            let _ = writeln!(out, "check/node-size: {e}");
            1
        }
    }
}

/// `clm backlog`: one line per backlog item, by priority, then kind, then slug.
pub fn backlog_command<I, S>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: BacklogArgs = match parse("backlog", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    lines(
        crate::backlog(root).map(|items| items.iter().map(ToString::to_string).collect()),
        out,
    )
}

/// `A` parsed from `args`, the strings after the command's name. A usage error is written to
/// `err` (help to `out`), and `Err` carries the status the command exits with: 2 for a usage
/// error.
fn parse<'a, A, I, S>(
    name: &'static str,
    args: I,
    out: &'a mut dyn Write,
    err: &'a mut dyn Write,
) -> Result<A, u8>
where
    A: Args + FromArgMatches,
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let command = A::augment_args(clap::Command::new(name).bin_name(format!("clm {name}")));
    let argv = std::iter::once(OsString::from(name)).chain(args.into_iter().map(Into::into));
    let parsed = command
        .try_get_matches_from(argv)
        .and_then(|matches| A::from_arg_matches(&matches));
    parsed.map_err(|e| {
        let stream = if e.use_stderr() { err } else { out };
        let _ = write!(stream, "{}", e.render());
        u8::try_from(e.exit_code()).unwrap_or(2)
    })
}

/// `lines` one per line on `out` and 0, or the violations and 1.
fn lines(result: Result<Vec<String>, Vec<Violation>>, out: &mut dyn Write) -> u8 {
    match result {
        Ok(lines) => {
            for line in lines {
                let _ = writeln!(out, "{line}");
            }
            0
        }
        Err(violations) => report(&violations, out),
    }
}

/// One line per violation on `out`; 1.
fn report(violations: &[Violation], out: &mut dyn Write) -> u8 {
    for v in violations {
        let _ = writeln!(out, "{}", crate::format(v));
    }
    1
}
