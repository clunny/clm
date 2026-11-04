//! `clm land --check-only` (`land/check-only`, `land-check-command`): the land's check
//! command and `clm check` on the working tree as it stands, landing and recording nothing.

use std::path::Path;

use crate::exe::Exe;
use crate::{Env, Error, Output, Repo};

/// How `--check-only` reports a failing check.
const FAILS: &str = "fails";

/// Runs `[land] check`, when `clm.toml` sets it, and then `clm check` in `root`'s working
/// tree, as a land runs them on its merge, but with `CARGO_TARGET_DIR` left as the caller's
/// environment has it. Fetches, commits, records and pushes nothing. `branch` is refused:
/// it lands nothing, so it takes none.
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
    if let Some(land) = &repo.settings.land.value {
        repo.project_check(&repo.root, &land.check, None, &[], FAILS, None)?;
    }
    repo.clm_check(exe.path(), &repo.root, true, FAILS)?;
    Ok(Output {
        lines: vec!["checks pass".into()],
        warnings: Vec::new(),
    })
}
