//! `clm apply`, `clm archive` and `clm hashes` as calls: their arguments, what they print and
//! their exit status (`commands-in-crate`).

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use clm_model::Violation;

/// `clm apply`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct ApplyArgs {
    /// Change ID (directory name under changes/).
    pub change: String,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `clm archive`'s arguments.
#[derive(clap::Args, Debug)]
pub struct ArchiveArgs {
    /// Change ID.
    pub change: String,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `clm hashes`'s arguments.
#[derive(clap::Args, Debug)]
pub struct HashesArgs {
    /// Component ID.
    #[arg(value_name = "COMPONENT")]
    pub node: String,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

#[derive(Parser)]
#[command(
    name = "clm apply",
    no_binary_name = true,
    about = "Apply a change's deltas to specs/, all or nothing."
)]
struct ApplyLine {
    #[command(flatten)]
    args: ApplyArgs,
}

#[derive(Parser)]
#[command(
    name = "clm archive",
    no_binary_name = true,
    about = "Move a done or abandoned change to changes/archive/."
)]
struct ArchiveLine {
    #[command(flatten)]
    args: ArchiveArgs,
}

#[derive(Parser)]
#[command(
    name = "clm hashes",
    no_binary_name = true,
    about = "Print a component's element hashes, for recording as delta bases."
)]
struct HashesLine {
    #[command(flatten)]
    args: HashesArgs,
}

/// `clm apply` with `args` as the command line gives them after the command's name, in `root`
/// unless `--root` overrides it. Writes what the command prints to `out` and `err` and returns
/// its exit status: 0 with the files written and removed, 1 with every violation, 2 for
/// arguments it can't parse. The environment lookup `_env` is taken as every command's call
/// takes it; this command reads none of it.
pub fn apply_command<I, T>(
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
    parsed(args, out, err, |ApplyLine { args }: ApplyLine, out, _| {
        let root = args.root.as_deref().unwrap_or(root);
        match crate::apply(root, &args.change) {
            Ok(ops) => {
                writeln!(out, "applied {}", args.change)?;
                for op in ops {
                    let verb = match op {
                        crate::FileOp::Write(..) => "wrote",
                        crate::FileOp::Remove(_) => "removed",
                    };
                    writeln!(out, "  {verb} {}", op.path().display())?;
                }
                Ok(0)
            }
            Err(violations) => report(&violations, out),
        }
    })
}

/// `clm archive`, as `apply_command` takes it: 0 with the directory the change moved to, 1
/// with the reason it can't move, 2 for arguments it can't parse.
pub fn archive_command<I, T>(
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
    parsed(
        args,
        out,
        err,
        |ArchiveLine { args }: ArchiveLine, out, _| {
            let root = args.root.as_deref().unwrap_or(root);
            match crate::archive(root, &args.change) {
                Ok(to) => writeln!(out, "archived {} -> {}", args.change, to.display()).map(|()| 0),
                Err(violations) => report(&violations, out),
            }
        },
    )
}

/// `clm hashes`, as `apply_command` takes it: 0 with one `<element> <hash>` line per element
/// of the component or entry of the layer, 2 for an unknown component or arguments it can't
/// parse.
pub fn hashes_command<I, T>(
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
    parsed(
        args,
        out,
        err,
        |HashesLine { args }: HashesLine, out, err| {
            let (graph, _) = clm_model::load(args.root.as_deref().unwrap_or(root));
            let node = &args.node;
            let lines = match clm_model::Layer::ALL.into_iter().find(|l| l.id() == node) {
                Some(layer) => crate::layer_hashes(&graph, layer),
                None => match graph.nodes.get(node) {
                    Some(n) => crate::hashes(n),
                    None => {
                        return writeln!(err, "clm hashes: no valid component `{node}`")
                            .map(|()| 2);
                    }
                },
            };
            for (element, hash) in lines {
                writeln!(out, "{element} {hash}")?;
            }
            Ok(0)
        },
    )
}

/// Parses `args` as `L`'s line and runs `run` on it; a usage error goes to `err` with status
/// 2, and `--help` to `out` with status 0, as clap prints them.
fn parsed<L, I, T>(
    args: I,
    out: &mut dyn Write,
    err: &mut dyn Write,
    run: impl FnOnce(L, &mut dyn Write, &mut dyn Write) -> io::Result<u8>,
) -> u8
where
    L: Parser,
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let written = match L::try_parse_from(args) {
        Err(e) if e.use_stderr() => write!(err, "{}", e.render()).map(|()| 2),
        Err(e) => write!(out, "{}", e.render()).map(|()| 0),
        Ok(line) => run(line, out, err),
    };
    written.unwrap_or(2)
}

/// One line per violation, as `clm-model` gives it (`model/violation-line`), with status 1.
fn report(violations: &[Violation], out: &mut dyn Write) -> io::Result<u8> {
    for v in violations {
        writeln!(out, "{v}")?;
    }
    Ok(1)
}
