//! `clm approve [<branch>]` (`approve`, `signed-approve`).

use std::path::Path;

use clm_ledger::{Body, person};

use crate::{Env, Error, Output, Repo, sign};

/// Appends the human's approval of `branch`'s (default: the current one's) diff against
/// `origin/main`, pinned to its fingerprint. The guard asks the human to confirm the command
/// (`approval-policy`); this doesn't ask again.
pub fn approve(root: &Path, branch: Option<&str>, env: &Env) -> Result<Output, Error> {
    // Before anything is fetched or appended (`approve-refuses-unconfirming-agent-harness`).
    let harness = clm_agent_harness::detect(env);
    if harness != clm_agent_harness::AgentHarness::None
        && !clm_agent_harness::abilities(harness).confirm
    {
        return Err(Error::Refused(format!(
            "`clm approve` can't run under {}: it can't make the human confirm the command \
             (`approval-policy`); the human approves by running the same `clm approve` command \
             in their own terminal, outside any agent",
            harness.name()
        )));
    }
    let root = std::fs::canonicalize(root)?;
    let settings = clm_config::load(Some(&root), env)?;
    let repo = Repo::new(&root, env, &settings);
    let branch = repo.branch_name(branch, "approved")?;
    repo.fetch()?;
    let head = repo.branch_head(&branch, false)?.head;
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
    let clears = repo.clears(&branch, &fingerprint)?;
    let body = Body::approved(fingerprint.clone(), branch.clone(), change).with_clears(clears);
    // Signed when the `main` fetched holds `docs/allowed_signers` (`signed-approve`).
    let appended = match repo.signed(&base)? {
        true => repo.append(repo.sign(body, sign::APPROVE)?)?,
        false => repo.append(body)?,
    };
    let mut warnings = Vec::new();
    if !appended.pushed {
        warnings.push(format!(
            "the approval isn't on origin yet, so no other clone counts it: git push origin \
             clm/ledger/{}",
            appended.record.writer
        ));
    }
    Ok(Output {
        lines: vec![format!("approved {fingerprint} {branch}")],
        warnings,
    })
}

impl Repo<'_> {
    /// The `id` of every `changes_requested` record the fold holds, after the fetch, for
    /// `branch` at `fingerprint` whose writer's person is the approver's, cleared before or
    /// not: the requests the approval clears (`ledger/request-cleared`).
    fn clears(&self, branch: &str, fingerprint: &str) -> Result<Vec<String>, Error> {
        let fold = clm_ledger::fold(&self.root)?;
        let approver = clm_ledger::writer(&self.root, self.env)?;
        let requests = fold.change_requests.get(branch).into_iter().flatten();
        Ok(requests
            .filter(|r| {
                matches!(&r.body, Body::ChangesRequested { fingerprint: f, .. } if f == fingerprint)
                    && person(&r.writer) == person(&approver)
            })
            .map(|r| r.id.clone())
            .collect())
    }
}
