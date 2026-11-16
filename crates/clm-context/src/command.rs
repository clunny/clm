//! `clm context` as a call: its arguments, what it prints and its exit status
//! (`commands-in-crate`). Native only: the web app calls `bundle_from`.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

/// `clm context`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct Args {
    /// Component ID.
    #[arg(value_name = "COMPONENT")]
    pub node: String,
    /// Print the bundle's content hash instead of the bundle.
    #[arg(long)]
    pub hash: bool,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The parser of `clm context` alone.
#[derive(Parser)]
#[command(
    name = "clm context",
    no_binary_name = true,
    about = "Print the context bundle an agent needs for a component, its ancestors' specs among it."
)]
struct Line {
    #[command(flatten)]
    args: Args,
}

/// `clm context` with `args` as the command line gives them after the command's name, in
/// `root` unless `--root` overrides it. Writes what the command prints to `out` and `err`
/// and returns its exit status: 0 with the bundle or its hash, 1 with the graph's
/// violations, 2 for an unknown component or arguments it can't parse. The environment
/// lookup `_env` is taken as every command's call takes it; this command reads none of it.
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
    let (graph, violations) = clm_model::load(root);
    if !violations.is_empty() {
        for v in &violations {
            writeln!(out, "{}: {}: {}", v.path.display(), v.rule, v.message)?;
        }
        return Ok(1);
    }
    match crate::bundle(&graph, root, &args.node) {
        Some(bundle) if args.hash => writeln!(out, "{}", crate::hash(&bundle)).map(|()| 0),
        Some(bundle) => write!(out, "{bundle}").map(|()| 0),
        None => writeln!(err, "clm context: no component `{}`", args.node).map(|()| 2),
    }
}
