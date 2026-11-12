//! `clm land --check-only` (`land/check-only`, `land-check-command`): the land's check
//! command and `clm check` on the working tree as it stands, landing nothing; a clean committed
//! `HEAD` that contains `origin/main` is recorded as checked (`worktree-check-counts`).

use std::path::Path;

use clm_ledger::Body;

use crate::exe::Exe;
use crate::lock::Rank;
use crate::{CheckHow, Env, Error, Output, Repo};

/// How `--check-only` reports a failing check.
const FAILS: &str = "fails";

/// Runs `[land] check`, when `clm.toml` sets it, and then `clm check` in `root`'s working
/// tree, as a land runs them on its merge, but with `CLM_LAND_DIR` removed and every
/// test selected. Fetches, commits and pushes nothing; the one
/// thing it appends is the `checked` record of a clean committed `HEAD` that contains
/// `origin/main`. `branch` is refused: it lands nothing, so it takes none.
pub fn check_only(root: &Path, branch: Option<&str>, env: &Env) -> Result<Output, Error> {
    if let Some(branch) = branch {
        return Err(Error::Refused(format!(
            "`--check-only` and a branch (`{branch}`) don't go together: it checks the \
             working tree and lands nothing"
        )));
    }
    // A settings error refuses, exit 1, as the checks' own failures do; any other error
    // opening the repository is a failure, exit 2.
    if let Ok(root) = std::fs::canonicalize(root) {
        clm_config::load(Some(&root), env).map_err(|e| Error::Refused(e.to_string()))?;
    }
    let repo = Repo::open(root, env)?;
    let exe = Exe::new(&repo)?;
    // The working tree's `[env]` (`land/check-env`).
    let env_vars = match &repo.settings.land.value {
        Some(_) => repo.env_of(std::fs::read(repo.root.join("clm.toml")).ok().as_deref())?,
        None => Vec::new(),
    };
    let before = Tree::read(&repo)?;
    if let Some(land) = &repo.settings.land.value {
        // The slot of the place that names the checked-out branch, 0 when `HEAD` is detached
        // (`land/check-slot`); a run in a build place, slot not 0, takes no check lock
        // (`land/check-lock`).
        let slot = match clm_git::current_branch(&repo.root)? {
            Some(branch) => repo.slot(&branch)?,
            None => 0,
        };
        repo.project_check(
            &repo.root,
            &land.check,
            &[],
            FAILS,
            CheckHow {
                env: &env_vars,
                log: None,
                slot,
                rank: (slot == 0).then_some(Rank::ORDINARY),
            },
        )?;
    }
    repo.clm_check(exe.path(), &repo.root, true, FAILS)?;
    let after = Tree::read(&repo)?;
    let mut out = Output::default();
    match not_recorded(&repo, &before, &after)? {
        Some(why) => out.lines.push(format!("not recorded: {why}")),
        None => match record(&repo, &after.head, &env_vars) {
            Ok(()) => out
                .lines
                .push(format!("recorded {} as checked", after.head)),
            // A land then runs its own check, so a failed append only warns.
            Err(e) => out.warnings.push(format!("checked not recorded: {e}")),
        },
    }
    out.lines.push("checks pass".into());
    Ok(out)
}

/// Appends the `checked` record for `head`'s code key with the command a land's names.
fn record(repo: &Repo, head: &str, env: &[(String, String)]) -> Result<(), Error> {
    let command = match &repo.settings.land.value {
        Some(land) => crate::checked_command(&land.check, env),
        None => "clm check".into(),
    };
    let code_key = clm_ledger::code_key(&repo.root, head)?;
    repo.append(Body::Checked {
        code_key,
        command,
        basis: None,
    })?;
    Ok(())
}

/// What the working tree was at one moment: its `HEAD` and whether it was clean.
struct Tree {
    head: String,
    clean: bool,
}

impl Tree {
    /// `HEAD`'s commit, and whether `git status --porcelain` prints nothing: no tracked file
    /// differs from `HEAD`, staged or not, and there is no untracked file git doesn't ignore.
    fn read(repo: &Repo) -> Result<Self, Error> {
        Ok(Tree {
            head: repo.git(&["rev-parse", "HEAD"])?,
            clean: repo.git(&["status", "--porcelain"])?.is_empty(),
        })
    }
}

/// Why the check doesn't count for a land, the first that holds, or none when it does: the
/// tree was clean and `HEAD` the same commit at both ends, and the local `origin/main` is an
/// ancestor of it.
fn not_recorded(repo: &Repo, before: &Tree, after: &Tree) -> Result<Option<&'static str>, Error> {
    if !before.clean || !after.clean {
        return Ok(Some("the working tree has uncommitted changes"));
    }
    if before.head != after.head {
        return Ok(Some("HEAD moved during the check"));
    }
    let main = format!("refs/remotes/{}/main", clm_git::REMOTE);
    let contains =
        repo.rev(&main)?.is_some() && clm_git::is_ancestor(&repo.root, &main, &after.head)?;
    Ok((!contains).then_some("HEAD doesn't contain origin/main"))
}
