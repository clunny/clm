//! `clm reconnect` (`land/offline-land`): a repository whose `origin` is newly set gets its
//! `main` and ledgers pushed.

use std::collections::HashSet;
use std::path::Path;

use clm_git::REMOTE;
use clm_ledger::Body;

use crate::{Env, Error, Output, Repo};

/// Pushes the local `main` when it fast-forwards `origin/main` (or `origin` has none), then
/// every local ledger branch. When `origin/main` moved, pushes only the ledgers and refuses,
/// naming each local land's branch to land again.
pub fn reconnect(root: &Path, env: &Env) -> Result<Output, Error> {
    let repo = Repo::open(root, env)?;
    if !repo.git(&["remote"])?.lines().any(|r| r == REMOTE) {
        return Err(Error::Refused(
            "no `origin`: add the remote, then reconnect".into(),
        ));
    }
    // Pruned, so a remote-tracking ref the old remote left stands for no branch of the new one.
    repo.fetch_retrying(&["fetch", "--quiet", "--prune", REMOTE])?;
    let main = repo
        .rev("refs/heads/main")?
        .ok_or_else(|| Error::Refused("no local `main` to push".into()))?;
    let theirs = repo.rev(&format!("refs/remotes/{REMOTE}/main"))?;
    let mut range = vec!["rev-list", main.as_str()];
    let not_theirs = theirs.as_ref().map(|t| format!("^{t}"));
    range.extend(not_theirs.as_deref());
    let unpushed: HashSet<String> = repo.git(&range)?.lines().map(str::to_string).collect();
    let fold = clm_ledger::fold(&repo.root)?;
    let lands: Vec<&str> = fold
        .records
        .iter()
        .filter_map(|r| match &r.body {
            Body::Landed { commit, branch, .. } if unpushed.contains(commit) => {
                Some(branch.as_str())
            }
            _ => None,
        })
        .collect();

    let fast_forward = match &theirs {
        None => true,
        Some(t) => clm_git::is_ancestor(&repo.root, t, &main)?,
    };
    if fast_forward {
        let lease = format!(
            "--force-with-lease=refs/heads/main:{}",
            theirs.as_deref().unwrap_or_default()
        );
        repo.git(&[
            "push",
            "--quiet",
            &lease,
            REMOTE,
            "refs/heads/main:refs/heads/main",
        ])?;
    }
    for ledger in repo
        .git(&[
            "for-each-ref",
            "--format=%(refname)",
            "refs/heads/clm/ledger/",
        ])?
        .lines()
    {
        repo.git(&["push", "--quiet", REMOTE, &format!("{ledger}:{ledger}")])?;
    }
    if fast_forward {
        return Ok(Output {
            lines: vec![format!("reconnected {} lands", lands.len())],
            warnings: Vec::new(),
        });
    }
    let mut why = vec![
        "origin's `main` has moved: run `clm rebase` on each branch landed locally, then land it again"
            .to_string(),
    ];
    why.extend(lands.iter().map(|b| format!("land again {b}")));
    Err(Error::Refused(why.join("\n")))
}
