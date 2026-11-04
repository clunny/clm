//! `retry-index-lock`: several sessions share one clone, and Claude Code, hooks and the panel
//! run `git` in it between a session's commands, so a `git` command `clm sync` runs can find
//! the worktree's `index.lock` held for a moment. Every `git` command this crate runs goes
//! through `git` or `run` here, which run it again when it fails naming `index.lock`.

use std::path::Path;
use std::thread::sleep;
use std::time::Duration;

use clm_git::{Error, Options};

/// The waits before each retry: under 4 seconds in all, fixed.
const WAITS: [Duration; 4] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
];

/// `clm-git`'s `git`, retried on a held `index.lock`.
pub fn git(root: &Path, args: &[&str]) -> Result<String, Error> {
    retried(|| clm_git::git(root, args))
}

/// `clm-git`'s `run` of `git`, retried on a held `index.lock`.
pub fn run(root: &Path, args: &[&str], options: Options) -> Result<Vec<u8>, Error> {
    retried(|| {
        let options = Options {
            stdin: options.stdin,
            env: options.env,
        };
        clm_git::run(root, "git", args, options)
    })
}

/// `command`'s result, after running it again up to four more times while it fails with
/// stderr naming `index.lock`; the last failure unchanged when every run fails. Keyed on
/// the text, not the exit code: `git rebase` exits 1 on the lock, `git add` 128.
fn retried<T>(mut command: impl FnMut() -> Result<T, Error>) -> Result<T, Error> {
    let mut waits = WAITS.iter();
    loop {
        let result = command();
        let locked =
            matches!(&result, Err(Error::Failed { stderr, .. }) if stderr.contains("index.lock"));
        match waits.next().filter(|_| locked) {
            Some(wait) => sleep(*wait),
            None => return result,
        }
    }
}
