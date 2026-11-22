//! A land finished after its push was made and its records were not (`land/unrecorded-land-finished`):
//! `clm land` of a branch whose head `main` already holds, merged by a commit no `landed`
//! record names, appends the records the land would have and merges nothing.

use std::path::Path;

use clm_ledger::{Body, Entry, Fold};

use crate::batch::{Found, judge};
use crate::lock::{self, Rank};
use crate::scratch::Scratch;
use crate::{Error, Output, Repo, Verdict};

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

/// What `origin/main` holding a head comes to (`land/already-on-main`).
pub(crate) enum OnMain {
    /// A commit of the first-parent chain merges the head as its second parent, and a
    /// `landed` record of the branch names it.
    Found(Found),
    /// The newest such commit, no `landed` record naming it
    /// (`land/unrecorded-land-finished`).
    Unrecorded(String),
    /// Any other head `main` holds: the oldest commit of the first-parent chain that contains
    /// it.
    Elsewhere(String),
}

impl OnMain {
    /// The refusal of a head `main` holds other than through a land of `branch`.
    pub(crate) fn elsewhere(branch: &str, commit: &str) -> Error {
        Error::Refused(format!("`{branch}` is already on main, in {commit}"))
    }
}

/// Whether `origin/main`, as last fetched, contains `head`, and how (`land/already-on-main`):
/// none when it doesn't.
pub(crate) fn on_main(
    repo: &Repo,
    fold: &Fold,
    branch: &str,
    head: &str,
) -> Result<Option<OnMain>, Error> {
    let contains = |commit: &str| clm_git::is_ancestor(&repo.root, head, commit);
    // With no `main` on `origin`, the land's refusal names `clm reconnect`.
    if repo.rev(repo.base())?.is_none() || !contains(repo.base())? {
        return Ok(None);
    }
    let chain = chain(repo)?;
    let merges: Vec<&Commit> = chain
        .iter()
        .filter(|c| c.parents.get(1).map(String::as_str) == Some(head))
        .collect();
    // Any land of the branch on one of them, when `main` merges the head more than once.
    for commit in &merges {
        for record in fold.lands.get(&commit.sha).into_iter().flatten() {
            if let Body::Landed {
                branch: landed,
                kind,
                review,
                light,
                ..
            } = &record.body
                && landed == branch
            {
                return Ok(Some(OnMain::Found(Found {
                    commit: commit.sha.clone(),
                    verdict: Verdict::of_record(*kind, *review, *light),
                })));
            }
        }
    }
    // The chain is newest first.
    if let Some(commit) = merges.iter().find(|c| !fold.lands.contains_key(&c.sha)) {
        return Ok(Some(OnMain::Unrecorded(commit.sha.clone())));
    }
    // Along the chain, from its newest commit, the ones that contain the head come first: the
    // last of them is the oldest.
    let (mut lo, mut hi) = (0, chain.len());
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        match contains(&chain[mid].sha)? {
            true => lo = mid,
            false => hi = mid,
        }
    }
    let oldest = chain
        .get(lo)
        .map_or_else(|| repo.base().to_string(), |c| c.sha.clone());
    Ok(Some(OnMain::Elsewhere(oldest)))
}

/// What finishing a land needs of the land that runs it.
pub(crate) struct Ctx<'a, 'r> {
    pub(crate) repo: &'a Repo<'r>,
    pub(crate) exe: &'a Path,
    pub(crate) places: &'a Path,
    pub(crate) check: Option<&'a str>,
    pub(crate) branch: &'a str,
    pub(crate) head: &'a str,
    pub(crate) on_origin: Option<String>,
    pub(crate) commit: &'a str,
    /// Whether the land already holds the land lock, and the push turn, which finishing then
    /// takes no second time.
    pub(crate) locked: bool,
    pub(crate) turned: bool,
}

/// Appends the records of the land that merged `ctx.head` as `ctx.commit` and of every other
/// land of its push that has none, under the land lock, and prints `landed`; refused with
/// `ledger/unrecorded-land` when no `checked` record covers the commit or the land was not one
/// `clm land` would have made.
pub(crate) fn finish(ctx: &Ctx) -> Result<Output, Error> {
    let repo = ctx.repo;
    // The land that pushed may still be appending its records, which it does inside its push
    // turn: wait for it, the locks taken in their order, and look again (`land/push-turn`). A
    // land that held the push turn alone let it go before it got here.
    let _lock = match ctx.locked {
        true => None,
        false => Some(lock::land(
            &ctx.places.join("land"),
            Rank::ORDINARY,
            "",
            repo.err,
        )?),
    };
    let _turn = match ctx.turned {
        true => None,
        false => Some(lock::push(ctx.places, repo.err)?),
    };
    repo.fetch()?;
    let fold = clm_ledger::fold(&repo.root)?;
    let mut out = Output::default();
    match on_main(repo, &fold, ctx.branch, ctx.head)? {
        Some(OnMain::Found(found)) => {
            return Ok(crate::land::found(
                repo,
                ctx.branch,
                ctx.head,
                ctx.on_origin.as_deref(),
                &found,
                &mut out,
            ));
        }
        Some(OnMain::Elsewhere(commit)) => return Err(OnMain::elsewhere(ctx.branch, &commit)),
        Some(OnMain::Unrecorded(_)) | None => {}
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
        let landed = judged
            .verdict
            .landed(commit.sha.clone(), branch, judged.fingerprint.clone())
            .with_code_key(Some(key.clone()))
            .with_changes(judged.changes.clone());
        entries.push(if signed {
            repo.approval.sign(landed, clm_approval::LAND)?.into()
        } else {
            landed.into()
        });
        if target {
            found = Some(Found {
                commit: commit.sha.clone(),
                verdict: judged.verdict,
            });
        }
    }
    let Some(found) = found else {
        return Err(refuse("it isn't a land `clm land` would have made"));
    };
    let appended = repo.append_landed(entries, ctx.branch)?;
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
        ctx.on_origin.as_deref(),
        &found,
        &mut out,
    ))
}
