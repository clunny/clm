//! `clm pr`, `clm issue` and `clm branch` as calls (`commands-in-crate`): their arguments,
//! what they print and their exit status.

use std::ffi::OsString;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use clap::Parser;

use crate::{Error, adapter, delete_pushed_branch, pr_open, push_branch, take_said};

/// The environment, as a lookup.
type Env<'a> = &'a dyn Fn(&str) -> Option<String>;

/// The arguments of `clm pr`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct PrArgs {
    #[command(subcommand)]
    page: Page,
}

impl PrArgs {
    /// The `--root` it was given.
    pub fn root(&self) -> Option<&Path> {
        match &self.page {
            Page::Open { root, .. } | Page::Body { root, .. } => root.as_deref(),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
enum Page {
    /// Open the branch's pull request against `main` on the host, or find the open one, and
    /// print its URL, or `none` without a host.
    Open {
        /// The branch, such as `spec/x` (default: the checked-out one).
        branch: Option<String>,
        /// The pull request's title (default: drawn from the branch).
        #[arg(long)]
        title: Option<String>,
        /// The file holding the pull request's body (default: drawn from the branch).
        #[arg(long)]
        body_file: Option<PathBuf>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Set the body of the current branch's open pull request to a file's contents.
    Body {
        /// The file holding the body.
        file: PathBuf,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

/// The arguments of `clm issue`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct IssueArgs {
    /// The issue's title.
    #[arg(long)]
    title: String,
    /// The file holding the issue's body.
    #[arg(long)]
    body_file: PathBuf,
    /// The repository to file it in, such as `clunny/clm`.
    #[arg(long)]
    repo: String,
    /// Repository root (default: the CLM repository holding the current directory).
    #[arg(long)]
    root: Option<PathBuf>,
}

impl IssueArgs {
    /// The `--root` it was given.
    pub fn root(&self) -> Option<&Path> {
        self.root.as_deref()
    }
}

/// The arguments of `clm branch`, as the command-line parser reads them.
#[derive(clap::Args, Debug)]
pub struct BranchArgs {
    #[command(subcommand)]
    branch: Branch,
}

impl BranchArgs {
    /// The `--root` it was given.
    pub fn root(&self) -> Option<&Path> {
        match &self.branch {
            Branch::Delete { root, .. } | Branch::Push { root } => root.as_deref(),
        }
    }
}

#[derive(clap::Subcommand, Debug)]
enum Branch {
    /// Delete `origin`'s branch when its tip is still `--lease` (default: the tip last
    /// fetched) and this clone pushed it, on every host.
    Delete {
        /// The branch, such as `impl/a`.
        branch: String,
        /// The commit `origin`'s branch must still be at.
        #[arg(long)]
        lease: Option<String>,
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
    /// Push the checked-out branch to `origin`, overwriting only what this clone last saw.
    Push {
        /// Repository root (default: the CLM repository holding the current directory).
        #[arg(long)]
        root: Option<PathBuf>,
    },
}

#[derive(Parser)]
#[command(name = "clm pr", bin_name = "clm pr", no_binary_name = true)]
struct PrLine {
    #[command(flatten)]
    args: PrArgs,
}

#[derive(Parser)]
#[command(name = "clm issue", bin_name = "clm issue", no_binary_name = true)]
struct IssueLine {
    #[command(flatten)]
    args: IssueArgs,
}

#[derive(Parser)]
#[command(name = "clm branch", bin_name = "clm branch", no_binary_name = true)]
struct BranchLine {
    #[command(flatten)]
    args: BranchArgs,
}

/// Parses `args`, writing clap's message for arguments it can't parse (status 2) or for
/// `--help` (status 0).
fn parse<L: Parser, I, T>(args: I, out: &mut dyn Write, err: &mut dyn Write) -> Result<L, u8>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    L::try_parse_from(args).map_err(|e| {
        let written = if e.use_stderr() {
            write!(err, "{}", e.render()).map(|()| 2)
        } else {
            write!(out, "{}", e.render()).map(|()| 0)
        };
        written.unwrap_or(2)
    })
}

/// `clm pr`: parses `args`, the strings after the command's name, and works in the `--root`
/// given, else `root`, the CLM repository `crates/clm` found from the directory it runs in,
/// with the `host` setting loaded with `env` as the environment. Writes what the command
/// prints to `out` and `err` and returns the exit status: 0 when it did its work, 1 when it
/// refused, 2 when the settings don't load, `git` or the host's tool fails, or the arguments
/// can't be parsed.
pub fn pr_command<I, T>(
    args: I,
    root: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<PrLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = args.root().unwrap_or(root).to_path_buf();
    match args.page {
        Page::Open {
            branch,
            title,
            body_file,
            ..
        } => hosted("pr open", &root, env, out, err, |host, root| {
            let url = pr_open(
                host,
                root,
                branch.as_deref(),
                title.as_deref(),
                body_file.as_deref(),
            )?;
            Ok(Some(url.unwrap_or_else(|| "none".into())))
        }),
        Page::Body { file, .. } => hosted("pr body", &root, env, out, err, |host, root| {
            adapter(host).page_body(root, &file)
        }),
    }
}

/// `clm issue`, as [`pr_command`] takes its arguments.
pub fn issue_command<I, T>(
    args: I,
    root: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<IssueLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = args.root().unwrap_or(root).to_path_buf();
    hosted("issue", &root, env, out, err, |host, root| {
        adapter(host)
            .issue(root, &args.title, &args.body_file, &args.repo)
            .map(Some)
    })
}

/// `clm branch`, as [`pr_command`] takes its arguments.
pub fn branch_command<I, T>(
    args: I,
    root: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = match parse::<BranchLine, _, _>(args, out, err) {
        Ok(line) => line.args,
        Err(code) => return code,
    };
    let root = args.root().unwrap_or(root).to_path_buf();
    match args.branch {
        Branch::Delete { branch, lease, .. } => {
            hosted("branch delete", &root, env, out, err, |host, root| {
                delete_pushed_branch(&*adapter(host), root, &branch, lease.as_deref())
                    .map(|()| None)
            })
        }
        Branch::Push { .. } => hosted("branch push", &root, env, out, err, |_, root| {
            push_branch(root).map(Some)
        }),
    }
}

/// Runs `op` with the repository's `host` setting, writing the line it returns, if any, and
/// giving the exit status: 1 when it refuses, 2 when the settings don't load or `git` or the
/// host's tool fails, or a stream can't be written.
fn hosted(
    command: &str,
    root: &Path,
    env: Env,
    out: &mut dyn Write,
    err: &mut dyn Write,
    op: impl FnOnce(clm_config::Host, &Path) -> Result<Option<String>, Error>,
) -> u8 {
    let written = match clm_config::load(Some(root), env) {
        Err(e) => writeln!(err, "clm {command}: {e}").map(|()| 2),
        Ok(settings) => {
            let host = settings.host.value;
            take_said();
            let result = op(host, root);
            take_said()
                .iter()
                .try_for_each(|line| writeln!(err, "{line}"))
                .and_then(|()| report(command, host, result, out, err))
        }
    };
    written.unwrap_or(2)
}

/// Writes what `result` prints and gives its exit status.
fn report(
    command: &str,
    host: clm_config::Host,
    result: Result<Option<String>, Error>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> io::Result<u8> {
    match result {
        Ok(line) => {
            if let Some(line) = line {
                writeln!(out, "{line}")?;
            }
            Ok(0)
        }
        Err(Error::Refused(why)) => writeln!(err, "clm {command}: {why}").map(|()| 1),
        Err(Error::Nowhere(report)) => {
            write!(out, "{report}")?;
            if !report.ends_with('\n') {
                writeln!(out)?;
            }
            writeln!(err, "clm {command}: host `{host}` has nowhere to file it").map(|()| 1)
        }
        Err(Error::Git(e)) => writeln!(err, "clm {command}: {e}").map(|()| 2),
    }
}
