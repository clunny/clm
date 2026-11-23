//! `clm status`, `clm view` and `clm pending` as calls (`commands-in-crate`): each parses the
//! command's arguments from the strings the command line gives, writes what the command
//! prints to the streams it is given and returns the exit status. `crates/clm` routes each
//! command here and adds nothing to what it writes.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::{Args, FromArgMatches, ValueEnum};
use clm_model::Violation;
use clm_view::{DEFAULT_WIDTH, Format, Name};

use crate::{Budgets, BudgetsFrom, Error, Source};

/// The environment, as a lookup of a variable's value.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// `config`'s `node_size` for a root, as `crates/clm` loads it: the budgets of a node that
/// sets none, or why the settings can't be loaded.
pub type NodeSize<'a> = &'a dyn Fn(&Path) -> Result<Budgets, Violation>;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ViewName {
    Spec,
    Flight,
    Blockers,
    Queue,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum ViewFormat {
    Text,
    Ansi,
    Svg,
    Json,
}

impl From<ViewFormat> for Format {
    fn from(f: ViewFormat) -> Self {
        match f {
            ViewFormat::Text => Format::Text,
            ViewFormat::Ansi => Format::Ansi,
            ViewFormat::Svg => Format::Svg,
            ViewFormat::Json => Format::Json,
        }
    }
}

/// Print a summary of the repository, then its components, changes and claims; never fetches.
#[derive(Args, Debug)]
pub struct StatusArgs {
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// Print a view of the repository: the architecture, the work in flight or what holds it up,
/// or what the build loop runs next; never fetches.
#[derive(Args, Debug)]
pub struct ViewArgs {
    /// The view.
    pub name: ViewName,
    /// The form: a chart, the chart with ANSI colour, an SVG document or the model.
    #[arg(long, value_enum, default_value_t = ViewFormat::Text)]
    pub format: ViewFormat,
    /// Columns the chart fits in, at least 1.
    #[arg(long, default_value_t = DEFAULT_WIDTH as u16, value_parser = clap::value_parser!(u16).range(1..))]
    pub width: u16,
    /// Describe this commit, read from git's objects, instead of the working tree.
    #[arg(long)]
    pub at: Option<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// List the requirements whose code hasn't caught up with their spec: behind, unbuilt or
/// obsolete, with what blocks them.
#[derive(Args, Debug)]
pub struct PendingArgs {
    /// Component ID; every component when omitted.
    #[arg(value_name = "COMPONENT")]
    pub node: Option<String>,
    /// List this commit's pending requirements, read from git's objects, instead of the
    /// working tree's.
    #[arg(long)]
    pub at: Option<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `clm status`: the summary on `out` and 0, or the spec graph's violations and 1. With
/// settings that don't load, `size not checked: <why>` on `err` and no node marked oversized.
pub fn status_command<I, S>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
    node_size: NodeSize,
) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: StatusArgs = match parse("status", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let budgets = budgets(root, node_size, err);
    match crate::status_to(root, budgets, err) {
        Ok(text) => written(out, &text),
        Err(violations) => report(&violations, out),
    }
}

/// `clm view <name>`: the view on `out` and 0; the spec graph's violations on `out` and 1;
/// a revision that couldn't be read on `err` and 1; for the queue as `svg`, why on `err` and
/// 2. Only `spec` marks oversized nodes, so the settings load only for it: the working
/// tree's from `node_size`, a commit's `clm.toml` through `budgets_from`.
pub fn view_command<I, S>(
    args: I,
    root: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
    node_size: NodeSize,
    budgets_from: BudgetsFrom,
) -> u8
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let args: ViewArgs = match parse("view", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let spec = matches!(args.name, ViewName::Spec);
    let source = match args.at.as_deref() {
        None => Source::Tree(if spec {
            budgets(root, node_size, err)
        } else {
            None
        }),
        Some(rev) => Source::At {
            rev,
            budgets: spec.then_some(budgets_from),
        },
    };
    let (format, width) = (args.format.into(), usize::from(args.width));
    let name = match args.name {
        ViewName::Spec => Name::Spec,
        ViewName::Flight => Name::Flight,
        ViewName::Blockers => Name::Blockers,
        // The queue is the loop's plan, which has no layout for a diagram.
        ViewName::Queue if matches!(args.format, ViewFormat::Svg) => {
            let _ = writeln!(err, "clm view: the queue has no svg form");
            return 2;
        }
        ViewName::Queue => return result(crate::queue(root, format, width, source, err), out, err),
    };
    result(
        crate::view(root, name, format, width, source, err),
        out,
        err,
    )
}

/// `clm pending [<component>] [--at <rev>]`: the line naming the commit on `err`, then the
/// pending lines on `out` and 0, or the spec graph's violations on `out` and 1, or a revision
/// that couldn't be read on `err` and 1.
pub fn pending_command<I, S>(
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
    let args: PendingArgs = match parse("pending", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = args.root.as_deref().unwrap_or(root);
    let (head, list) = crate::pending(root, args.node.as_deref(), args.at.as_deref(), err);
    if let Some(head) = head {
        let _ = writeln!(err, "{head}");
    }
    let text = list.map(|list| {
        list.iter()
            .map(|line| format!("{line}\n"))
            .collect::<String>()
    });
    result(text, out, err)
}

/// The budgets `node_size` gives for `root`, or `None` with `size not checked: <why>` on `err`.
fn budgets(root: &Path, node_size: NodeSize, err: &mut dyn Write) -> Option<Budgets> {
    node_size(root)
        .inspect_err(|v| {
            let _ = writeln!(err, "size not checked: {}", v.message);
        })
        .ok()
}

/// What a command printed: the text on `out` and 0, or the violations on `out` and 1, or a
/// revision that couldn't be read on `err` and 1.
fn result(result: Result<String, Error>, out: &mut dyn Write, err: &mut dyn Write) -> u8 {
    match result {
        Ok(text) => written(out, &text),
        Err(Error::Violations(violations)) => report(&violations, out),
        Err(Error::At(why)) => {
            let _ = writeln!(err, "{why}");
            1
        }
    }
}

/// `text` on `out` and 0, or 1 when it can't be written.
fn written(out: &mut dyn Write, text: &str) -> u8 {
    u8::from(out.write_all(text.as_bytes()).is_err())
}

/// One line per violation on `out`, as `clm-model` gives it; 1.
fn report(violations: &[Violation], out: &mut dyn Write) -> u8 {
    for v in violations {
        let _ = writeln!(out, "{v}");
    }
    1
}

/// `A` parsed from `args`, the strings after the command's name. A usage error is written to
/// `err` (help to `out`), and `Err` carries the status the command exits with: 2 for a usage
/// error, 0 for help, or 1 when the help can't be written to `out`.
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
        if e.use_stderr() {
            let _ = write!(err, "{}", e.render());
            return u8::try_from(e.exit_code()).unwrap_or(2);
        }
        written(out, &e.render().to_string())
    })
}
