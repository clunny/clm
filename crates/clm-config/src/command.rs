//! `clm config` as a call (`commands-in-crate`): its arguments, what it prints and its exit
//! status. Native only: the web app compiles this crate to WebAssembly without it.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::Path;

use clap::Parser;

/// The arguments of `clm config`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct Args {
    #[command(subcommand)]
    pub config: Config,
}

/// `clm config`'s subcommands.
#[derive(clap::Subcommand, Debug)]
pub enum Config {
    /// Print a key's value and the layer it came from, tab-separated.
    Get {
        /// The key, such as `defects`.
        key: String,
    },
    /// Print every key with its value and the layer it came from, tab-separated.
    List,
}

/// The command line `command` parses: the command's own arguments, without its name.
#[derive(Parser)]
#[command(name = "clm config", bin_name = "clm config", no_binary_name = true)]
struct Line {
    #[command(flatten)]
    args: Args,
}

/// `clm config`: parses `args`, the strings after the command's name, loads the settings of
/// `root`, the CLM repository `crates/clm` found from the directory it runs in (none outside
/// one), with `env` as the environment, and writes what the command prints to `out` and `err`.
/// Returns the exit status: 0 when it printed the lines, 1 with the error when the key is
/// unknown or the settings can't be loaded, 2 for arguments it can't parse. `_cwd`, the
/// directory it runs in, is taken as every command's call takes it; the root is all of it this
/// command reads.
pub fn command<I, T>(
    args: I,
    root: Option<&Path>,
    _cwd: &Path,
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
        Ok(Line { args }) => report(
            crate::load(root, env).and_then(|s| match args.config {
                Config::Get { key } => s.lines(&key),
                Config::List => s.list(),
            }),
            out,
            err,
        ),
    };
    written.unwrap_or(2)
}

/// Writes the lines, or the error, as the command prints them and gives its exit status.
fn report(
    lines: Result<Vec<String>, crate::Error>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match lines {
        Ok(lines) => {
            for line in lines {
                writeln!(out, "{line}")?;
            }
            Ok(0)
        }
        Err(e) => writeln!(err, "clm config: {e}").map(|()| 1),
    }
}
