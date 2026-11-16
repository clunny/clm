//! `clm init`, `clm survey` and `clm guide` as calls (`commands-in-crate`): their arguments,
//! what they print and their exit status.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::{Args, Parser};
use clm_config::Host;

/// The environment, as a lookup.
pub type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The arguments of `clm init`, as the command-line parser reads them.
#[derive(Args, Debug)]
pub struct InitArgs {
    /// A language the repository uses, for its dead-code detector and build script;
    /// repeatable. Known: rust, typescript, python.
    #[arg(long = "lang")]
    pub langs: Vec<String>,
    /// Write no detectors, for an existing codebase whose dead code isn't cleaned up yet.
    #[arg(long)]
    pub no_detectors: bool,
    /// Where the remote lives (default: from `origin`'s URL, or `none` without one).
    #[arg(long, value_parser = ["none", "git", "github", "azure"])]
    pub host: Option<String>,
    /// Write nothing: print the allow rules unattended sessions need that the project's
    /// Claude Code settings lack, and exit 1 when one is missing.
    #[arg(long, conflicts_with_all = ["langs", "no_detectors", "host"])]
    pub check_permissions: bool,
}

/// The arguments of `clm survey`, as the command-line parser reads them.
#[derive(Args, Debug)]
pub struct SurveyArgs {
    /// Check this component map (YAML: `components:` a list of `{ id, paths, depends_on }`, and
    /// optional `outside` globs) against the code instead; exit 1 with what's wrong.
    #[arg(long = "components", value_name = "FILE")]
    pub nodes: Option<PathBuf>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// The arguments of `clm guide`, as the command-line parser reads them.
#[derive(Args, Debug)]
pub struct GuideArgs {
    /// The page to print, by name (`index` for the guide's start).
    pub page: Option<String>,
}

/// A command line one of the calls parses: the command's own arguments, without its name.
#[derive(Parser)]
#[command(no_binary_name = true)]
struct Line<A: Args> {
    #[command(flatten)]
    args: A,
}

/// Parses `args` as the arguments of `name`, such as `clm init`, writing help to `out` and a
/// usage error to `err`; the status to return when there is nothing to run.
fn parse<A: Args, I, T>(
    name: &'static str,
    args: I,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<A, u8>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let command = <Line<A> as clap::CommandFactory>::command()
        .name(name)
        .bin_name(name);
    let parsed = command
        .try_get_matches_from(args)
        .and_then(|mut m| <Line<A> as clap::FromArgMatches>::from_arg_matches_mut(&mut m));
    match parsed {
        Ok(line) => Ok(line.args),
        Err(e) if e.use_stderr() => {
            let _ = write!(err, "{}", e.render());
            Err(2)
        }
        Err(e) => Err(write!(out, "{}", e.render()).map_or(2, |()| 0)),
    }
}

/// `clm init`: parses `args`, the strings after the command's name, and sets up CLM's
/// tooling in the CLM repository found from `cwd`, or at the top of the git repository
/// holding it (`init-root`), reading settings through `env`. `_root`, the root `crates/clm`
/// found, is taken as the Contract states the call; `clm init` finds its repository itself.
/// Returns 0 when it wrote what it could, 1 with the error, or with `--check-permissions`
/// when a rule is missing, and 2 for arguments it can't parse or a stream it can't write.
pub fn init_command<I, T>(
    args: I,
    _root: Option<&Path>,
    cwd: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: InitArgs = match parse("clm init", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let written = if args.check_permissions {
        match crate::check_permissions(cwd) {
            Ok(text) if text.is_empty() => Ok(0),
            Ok(text) => write!(out, "{text}").map(|()| 1),
            Err(e) => writeln!(err, "clm init: {e}").map(|()| 1),
        }
    } else {
        let opts = crate::Options {
            langs: args.langs,
            detectors: !args.no_detectors,
            host: args.host.map(|h| match h.as_str() {
                "none" => Host::None,
                "git" => Host::Git,
                "github" => Host::Github,
                "azure" => Host::Azure,
                other => unreachable!("the parser allows no host `{other}`"),
            }),
        };
        match crate::init(cwd, &opts, env) {
            Ok(text) => write!(out, "{text}").map(|()| 0),
            Err(e) => writeln!(err, "clm init: {e}").map(|()| 1),
        }
    };
    written.unwrap_or(2)
}

/// `clm survey`: parses `args` and prints the map of the repository at `--root`, else
/// `root`, the CLM repository `crates/clm` found from `cwd`, else the top of the git
/// repository holding `cwd` (`root-from-cwd`); with `--components`, a file read relative to
/// `cwd`, checks that map instead. Returns 0 with the map or a map that passes, 1 with one
/// line per violation on `out`, and 2 with the error, for arguments it can't parse or a
/// stream it can't write.
pub fn survey_command<I, T>(
    args: I,
    root: Option<&Path>,
    cwd: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: SurveyArgs = match parse("clm survey", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let root = survey_root(args.root.as_deref(), root, cwd);
    let written = match args.nodes {
        None => match crate::survey(&root) {
            Ok(text) => write!(out, "{text}").map(|()| 0),
            Err(e) => writeln!(err, "clm survey: {e}").map(|()| 2),
        },
        Some(file) => {
            let checked = std::fs::read_to_string(cwd.join(&file))
                .map_err(|e| format!("{}: {e}", file.display()))
                .and_then(|yaml| crate::check_nodes(&root, &file, &yaml));
            match checked {
                Ok(violations) if violations.is_empty() => Ok(0),
                Ok(violations) => violations
                    .iter()
                    .try_for_each(|v| {
                        writeln!(out, "{}: {}: {}", v.path.display(), v.rule, v.message)
                    })
                    .map(|()| 1),
                Err(e) => writeln!(err, "clm survey: {e}").map(|()| 2),
            }
        }
    };
    written.unwrap_or(2)
}

/// The survey's root: `given`, relative to `cwd`; else `found`; else the top of the git
/// repository holding `cwd`, else `cwd` itself.
fn survey_root(given: Option<&Path>, found: Option<&Path>, cwd: &Path) -> PathBuf {
    if let Some(given) = given {
        return cwd.join(given);
    }
    if let Some(found) = found {
        return found.to_path_buf();
    }
    clm_git::git(cwd, &["rev-parse", "--show-toplevel"])
        .ok()
        .filter(|top| !top.is_empty())
        .map_or_else(|| cwd.to_path_buf(), PathBuf::from)
}

/// `clm guide`: parses `args` and prints the guide's index, or the page it names. Reads no
/// file, so `_root`, `_cwd` and `_env` are taken only as the Contract states the call.
/// Returns 0 with the text, 1 with an unknown page's error and the index on `err`, and 2 for
/// arguments it can't parse or a stream it can't write.
pub fn guide_command<I, T>(
    args: I,
    _root: Option<&Path>,
    _cwd: &Path,
    _env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args: GuideArgs = match parse("clm guide", args, out, err) {
        Ok(args) => args,
        Err(code) => return code,
    };
    let written: io::Result<u8> = match crate::guide(args.page.as_deref()) {
        Ok(text) => write!(out, "{text}").map(|()| 0),
        Err(e) => write!(err, "{e}").map(|()| 1),
    };
    written.unwrap_or(2)
}
