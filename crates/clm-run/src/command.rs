//! `clm run`, `clm setup` and `clm superpowers` as calls: their arguments, what they print
//! and their exit status (`commands-in-crate`).

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::{Env, Error, RunError};

/// `clm run`'s arguments, which `crates/clm`'s parser reads too.
#[derive(clap::Args, Debug)]
pub struct RunArgs {
    /// A component ID, as `clm next` prints it.
    #[arg(value_name = "COMPONENT")]
    pub node: String,
    /// Finish the component's build instead of starting one: free its place and, once it
    /// has landed, tidy its worktree.
    #[arg(long)]
    pub finish: bool,
    /// With `--finish`: the build's subagent has handed back, so a lock a running process
    /// holds keeps no worktree.
    #[arg(long, requires = "finish")]
    pub returned: bool,
    /// Hours without a commit after which a claim lapses.
    #[arg(long, default_value_t = clm_orchestrator::DEFAULT_EXPIRY_HOURS)]
    pub expiry: u64,
    /// The alias of the model the build's final review runs on, passed into the prompt
    /// unchecked (default: the build session's own model).
    #[arg(long, value_name = "MODEL", conflicts_with = "finish")]
    pub review_model: Option<String>,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `clm setup`'s arguments.
#[derive(clap::Args, Debug)]
pub struct SetupArgs {
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    pub root: Option<PathBuf>,
}

/// `clm superpowers`'s arguments.
#[derive(clap::Args, Debug)]
pub struct SuperpowersArgs {
    /// The skill's name, such as `brainstorming`.
    pub skill: String,
}

#[derive(Parser)]
#[command(
    name = "clm run",
    no_binary_name = true,
    about = "Claim a component, or pick up its unfinished build, and write its build's prompt."
)]
struct RunLine {
    #[command(flatten)]
    args: RunArgs,
}

#[derive(Parser)]
#[command(
    name = "clm setup",
    no_binary_name = true,
    about = "Run the repository's `[build] setup`."
)]
struct SetupLine {
    #[command(flatten)]
    args: SetupArgs,
}

#[derive(Parser)]
#[command(
    name = "clm superpowers",
    no_binary_name = true,
    about = "Print the path of a skill's SKILL.md in the pinned superpowers copy."
)]
struct SuperpowersLine {
    #[command(flatten)]
    args: SuperpowersArgs,
}

/// `clm run` with `args` as the command line gives them after the command's name, in `root`
/// unless `--root` overrides it, run in `cwd` with the environment `env`. Writes what the
/// command prints to `out` and `err` and returns its exit status: 0 with the launcher, or with
/// `--finish` what it did; 1 when the run is refused; 2 when git fails or for arguments it
/// can't parse; 3 when the clone has no free build place.
pub fn run_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    parsed(args, out, err, |RunLine { args }: RunLine, out, err| {
        let root = args.root.as_deref().unwrap_or(root);
        if args.finish {
            return printed(
                "run",
                crate::finish(root, cwd, &args.node, args.expiry, args.returned),
                out,
                err,
            );
        }
        let review_model = args.review_model.as_deref();
        match crate::run(root, cwd, env, &args.node, args.expiry, review_model) {
            Err(RunError::NoPlace(why)) => writeln!(err, "clm run: {why}").map(|()| 3),
            Err(RunError::Failed(e)) => printed("run", Err(e), out, err),
            Ok(launcher) => printed("run", Ok(launcher), out, err),
        }
    })
}

/// `clm setup`, as `run_command` takes it: 0 when the command passes or none is set, 1
/// otherwise or when the settings can't be loaded, 2 for arguments it can't parse. What the
/// command prints goes to `out` and `err`.
pub fn setup_command<I, T>(
    args: I,
    root: &Path,
    cwd: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    parsed(args, out, err, |SetupLine { args }: SetupLine, out, err| {
        let root = cwd.join(args.root.as_deref().unwrap_or(root));
        let result = crate::setup(&root, env, out, err);
        printed("setup", result, out, err)
    })
}

/// `clm superpowers`, as `run_command` takes it: 0 with the skill's path, 1 when the name
/// isn't a skill's, 2 when the fetch fails or for arguments it can't parse. It reads no
/// repository, so `_root` is unused.
pub fn superpowers_command<I, T>(
    args: I,
    _root: &Path,
    cwd: &Path,
    env: Env,
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
        |SuperpowersLine { args }: SuperpowersLine, out, err| {
            let path = crate::superpowers_skill(&args.skill, cwd, env);
            printed("superpowers", path, out, err)
        },
    )
}

/// What `clm <cmd>` prints for `result`: its text on `out` with status 0, a refusal on `err`
/// with 1, the violations on `err`, one line each as `clm-model` gives it, with 1, and git's
/// failure on `err` with 2: a run that fails prints nothing on `out` (`run`).
fn printed(
    cmd: &str,
    result: Result<String, Error>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match result {
        Ok(text) if text.is_empty() => Ok(0),
        Ok(text) => writeln!(out, "{text}").map(|()| 0),
        Err(Error::Refused(why)) => writeln!(err, "clm {cmd}: {why}").map(|()| 1),
        Err(Error::Spec(violations)) => {
            for v in violations {
                writeln!(err, "{v}")?;
            }
            Ok(1)
        }
        Err(Error::Git(e)) => writeln!(err, "clm {cmd}: {e}").map(|()| 2),
    }
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
