//! `clm approve [<branch>]` (`approve`, `signed-approve`).

use std::path::Path;

use clm_ledger::Body;

use crate::{Env, Error, Output, Repo, sign};

/// Appends the human's approval of `branch`'s (default: the current one's) diff against
/// `origin/main`, pinned to its fingerprint. The guard asks the human to confirm the command
/// (`approve-in-session`); this doesn't ask again.
pub fn approve(root: &Path, branch: Option<&str>, env: &Env) -> Result<Output, Error> {
    let root = std::fs::canonicalize(root)?;
    let settings = clm_config::load(Some(&root), env)?;
    let repo = Repo::new(&root, env, &settings);
    let branch = repo.branch_name(branch, "approved")?;
    repo.fetch()?;
    let (head, _) = repo.head(&branch, false)?;
    // One commit of the `main` fetched, for the fingerprint and for whether to sign.
    let base = repo
        .rev(repo.base())?
        .ok_or_else(|| Error::Refused(format!("no `{}` to approve against", repo.base())))?;
    let fingerprint = repo.fingerprint(&base, &head)?;
    if fingerprint == "empty" {
        return Err(Error::Refused(format!(
            "`{branch}` has no diff against {}: nothing to approve",
            repo.base()
        )));
    }
    let change = repo.changes(&head)?.into_iter().next();
    let body = Body::Approved {
        fingerprint: fingerprint.clone(),
        branch: branch.clone(),
        change,
    };
    // Signed when the `main` fetched holds `docs/allowed_signers` (`signed-approve`).
    match repo.signed(&base)? {
        true => repo.append(repo.sign(body, sign::APPROVE)?)?,
        false => repo.append(body)?,
    };
    Ok(Output {
        lines: vec![format!("approved {fingerprint} {branch}")],
        warnings: Vec::new(),
    })
}
