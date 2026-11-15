//! A land finished after its push was made and its records were not (`land/unrecorded-land-finished`):
//! `clm land` of a branch whose head `main` already holds, merged by a commit no `landed`
//! record names, appends the records the land would have and merges nothing.

use std::path::Path;

use clm_ledger::{Body, Entry, Fold};

use crate::batch::{Found, judge};
use crate::lock::{self, Rank};
use crate::scratch::Scratch;
use crate::{Error, Output, Repo};

/// A commit of `origin/main`'s first-parent chain.
struct Commit {
    sha: String,
    parents: Vec<String>,
    /// The code key its `Checked:` line names, empty when it has none.
    checked: String,
    subject: String,
}

fn chain(repo: &Repo) -> Result<Vec<Commit>, Error> {
    let log = repo.git(&[
        "log",
        "--first-parent",
        "--format=%H%x1f%P%x1f%(trailers:key=Checked,valueonly,separator=%x1e)%x1f%s",
        repo.base(),
    ])?;
    Ok(log
        .lines()
        .filter_map(|line| {
            let mut fields = line.splitn(4, '\x1f');
            Some(Commit {
                sha: fields.next()?.to_string(),
                parents: fields
                    .next()?
                    .split_whitespace()
                    .map(str::to_string)
                    .collect(),
                checked: fields.next()?.trim().to_string(),
                subject: fields.next()?.to_string(),
            })
        })
        .collect())
}

/// The commit of `origin/main`'s first-parent chain that merges `head` as its second parent
/// and that no `landed` record names; none for a head that isn't on `main`, one on the chain
/// itself, or one whose merge a record names, which land as before.
pub(crate) fn unrecorded_commit(
    repo: &Repo,
    fold: &Fold,
    head: &str,
) -> Result<Option<String>, Error> {
    if !clm_git::is_ancestor(&repo.root, head, repo.base()).unwrap_or(false) {
        return Ok(None);
    }
    let chain = chain(repo)?;
    if chain.iter().any(|c| c.sha == head) {
        return Ok(None);
    }
    let merge = chain
        .iter()
        .find(|c| c.parents.get(1).map(String::as_str) == Some(head));
    Ok(merge
        .filter(|c| !fold.lands.contains_key(&c.sha))
        .map(|c| c.sha.clone()))
}

/// What finishing a land needs of the land that runs it.
pub(crate) struct Ctx<'a, 'r> {
    pub(crate) repo: &'a Repo<'r>,
    pub(crate) exe: &'a Path,
    pub(crate) places: &'a Path,
    pub(crate) check: Option<&'a str>,
    pub(crate) branch: &'a str,
    pub(crate) head: &'a str,
    pub(crate) on_origin: bool,
    pub(crate) commit: &'a str,
}

/// Appends the records of the land that merged `ctx.head` as `ctx.commit` and of every other
/// land of its push that has none, under the land lock, and prints `landed`; refused with
/// `ledger/unrecorded-land` when no `checked` record covers the commit or the land was not one
/// `clm land` would have made.
pub(crate) fn finish(ctx: &Ctx) -> Result<Output, Error> {
    let repo = ctx.repo;
    // The land that pushed may still be appending its records: wait for it, and look again.
    let _lock = lock::land(&ctx.places.join("land"), Rank::ORDINARY, "")?;
    repo.fetch()?;
    let fold = clm_ledger::fold(&repo.root)?;
    let mut out = Output::default();
    if let Some(found) = crate::batch::landed_record(repo, &fold, ctx.branch, ctx.head)? {
        return Ok(crate::land::finish_found(
            repo,
            ctx.branch,
            ctx.head,
            ctx.on_origin,
            &found,
            &mut out,
        ));
    }
    let refuse = |why: &str| {
        Error::Refused(format!(
            "{}: ledger/unrecorded-land: {} is on main with no `landed` record, and {why}",
            ctx.commit, ctx.commit
        ))
    };
    let commits = chain(repo)?;
    let Some(at) = commits.iter().position(|c| c.sha == ctx.commit) else {
        return Err(refuse("it isn't on main's first-parent chain"));
    };
    // The run of commits holding the same `Checked:` line is one push's lands.
    let line = commits[at].checked.clone();
    let (mut lo, mut hi) = (at, at);
    if !line.is_empty() {
        while lo > 0 && commits[lo - 1].checked == line {
            lo -= 1;
        }
        while hi + 1 < commits.len() && commits[hi + 1].checked == line {
            hi += 1;
        }
    }
    let command = match ctx.check {
        Some(check) => crate::checked_command(check, &repo.env_at(ctx.commit)?),
        None => "clm check".to_string(),
    };
    // The key the check ran on: the code key of the run's last commit when the line names it,
    // else the commit's own.
    let own = clm_ledger::code_key(&repo.root, ctx.commit)?;
    let key = if !line.is_empty() && clm_ledger::code_key(&repo.root, &commits[lo].sha)? == line {
        line
    } else {
        own
    };
    if !crate::land::checked_with(&fold, &key, &command) {
        return Err(refuse("no `checked` record covers it"));
    }
    let signed = repo.approval.signed(&commits[at].parents[0])?;
    if signed {
        repo.approval.require_key()?;
    }
    let dir = Scratch::temp(repo, ctx.commit)?;
    let mut entries: Vec<Entry> = Vec::new();
    let mut found = None;
    for (i, commit) in commits.iter().enumerate().take(hi + 1).skip(lo) {
        let target = i == at;
        if !target && (fold.lands.contains_key(&commit.sha) || commit.parents.len() != 2) {
            continue;
        }
        let (branch, head) = if target {
            (ctx.branch.to_string(), ctx.head.to_string())
        } else {
            let Some(branch) = commit
                .subject
                .strip_prefix("Merge ")
                .and_then(|s| s.strip_suffix(" into main"))
            else {
                continue;
            };
            (branch.to_string(), commit.parents[1].clone())
        };
        let judged = commit
            .parents
            .first()
            .ok_or_else(|| Error::Refused("a root commit".into()))
            .and_then(|first| {
                dir.checkout(&commit.sha)?;
                judge(
                    repo, ctx.exe, &dir, &fold, &branch, first, &head, signed, false,
                )
            });
        let judged = match judged {
            Ok(judged) => judged,
            Err(e) if target => {
                return Err(refuse(&format!(
                    "it isn't a land `clm land` would have made: {e}"
                )));
            }
            Err(e) => {
                out.warnings
                    .push(format!("left unrecorded: {} ({branch}): {e}", commit.sha));
                continue;
            }
        };
        let options = clm_effort::Options {
            branch: Some(branch.clone()),
            ..Default::default()
        };
        if let Ok(read) = clm_effort::unrecorded(&repo.root, &options, repo.env) {
            entries.extend(read.bodies.into_iter().map(Into::into));
        }
        let landed = Body::Landed {
            commit: commit.sha.clone(),
            branch,
            kind: judged.kind,
            review: judged.review,
            fingerprint: judged.fingerprint.clone(),
            code_key: Some(key.clone()),
            changes: judged.changes.clone(),
            check_seconds: None,
            steps: Vec::new(),
            recheck: None,
            recheck_files: None,
        };
        entries.push(if signed {
            repo.approval.sign(landed, clm_approval::LAND)?.into()
        } else {
            landed.into()
        });
        if target {
            found = Some(Found {
                commit: commit.sha.clone(),
                kind: judged.kind,
                review: judged.review,
                changes: judged.changes,
            });
        }
    }
    let Some(found) = found else {
        return Err(refuse("it isn't a land `clm land` would have made"));
    };
    let appended = repo.append_all(entries)?;
    if let Some(last) = appended.last()
        && !last.pushed
    {
        out.warnings.push(format!(
            "the landed record isn't on origin yet: git push origin clm/ledger/{}",
            last.record.writer
        ));
    }
    drop(dir);
    Ok(crate::land::finish_found(
        repo,
        ctx.branch,
        ctx.head,
        ctx.on_origin,
        &found,
        &mut out,
    ))
}
