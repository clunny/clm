//! Writing a ledger branch with git plumbing alone, never touching the working tree or the
//! index, and pushing it.

use std::ffi::OsStr;
use std::path::Path;
use std::time::Duration;

use crate::Error;

pub(crate) const FILE: &str = "records.jsonl";

/// The writer's branch ref.
pub(crate) fn ref_of(writer: &str) -> String {
    format!("refs/heads/clm/ledger/{writer}")
}

/// The commit `name` points at, or `None` when it doesn't exist.
pub(crate) fn tip(root: &Path, name: &str) -> Result<Option<String>, Error> {
    let commit = format!("{name}^{{commit}}");
    match clm_git::git(root, &["rev-parse", "--verify", "--quiet", &commit]) {
        Ok(sha) => Ok(Some(sha)),
        Err(clm_git::Error::Failed { status, .. }) if status.code() == Some(1) => Ok(None),
        Err(e) => Err(e.into()),
    }
}

/// `records.jsonl` at `rev`.
pub(crate) fn read(root: &Path, rev: &str) -> Result<Vec<u8>, Error> {
    let blob = format!("{rev}:{FILE}");
    Ok(clm_git::run(
        root,
        "git",
        &["cat-file", "blob", &blob],
        clm_git::Options::default(),
    )?)
}

/// Rewrites the branch's file as `edit` makes it from the current one (empty when the
/// branch doesn't exist yet), as a new commit on its tip, or the root commit of an orphan
/// branch. The ref moves only from the tip `edit` saw, so a session racing another in the
/// same clone reads the new tip and edits again. `edit` returning `None` leaves the branch as
/// it is. Returns whether a commit was made.
pub(crate) fn rewrite(
    root: &Path,
    name: &str,
    message: &str,
    mut edit: impl FnMut(&[u8]) -> Option<Vec<u8>>,
) -> Result<bool, Error> {
    const ATTEMPTS: usize = 50;
    for attempt in 1..=ATTEMPTS {
        let old = tip(root, name)?;
        let current = match &old {
            Some(sha) => read(root, sha)?,
            None => Vec::new(),
        };
        let Some(content) = edit(&current) else {
            return Ok(false);
        };
        let blob = stdin_git(root, &["hash-object", "-w", "--stdin"], &content)?;
        let tree = stdin_git(
            root,
            &["mktree"],
            format!("100644 blob {blob}\t{FILE}\n").as_bytes(),
        )?;
        let mut args = vec!["commit-tree", tree.as_str(), "-m", message];
        if let Some(parent) = &old {
            args.extend(["-p", parent.as_str()]);
        }
        // The writer names the commit, so a clone whose git has no `user.name` still writes.
        let writer = name
            .split_once("clm/ledger/")
            .map_or("", |(_, writer)| writer);
        let person = Some(crate::person(writer))
            .filter(|p| !p.is_empty())
            .unwrap_or("clm");
        let env = [
            ("GIT_AUTHOR_NAME", OsStr::new(person)),
            ("GIT_COMMITTER_NAME", OsStr::new(person)),
        ];
        let commit = run_git(root, &args, None, &env)?;
        // An empty old value means "must not exist yet", whatever the hash's length.
        let expected = old.clone().unwrap_or_default();
        match clm_git::git(root, &["update-ref", name, &commit, &expected]) {
            Ok(_) => return Ok(true),
            Err(e) if attempt == ATTEMPTS || !raced(root, name, &old, &e)? => {
                return Err(e.into());
            }
            Err(_) => std::thread::sleep(Duration::from_millis(5 * attempt as u64)),
        }
    }
    unreachable!("the last attempt returns")
}

/// Whether a failed `update-ref` lost to another session: the tip moved from `old`, or
/// another session holds the ref's lock. Any other failure is returned at once.
fn raced(root: &Path, name: &str, old: &Option<String>, e: &clm_git::Error) -> Result<bool, Error> {
    let locked = matches!(e, clm_git::Error::Failed { stderr, .. } if stderr.contains("lock"));
    Ok(locked || tip(root, name)? != *old)
}

fn stdin_git(root: &Path, args: &[&str], input: &[u8]) -> Result<String, Error> {
    run_git(root, args, Some(input), &[])
}

fn run_git(
    root: &Path,
    args: &[&str],
    stdin: Option<&[u8]>,
    env: &[(&str, &OsStr)],
) -> Result<String, Error> {
    let out = clm_git::run(root, "git", args, clm_git::Options { stdin, env })?;
    Ok(String::from_utf8_lossy(&out).trim().to_string())
}

/// Pushes the branch to `origin` without force, on every host, `none` included, whose
/// `origin` is the clone's own remote (`host/none-keeps-a-remote`). Nobody else writes
/// the branch, so a refusal is a network or lock failure, or a hook declining it, and it is
/// tried a few times. Returns whether it was pushed: a push still refused, for any reason,
/// leaves the commit local and unpushed (`ledger/append-refused-push`).
pub(crate) fn push(root: &Path, name: &str) -> Result<bool, Error> {
    let spec = format!("{name}:{name}");
    for attempt in 1..=3u64 {
        if clm_git::git(root, &["push", "--quiet", clm_git::REMOTE, &spec]).is_ok() {
            return Ok(true);
        }
        if attempt < 3 {
            std::thread::sleep(Duration::from_millis(200 * attempt));
        }
    }
    Ok(false)
}
