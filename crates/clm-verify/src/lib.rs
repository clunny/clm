//! `clm verify`: a project's slow tests run after the land, at the moments the project
//! chooses, and the result recorded in the ledger (specs/verify/SPEC.md,
//! `capability/slow-tests-after-land`). Every `git` call goes through `clm-git`, the ledger
//! through `clm-ledger`, the host's pipeline runs through `clm-host`, and the check through the
//! runner `clm-land` shares.

mod bisect;
mod chain;
mod due;
mod file;
mod import;
mod kept;
mod mutate;
mod run;
mod tree;

pub use kept::kept_log;

use std::cell::{Cell, RefCell};
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub use clm_land::Env;

use chain::State;

/// What `clm verify` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// `--due [--event work-done] [--dry-run]`.
    Due { work_done: bool, dry_run: bool },
    /// `--commit <sha>`.
    Commit(String),
    /// `--import`.
    Import,
    /// `--bisect <sha>`.
    Bisect(String),
}

/// What a verification came to: the one `verify ...` line for stdout, notes for stderr, and
/// the exit code (0 passed, nothing due or nothing to import; 1 failed; 3 nothing due under
/// `--dry-run`; 4 pending).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub line: String,
    pub notes: Vec<String>,
    pub code: u8,
    /// The position in the chain of the commit this outcome verified with a passing check
    /// here, which the mutation pass may follow (`mutation-pass`).
    pub(crate) mutate: Option<usize>,
}

impl Outcome {
    pub(crate) fn new(line: impl Into<String>, code: u8) -> Self {
        Outcome {
            line: line.into(),
            notes: Vec::new(),
            code,
            mutate: None,
        }
    }
}

#[derive(Debug)]
pub enum Error {
    /// Nothing was verified, for the reason given: exit 1.
    Refused(String),
    /// git, the ledger or a file failed: exit 2.
    Git(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Refused(why) | Error::Git(why) => f.write_str(why),
        }
    }
}

impl From<clm_git::Error> for Error {
    fn from(e: clm_git::Error) -> Self {
        Error::Git(e.to_string())
    }
}

impl From<clm_ledger::Error> for Error {
    fn from(e: clm_ledger::Error) -> Self {
        Error::Git(e.to_string())
    }
}

impl From<clm_config::Error> for Error {
    /// Settings that can't be used are a reason, not a git failure.
    fn from(e: clm_config::Error) -> Self {
        Error::Refused(e.to_string())
    }
}

impl From<clm_land::Error> for Error {
    fn from(e: clm_land::Error) -> Self {
        match e {
            clm_land::Error::Refused(why) => Error::Refused(why),
            clm_land::Error::Git(why) => Error::Git(why),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Git(e.to_string())
    }
}

/// Says `line` on stderr. A stderr that fails is no reason to end a run, since the check may be
/// running unwatched or its result not yet recorded, so the write's result is dropped
/// (`verify/verify-run`).
pub(crate) fn say(line: &str) {
    let _ = writeln!(std::io::stderr(), "{line}");
}

/// The text of the file `source` (`<rev>:<path>`) names, or none when the revision or the file
/// doesn't exist; any other git failure is `Error::Git` (`settings-from-main`, `verify-env`).
pub(crate) fn committed_text(root: &Path, source: &str) -> Result<Option<String>, Error> {
    match clm_git::git(root, &["rev-parse", "--verify", "--quiet", source]) {
        Ok(_) => Ok(Some(clm_git::git(root, &["show", source])?)),
        // `--quiet` reports a missing object with status 1 and nothing on stderr.
        Err(clm_git::Error::Failed { status, stderr, .. })
            if status.code() == Some(1) && stderr.trim().is_empty() =>
        {
            Ok(None)
        }
        Err(e) => Err(e.into()),
    }
}

/// The cap `land` applies when `[land]` sets none, which `config` doesn't offer without the
/// table.
const DEFAULT_CAP_GB: u64 = 15;

/// The repository a command runs in, with its `[verify]` settings.
pub(crate) struct Ctx<'a> {
    pub(crate) root: PathBuf,
    pub(crate) env: &'a Env,
    pub(crate) verify: clm_config::Verify,
    /// `[affected] all`.
    pub(crate) affected_all: Vec<String>,
    /// `[land] target_cap_gb`, the cap on the build place (`verify-build-cap`).
    pub(crate) cap_gb: u64,
    /// `max_builds`: the pass takes the slot after the build places (`mutation-pass`).
    pub(crate) max_builds: u64,
    pub(crate) host: clm_config::Host,
    /// `origin/main`.
    pub(crate) base: String,
    /// Where the CLM repository is in its git repository: `rev-parse --show-prefix`.
    pub(crate) prefix: String,
    /// Whether this call has checked a commit out into the verify tree, which it then clears
    /// before it returns (`verify-tree-cleared`).
    pub(crate) checked_out: Cell<bool>,
    /// The verify tree's lock, held from the call's first checkout until it clears the tree.
    pub(crate) tree_lock: RefCell<Option<std::fs::File>>,
}

impl<'a> Ctx<'a> {
    /// The context for `root`, or, in `Err(source)`, the name of the committed `clm.toml` whose
    /// settings hold no `[verify]`. Every setting, `host` included, is loaded after fetching
    /// `origin`, with the `clm.toml` committed at `origin/main` as the repository layer, under
    /// the clone's `clm.local.toml` and the user's layers as everywhere; the working tree's
    /// `clm.toml` is never loaded (`settings-from-main`).
    fn open(root: &Path, env: &'a Env) -> Result<Result<Self, String>, Error> {
        let root = std::fs::canonicalize(root)?;
        clm_git::fetch(&root)?;
        let base = format!("{}/main", clm_git::REMOTE);
        let prefix = clm_git::git(&root, &["rev-parse", "--show-prefix"])?;
        let source = format!("{base}:{prefix}clm.toml");
        let text = committed_text(&root, &source)?;
        let settings = clm_config::load_committed(&root, text.as_deref(), &source, env)?;
        let Some(verify) = settings.verify.value else {
            return Ok(Err(source));
        };
        Ok(Ok(Ctx {
            affected_all: settings.affected.value.map(|a| a.all).unwrap_or_default(),
            cap_gb: settings.land.value.map_or(DEFAULT_CAP_GB, |l| l.cap_gb()),
            max_builds: settings.max_builds.value,
            root,
            env,
            verify,
            host: settings.host.value,
            base,
            prefix,
            checked_out: Cell::new(false),
            tree_lock: RefCell::new(None),
        }))
    }

    pub(crate) fn git(&self, args: &[&str]) -> Result<String, Error> {
        Ok(clm_git::git(&self.root, args)?)
    }

    /// `commit` as `git rev-parse --short` abbreviates it.
    pub(crate) fn short(&self, commit: &str) -> Result<String, Error> {
        self.git(&["rev-parse", "--short", commit])
    }
}

/// What the caller gives the mutation pass (`mutation-pass`): the file it reads available
/// memory from, the length of a minute and of the grace between `SIGTERM` and `SIGKILL`, and a
/// stop request the pass watches while its command runs and that keeps the filing from
/// starting what it hasn't started yet (`files-unproven`).
pub struct Call<'a> {
    pub memory: &'a Path,
    pub minute: Duration,
    pub grace: Duration,
    pub stop: &'a AtomicBool,
}

impl<'a> Call<'a> {
    /// The call `clm` makes: `/proc/meminfo`, a minute of 60 seconds and a grace of 10, with
    /// `stop` as its stop request (`mutation-pass`).
    pub fn standard(stop: &'a AtomicBool) -> Self {
        Call {
            memory: Path::new("/proc/meminfo"),
            minute: Duration::from_secs(60),
            grace: Duration::from_secs(10),
            stop,
        }
    }
}

/// Runs one `clm verify` command in the repository at `root` with `call`. `env` is the
/// environment `config` reads. It calls `announce` with the outcome as soon as the result is
/// recorded, before the mutation pass a green run goes on to, so the caller can print the
/// outcome line first (`mutation-pass`); the pass's own notes come last in the returned
/// outcome's notes.
pub fn verify_with(
    root: &Path,
    request: &Request,
    env: &Env,
    call: &Call,
    announce: impl FnOnce(&Outcome),
) -> Result<Outcome, Error> {
    let ctx = match Ctx::open(root, env)? {
        Ok(ctx) => ctx,
        Err(source) => {
            let outcome = Outcome::new(format!("verify off: no [verify] in {source}"), 0);
            announce(&outcome);
            return Ok(outcome);
        }
    };
    let outcome = judge(&ctx, request, call, announce);
    // However the call ended, the tree keeps nothing past it, the pass's builds included; and
    // before any filing, which lands a branch (`verify-tree-cleared`).
    if ctx.checked_out.get() {
        tree::clear(&ctx);
    }
    // Filing doesn't use the tree, so another call may have it meanwhile.
    drop(ctx.tree_lock.take());
    let mut outcome = outcome?;
    // What the pass found reaches `main` as backlog items, whether or not a run was due
    // (`files-unproven`); only a clone in `local` mode holds the right to land them.
    let dry = matches!(request, Request::Due { dry_run: true, .. });
    let due = matches!(request, Request::Due { .. });
    if due
        && !dry
        && ctx.verify.mode == clm_config::VerifyMode::Local
        && ctx.verify.mutate.is_some()
    {
        outcome.notes.extend(file::file(&ctx, call.stop));
    }
    Ok(outcome)
}

/// The verification `request` asks for, announced, and the mutation pass a green run goes on
/// to.
fn judge(
    ctx: &Ctx,
    request: &Request,
    call: &Call,
    announce: impl FnOnce(&Outcome),
) -> Result<Outcome, Error> {
    let state = State::load(ctx)?;
    let mut outcome = match request {
        Request::Due { work_done, dry_run } => due::due(ctx, &state, *work_done, *dry_run),
        Request::Commit(sha) => run::commit(ctx, &state, sha),
        Request::Import => import::import_all(ctx, &state),
        Request::Bisect(sha) => bisect::bisect_red(ctx, &state, sha),
    }?;
    announce(&outcome);
    if let Some(end) = outcome.mutate {
        outcome.notes.extend(mutate::pass(ctx, &state, end, call));
    }
    Ok(outcome)
}
