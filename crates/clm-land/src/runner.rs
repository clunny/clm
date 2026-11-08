//! The runner `clm land` and `clm land --check-only` run `[land] check` through, offered to
//! `verify` for `[verify] check` (`land/shared-check-runner`): one check at a time on the
//! clone, its output copied to stderr and a log, its last lines and open step returned.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::Error;
use crate::check_log;
pub use crate::check_log::Step;
use crate::lock::Rank;

/// The environment a check runs in, beyond the caller's own: what to set and what to remove.
/// `CLM_SELECTION` is never inherited: it is set from the selection given, else removed.
#[derive(Debug, Default, Clone)]
pub struct CheckEnv {
    pub set: Vec<(String, OsString)>,
    pub unset: Vec<String>,
}

/// What a run of the check came to.
#[derive(Debug, Clone)]
pub struct CheckRun {
    pub passed: bool,
    /// The last 40 lines of its output.
    pub tail: Vec<String>,
    /// Whether the log was created and written whole.
    pub logged: bool,
    /// The step open as its output ended, if any.
    pub step: Option<Step>,
    /// The command's wall-clock seconds, rounded down (`land/step-times`).
    pub seconds: u64,
    /// The steps it marked, in order, with their seconds and selecting paths.
    pub steps: Vec<clm_ledger::TimedStep>,
}

/// Runs `command` through `sh -c` in `dir`, under the check lock of the clone `dir` belongs
/// to (`.clm/check.lock` in the CLM root of its main worktree), printing a waiting line once
/// when another check holds it. Its stdout and stderr go, in the order written, to stderr and
/// to `log` when given (created with its directories and truncated), as `check-log` says. The
/// log is left for the caller to keep or remove. `dir` is the CLM root of a checkout (a land
/// tree included), not a subdirectory of it, since the lock's place follows from it.
pub fn run_check(
    command: &str,
    dir: &Path,
    selection: Option<&Path>,
    env: &CheckEnv,
    log: Option<&Path>,
) -> Result<CheckRun, Error> {
    let places = places_of(dir)?;
    run_in(
        &places,
        command,
        dir,
        selection,
        env,
        log,
        Some(Rank::ORDINARY),
        |l| eprintln!("{l}"),
    )
}

/// `<main worktree>/<prefix>/.clm` for the repository `root` is in, `<prefix>` where `root`
/// is in its git repository: where a land keeps everything it keeps (`land/land-tree`).
pub(crate) fn places_of(root: &Path) -> Result<PathBuf, Error> {
    let prefix = clm_git::git(root, &["rev-parse", "--show-prefix"])?;
    let main = clm_git::worktrees(root)?
        .into_iter()
        .next()
        .map_or_else(|| root.to_path_buf(), |w| w.path);
    Ok(main.join(prefix).join(".clm"))
}

/// `run_check` with the clone's `places` known and the waiting line passed to `say`, taking
/// the check lock, with a ticket of `rank`, unless it is `None`: a `--check-only` run in a
/// build place takes none (`land/check-lock`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_in(
    places: &Path,
    command: &str,
    dir: &Path,
    selection: Option<&Path>,
    env: &CheckEnv,
    log: Option<&Path>,
    rank: Option<Rank>,
    say: impl FnMut(&str),
) -> Result<CheckRun, Error> {
    // One check at a time on the clone, whatever the target, held until the command ends
    // (`land/check-lock`).
    let _lock = rank
        .map(|rank| crate::lock::check(places, rank, say))
        .transpose()?;
    let logged = check_log::run(shell(command, dir, selection, env), log)?;
    Ok(CheckRun {
        passed: logged.status.success(),
        tail: logged.tail,
        logged: logged.logged,
        step: logged.step,
        seconds: logged.seconds,
        steps: logged.steps,
    })
}

/// `sh -c <check>` in `dir`, with `env` applied and `CLM_SELECTION` naming `selection` or
/// removed, so only a selection this run was given reaches the check.
fn shell(check: &str, dir: &Path, selection: Option<&Path>, env: &CheckEnv) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(check)
        .current_dir(dir)
        .stdin(Stdio::null());
    for name in &env.unset {
        command.env_remove(name);
    }
    command.envs(env.set.iter().map(|(k, v)| (k, v)));
    match selection {
        Some(path) => command.env("CLM_SELECTION", path),
        None => command.env_remove("CLM_SELECTION"),
    };
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsStr;
    use std::time::{Duration, Instant};

    #[test]
    fn check_runs_outside_ci() {
        // req:land/land@f4bd0a7c
        let env = CheckEnv {
            set: vec![("CARGO_TARGET_DIR".into(), "/r/.clm/land".into())],
            unset: vec!["CI".into()],
        };
        let command = shell("true", Path::new("/w"), None, &env);
        let envs: Vec<_> = command.get_envs().collect();
        assert!(envs.contains(&(OsStr::new("CI"), None)), "{envs:?}");
        // req:land/selected-tests@912a8d72
        let selection = (OsStr::new("CLM_SELECTION"), None);
        assert!(envs.contains(&selection), "{envs:?}");
        assert!(
            envs.contains(&(
                OsStr::new("CARGO_TARGET_DIR"),
                Some(OsStr::new("/r/.clm/land"))
            )),
            "{envs:?}"
        );
        let command = shell("true", Path::new("/w"), Some(Path::new("/s")), &env);
        let envs: Vec<_> = command.get_envs().collect();
        assert!(
            envs.contains(&(OsStr::new("CLM_SELECTION"), Some(OsStr::new("/s")))),
            "{envs:?}"
        );
    }

    #[test]
    fn the_waiting_line_is_said_once_and_the_command_runs_after_the_lock() {
        // req:land/shared-check-runner@05214b4c
        let dir = tempfile::tempdir().unwrap();
        let places = dir.path().join(".clm");
        std::fs::create_dir_all(&places).unwrap();
        let lock = std::fs::File::create(places.join("check.lock")).unwrap();
        lock.lock().unwrap();
        let releaser = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(500));
            drop(lock);
        });
        let log = dir.path().join("log");
        let mut said = Vec::new();
        let start = Instant::now();
        let run = run_in(
            &places,
            "echo ok",
            dir.path(),
            None,
            &CheckEnv::default(),
            Some(&log),
            Some(Rank::ORDINARY),
            |l| said.push(l.to_string()),
        )
        .unwrap();
        assert!(start.elapsed() >= Duration::from_millis(400));
        releaser.join().unwrap();
        assert_eq!(said.len(), 1, "{said:?}");
        assert!(
            said[0].starts_with("waiting for another check on ") && said[0].ends_with(": 1 ahead"),
            "{said:?}"
        );
        assert!(run.passed);
        assert_eq!(std::fs::read_to_string(&log).unwrap(), "ok\n");
    }
}
