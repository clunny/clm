//! `clm land` and `clm reconnect`: landing on `main` from the machine that runs the command
//! (specs/land/SPEC.md, `land-locally`); `clm approve` is `clm-approval`'s. Every `git` call goes
//! through `clm-git`, the ledger through `clm-ledger`, branch deletion through `clm-host`; the
//! gate and `clm check` run as commands from a copy of this binary (`exe`).

mod cap;
mod check_log;
mod check_only;
mod derived;
mod exe;
mod land;
mod lock;
mod reconnect;
mod runner;
mod scratch;
mod time_report;
mod verify_stop;

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub use check_only::check_only;
pub use land::land;
pub use reconnect::reconnect;
pub use runner::{CheckEnv, CheckRun, Step, run_check};

pub use clm_approval::{Env, Output};

/// `clm approve [<branch>]`, which `clm-approval` holds (`land.approval/approve`).
pub fn approve(root: &Path, branch: Option<&str>, env: &Env) -> Result<Output, Error> {
    Ok(clm_approval::approve(root, branch, env)?)
}

#[derive(Debug)]
pub enum Error {
    /// Nothing was done, for the reason given: exit 1.
    Refused(String),
    /// `git`, the ledger or the settings failed: exit 2.
    Git(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Refused(why) | Error::Git(why) => f.write_str(why),
        }
    }
}

impl From<clm_approval::Error> for Error {
    fn from(e: clm_approval::Error) -> Self {
        match e {
            clm_approval::Error::Refused(why) => Error::Refused(why),
            clm_approval::Error::Git(why) => Error::Git(why),
        }
    }
}

impl From<clm_git::Failure> for Error {
    fn from(e: clm_git::Failure) -> Self {
        match e {
            clm_git::Failure::Refused(why) => Error::Refused(why),
            clm_git::Failure::Git(why) => Error::Git(why),
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
    fn from(e: clm_config::Error) -> Self {
        Error::Git(e.to_string())
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Git(e.to_string())
    }
}

/// The branches deleted from `origin` once landed.
const CLAIM_PREFIXES: [&str; 3] = ["impl/", "spec/", "propose/"];

const OFFLINE: &str = "offline: origin can't be reached; push and land when it can";

fn is_claim(branch: &str) -> bool {
    CLAIM_PREFIXES.iter().any(|p| branch.starts_with(p))
}

/// The repository a command runs in, with its settings.
pub(crate) struct Repo<'a> {
    root: PathBuf,
    env: &'a Env,
    settings: clm_config::Settings,
    /// `origin/main`, the ref every land is made on, whatever the host (`land/offline-land`).
    base: String,
    /// What `clm-approval` judges a branch's approval with, given the same root and settings.
    approval: clm_approval::Repo<'a>,
}

impl<'a> Repo<'a> {
    fn open(root: &Path, env: &'a Env) -> Result<Self, Error> {
        let root = std::fs::canonicalize(root)?;
        let settings = clm_config::load(Some(&root), env)?;
        let base = format!("{}/main", clm_git::REMOTE);
        let approval = clm_approval::Repo::new(&root, env, &settings);
        Ok(Repo {
            root,
            env,
            settings,
            base,
            approval,
        })
    }

    /// The ref `main` is landed on: `origin/main`.
    fn base(&self) -> &str {
        &self.base
    }

    fn git(&self, args: &[&str]) -> Result<String, Error> {
        Ok(clm_git::git(&self.root, args)?)
    }

    /// `git fetch origin`, tried again after a failure (`land/fetch-retry-in-layer`).
    fn fetch(&self) -> Result<(), Error> {
        self.fetch_with(&["fetch", clm_git::REMOTE])
    }

    /// The fetch `args` through the layer's retrying fetch: `Offline` is the offline refusal,
    /// and a fetch git refused keeps git's message (`land/offline-land`).
    pub(crate) fn fetch_with(&self, args: &[&str]) -> Result<(), Error> {
        clm_git::fetch_retrying(&self.root, args).map_err(|e| match e {
            clm_git::FetchError::Offline => Error::Refused(OFFLINE.into()),
            clm_git::FetchError::Git(e) => Error::Refused(e.to_string()),
        })
    }

    /// The commit `rev` names, or `None` when it names none.
    fn rev(&self, rev: &str) -> Result<Option<String>, Error> {
        Ok(self.approval.rev(rev)?)
    }

    /// The gate on `dir`'s HEAD against the base, run from `exe` unless `CLM_GATE` names it,
    /// refused with what it printed when it blocks.
    fn gate(&self, exe: &Path, dir: &Path, approved: bool, hint: &str) -> Result<Verdict, Error> {
        let mut args = vec!["--base", self.base()];
        if approved {
            args.push("--approved");
        }
        let out = self.command(exe, "CLM_GATE", "gate", &args, dir).output()?;
        let stdout = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
        if !out.status.success() {
            let mut why = format!("the gate blocks the land:\n{stdout}");
            if stdout.contains("gate/approval") {
                why.push('\n');
                why.push_str(hint);
            }
            return Err(Error::Refused(why));
        }
        let first = stdout.lines().next().unwrap_or_default();
        match first.split_whitespace().collect::<Vec<_>>()[..] {
            ["code"] => Ok(Verdict::Code),
            ["spec"] => Ok(Verdict::Spec { review: false }),
            ["spec", "review"] => Ok(Verdict::Spec { review: true }),
            _ => Err(Error::Refused(format!(
                "the gate printed `{first}`, not a kind `clm land` lands"
            ))),
        }
    }

    /// `clm check` in `dir`, or without its detectors, run from `exe` unless `CLM_CHECK`
    /// names it; refused as `` `clm check` <fails> `` when it fails.
    fn clm_check(&self, exe: &Path, dir: &Path, detectors: bool, fails: &str) -> Result<(), Error> {
        let args: &[&str] = if detectors { &[] } else { &["--no-detectors"] };
        let status = self
            .command(exe, "CLM_CHECK", "check", args, dir)
            .stdout(std::io::stderr())
            .status()?;
        if !status.success() {
            return Err(Error::Refused(format!("`clm check` {fails}")));
        }
        Ok(())
    }

    /// The repository's main worktree: the first one git lists.
    fn main_worktree(&self) -> Result<PathBuf, Error> {
        Ok(clm_git::worktrees(&self.root)?
            .into_iter()
            .next()
            .map_or_else(|| self.root.clone(), |w| w.path))
    }

    /// `<home>/.clm`, `<home>` the CLM root in the main worktree (`<main>/<prefix>`,
    /// `<prefix>` where the CLM root is in its git repository): where a land keeps everything
    /// it keeps (`land/land-tree`). Computed once per land, every place derived from it.
    fn places(&self) -> Result<PathBuf, Error> {
        runner::places_of(&self.root)
    }

    /// The slot of the build place that names `branch`, 0 outside one (`land/check-slot`),
    /// for `CLM_SLOT`; exit 2 with `place_slot`'s error when it fails.
    fn slot(&self, branch: &str) -> Result<u64, Error> {
        clm_orchestrator::place_slot(&self.root, branch).map_err(|e| match e {
            clm_orchestrator::Error::Refused(why) | clm_orchestrator::Error::Git(why) => {
                Error::Git(why)
            }
            clm_orchestrator::Error::Spec(_) => Error::Git("the specs don't load".into()),
        })
    }

    /// `[land] check` in `dir` through `sh -c`, outside CI, with `vars` set and `CLM_SLOT` set to `slot`, under the clone's check lock
    /// with a ticket of `rank`, when it has one; refused as `` `<check>` <fails> `` when it fails. With `log`
    /// (a land's), its output also goes to that log and a failure repeats its last lines and
    /// names the log, which a pass removes (`land/check-log`).
    fn project_check(
        &self,
        dir: &Path,
        check: &str,
        vars: &[(&str, &Path)],
        fails: &str,
        how: CheckHow,
    ) -> Result<Timing, Error> {
        let CheckHow { log, slot, rank } = how;
        // The runner `verify` shares: one check at a time on the clone, whatever the target
        // (`land/check-lock`, `land/shared-check-runner`), except a `--check-only` run in a
        // build place. `--check-only` passes no log, so its refusal repeats no tail and names
        // no step.
        let mut env = runner::CheckEnv {
            set: vars
                .iter()
                .filter(|(name, _)| *name != "CLM_SELECTION")
                .map(|(name, value)| (name.to_string(), value.as_os_str().to_owned()))
                .collect(),
            // `CLM_LAND_DIR` is set only by a land holding the land lock, through `vars`; any
            // other run has the caller's removed (`land/land-dir-env`).
            unset: vec!["CI".to_string(), "CLM_LAND_DIR".to_string()],
        };
        // Replacing the caller's own (`land/check-slot`).
        env.set.push(("CLM_SLOT".into(), slot.to_string().into()));
        let selection = vars
            .iter()
            .find(|(name, _)| *name == "CLM_SELECTION")
            .map(|(_, path)| *path);
        let run = runner::run_in(
            &self.places()?,
            check,
            dir,
            selection,
            &env,
            log,
            rank,
            |line| eprintln!("{line}"),
        )?;
        let Some(log) = log else {
            if !run.passed {
                return Err(Error::Refused(format!("`{check}` {fails}")));
            }
            return Ok(Timing::of(run));
        };
        if run.passed {
            let _ = std::fs::remove_file(log);
            return Ok(Timing::of(run));
        }
        let mut why = format!("last {} lines of `{check}`:\n", check_log::TAIL);
        for line in &run.tail {
            why.push_str(line);
            why.push('\n');
        }
        why.push_str(&format!("`{check}` {fails}"));
        if let Some(step) = &run.step {
            why.push_str(&format!(
                "; the step open when it ended is `{}`, lines {} to {} of its output",
                step.name, step.first, step.last
            ));
        }
        if run.logged {
            why.push_str(&format!("; its output is at {}", log.display()));
        }
        Err(Error::Refused(why))
    }

    /// What the branch can break, as `clm affected --base <base>` in `dir` prints it, or with
    /// `readers` (checked code) only the readers of what differs, run from `exe` unless
    /// `CLM_AFFECTED` names it, in a `land-sel-*` file for `[land] check`'s `CLM_SELECTION`
    /// (`land/selected-tests`). When it fails it says so on stderr: the check then runs every
    /// test.
    fn selection(&self, exe: &Path, dir: &Path, base: &str, readers: bool) -> Selected {
        let mut args = vec!["--base", base];
        if readers {
            args.push("--readers");
        }
        self.affected(exe, dir, &args, readers)
            .unwrap_or_else(|why| {
                eprintln!("clm affected failed: {why}; the check runs every test");
                Selected::Failed
            })
    }

    /// What changing both the branch's head and the fetched base can break, as
    /// `clm affected --overlap <head> <base>` prints it, with nothing printed as `Nothing`
    /// (`land/overlap-recheck`). When it fails it says so on stderr, and the land checks its
    /// own selection.
    fn overlap(&self, exe: &Path, dir: &Path, head: &str, base: &str) -> Selected {
        let args = ["--overlap", head, base];
        self.affected(exe, dir, &args, true).unwrap_or_else(|why| {
            eprintln!("clm affected failed: {why}; the land checks its own selection");
            Selected::Failed
        })
    }

    /// `clm affected` with `args` in `dir`, run from `exe` unless `CLM_AFFECTED` names it, its
    /// output in a `land-sel-*` file; with `empty_is_nothing`, an output with no line is
    /// `Nothing`. The reason when it fails.
    fn affected(
        &self,
        exe: &Path,
        dir: &Path,
        args: &[&str],
        empty_is_nothing: bool,
    ) -> Result<Selected, String> {
        let out = self
            .command(exe, "CLM_AFFECTED", "affected", args, dir)
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let first = stderr.lines().map(str::trim).find(|l| !l.is_empty());
            return Err(first.map_or_else(|| out.status.to_string(), str::to_string));
        }
        if empty_is_nothing && out.stdout.iter().all(u8::is_ascii_whitespace) {
            return Ok(Selected::Nothing);
        }
        let parent = temp_parent(self.env).map_err(|e| e.to_string())?;
        let dir = tempfile::Builder::new()
            .prefix("land-sel-")
            .tempdir_in(parent)
            .map_err(|e| e.to_string())?;
        let path = dir.path().join("selection");
        std::fs::write(&path, &out.stdout).map_err(|e| e.to_string())?;
        Ok(Selected::Tests(Selection { _dir: dir, path }))
    }

    /// Why the change selects `path`, as `clm affected --why <path> --base <base>` prints it in
    /// `dir`, run as `selection` runs `clm affected`; the reason when it fails
    /// (`land/check-time-report`).
    fn why(&self, exe: &Path, dir: &Path, base: &str, path: &str) -> Result<Vec<String>, String> {
        let out = self
            .command(
                exe,
                "CLM_AFFECTED",
                "affected",
                &["--why", path, "--base", base],
                dir,
            )
            .stderr(Stdio::piped())
            .output()
            .map_err(|e| e.to_string())?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            let first = stderr.lines().map(str::trim).find(|l| !l.is_empty());
            return Err(first.map_or_else(|| out.status.to_string(), str::to_string));
        }
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect())
    }

    /// The command `var` names, split on whitespace, else `exe`'s `sub`; with `args`, run in
    /// `dir`.
    fn command(&self, exe: &Path, var: &str, sub: &str, args: &[&str], dir: &Path) -> Command {
        let mut words: Vec<String> = match (self.env)(var).filter(|c| !c.trim().is_empty()) {
            Some(c) => c.split_whitespace().map(str::to_string).collect(),
            None => vec![exe.display().to_string(), sub.to_string()],
        };
        words.extend(args.iter().map(|a| a.to_string()));
        let mut command = Command::new(&words[0]);
        command
            .args(&words[1..])
            .current_dir(dir)
            .stdin(Stdio::null())
            .stderr(Stdio::inherit());
        command
    }

    fn append(&self, entry: impl Into<clm_ledger::Entry>) -> Result<clm_ledger::Appended, Error> {
        Ok(clm_ledger::append(&self.root, entry, self.env)?)
    }

    fn append_all(
        &self,
        entries: Vec<clm_ledger::Entry>,
    ) -> Result<Vec<clm_ledger::Appended>, Error> {
        Ok(clm_ledger::append_all(&self.root, entries, self.env)?)
    }
}

/// The times a passing `[land] check` took, for the `landed` record (`land/step-times`).
pub(crate) struct Timing {
    pub(crate) seconds: u64,
    pub(crate) steps: Vec<clm_ledger::TimedStep>,
}

impl Timing {
    fn of(run: runner::CheckRun) -> Self {
        Timing {
            seconds: run.seconds,
            steps: run.steps,
        }
    }
}

/// How a `[land] check` is run beyond its command: where its output is logged, the slot it
/// gets (`land/check-slot`) and the rank it takes the check lock in, if any (`land/check-lock`, `land/lock-queue`).
pub(crate) struct CheckHow<'a> {
    pub(crate) log: Option<&'a Path>,
    pub(crate) slot: u64,
    pub(crate) rank: Option<lock::Rank>,
}

/// The directory `clm-land`'s temporary directories go in: `TMPDIR` through the injected
/// lookup, so each test fixture gives its own, else the system's, made absolute, since
/// `tempfile` resolves a relative one against the process's directory and `git` against
/// the repository.
pub(crate) fn temp_parent(env: &Env) -> Result<PathBuf, Error> {
    let parent = env("TMPDIR")
        .filter(|t| !t.is_empty())
        .map_or_else(std::env::temp_dir, PathBuf::from);
    Ok(std::path::absolute(parent)?)
}

/// The file `clm affected`'s output is in, removed with its directory when dropped.
pub(crate) struct Selection {
    _dir: tempfile::TempDir,
    pub(crate) path: PathBuf,
}

impl Selection {
    /// The number of `file` lines the selection holds, none when it holds an `all` line, which
    /// runs every test (`land/overlap-recheck`).
    pub(crate) fn files(&self) -> Option<u64> {
        let text = std::fs::read_to_string(&self.path).ok()?;
        let mut files = 0;
        for line in text.lines() {
            match line.split_whitespace().next() {
                Some("all") => return None,
                Some("file") => files += 1,
                _ => {}
            }
        }
        Some(files)
    }
}

/// What `clm affected` made of a land.
pub(crate) enum Selected {
    /// Its readers of checked code were none: no check runs.
    Nothing,
    Tests(Selection),
    /// It failed; the check runs without a selection.
    Failed,
}

/// What the gate says a branch is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Verdict {
    Code,
    /// A spec branch, and whether it needs review (`gate/needs-review`).
    Spec {
        review: bool,
    },
}
