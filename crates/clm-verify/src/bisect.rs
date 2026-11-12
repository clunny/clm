//! `verify-bisect`: the land that broke a red commit, and the components it touched.

use std::cmp::Reverse;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clm_ledger::{Body, Recheck};

use crate::chain::State;
use crate::{Ctx, Error, Outcome, run};

/// What the search over the lands up to a red one found.
pub(crate) struct Found {
    /// The first red land, when the search could tell.
    pub(crate) culprit: Option<String>,
    /// With no culprit, the lands that may hold it: those from the last green probe to the first
    /// red land.
    pub(crate) may_hold: Option<Vec<String>>,
    /// What to say on stderr after the outcome line when the culprit is unknown.
    pub(crate) note: Option<String>,
}

/// The land in `lo..hi` not set aside that is nearest the middle, the later of two equally near.
fn nearest(lo: usize, hi: usize, aside: &BTreeSet<usize>) -> Option<usize> {
    let target = lo + (hi - lo) / 2;
    (lo..hi)
        .filter(|p| !aside.contains(p))
        .min_by_key(|p| (p.abs_diff(target), Reverse(*p)))
}

/// The first land that fails `[verify] check` among those after the green commit `green` up to
/// `end`, which is known to fail: a binary search, each probe run as the verification is, under
/// the check lock afresh. A land whose check could not run (`run::Probe::untestable`) is set
/// aside, and another of the range left is probed; when only such lands lie between the last
/// green probe and the first red land, the culprit is unknown.
pub(crate) fn first_red(
    ctx: &Ctx,
    state: &State,
    green: Option<usize>,
    end: usize,
) -> Result<Found, Error> {
    let (mut lo, mut hi) = (green.map_or(0, |g| g + 1), end);
    let green = green.map(|g| state.chain[g].as_str());
    let mut aside = BTreeSet::new();
    while let Some(mid) = nearest(lo, hi, &aside) {
        let probe = run::check(ctx, &state.chain[mid], &state.chain[end], green, None)?;
        if probe.untestable {
            aside.insert(mid);
        } else if probe.passed {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == hi {
        return Ok(Found {
            culprit: Some(state.chain[hi].clone()),
            may_hold: None,
            note: None,
        });
    }
    let short = |p: usize| ctx.short(&state.chain[p]);
    let note = format!(
        "culprit unknown: [verify] check could not run on {} lands, {} to {}, before the first \
         red one, {}",
        hi - lo,
        short(lo)?,
        short(hi - 1)?,
        short(hi)?
    );
    Ok(Found {
        culprit: None,
        may_hold: Some(state.chain[lo..=hi].to_vec()),
        note: Some(note),
    })
}

/// The components whose `paths` hold a file the culprit's first-parent diff changes, or, with
/// no culprit, a file any of `lands`' diffs change; the root component when none does. The
/// components are those of `commit`'s specs.
pub(crate) fn components(
    ctx: &Ctx,
    commit: &str,
    culprit: Option<&str>,
    lands: &[String],
) -> Result<Vec<String>, Error> {
    let covered: Vec<&str> = match culprit {
        Some(c) => vec![c],
        None => lands.iter().map(String::as_str).collect(),
    };
    let mut files = BTreeSet::new();
    for land in covered {
        files.extend(changed_files(ctx, land)?);
    }
    let graph = graph_at(ctx, commit)?;
    let owners: BTreeSet<String> = files
        .iter()
        .filter_map(|f| clm_coverage::owner(&graph, Path::new(f)).map(str::to_string))
        .collect();
    Ok(match owners.is_empty() {
        true => vec![clm_model::ROOT_ID.to_string()],
        false => owners.into_iter().collect(),
    })
}

/// What to say on stderr of how the `culprit` land checked its merged tree, when its `landed`
/// record says it ran no check or only the overlap (`culprit-recheck`): the keys are those of
/// the composed `checked` record for the land's code key, or `unknown`. Nothing of the
/// failure's cause.
pub(crate) fn recheck_note(
    ctx: &Ctx,
    state: &State,
    culprit: &str,
) -> Result<Option<String>, Error> {
    let landed = state
        .fold
        .lands
        .get(culprit)
        .and_then(|records| records.last());
    let Some(Body::Landed {
        code_key,
        recheck: Some(recheck),
        recheck_files,
        ..
    }) = landed.map(|r| &r.body)
    else {
        return Ok(None);
    };
    let how = match (recheck, recheck_files) {
        (Recheck::None, _) => "nothing".to_string(),
        (Recheck::Overlap, Some(n)) => format!("the overlap, {n} files"),
        (Recheck::Overlap, None) => "the overlap, every test".to_string(),
        (Recheck::Checked | Recheck::Selected, _) => return Ok(None),
    };
    let basis = code_key
        .as_ref()
        .and_then(|key| state.fold.checks.get(key))
        .and_then(|records| {
            records.iter().rev().find_map(|r| match &r.body {
                Body::Checked { basis, .. } => basis.as_ref(),
                _ => None,
            })
        });
    let short = |key: &str| key.chars().take(12).collect::<String>();
    let (head, base) = basis.map_or_else(
        || ("unknown".to_string(), "unknown".to_string()),
        |b| (short(&b.head), short(&b.base)),
    );
    Ok(Some(format!(
        "culprit {} re-checked {how} at land; its branch's check and main's were composed \
         from {head} and {base}",
        ctx.short(culprit)?
    )))
}

/// The files `land`'s first-parent diff changes, relative to the CLM root.
fn changed_files(ctx: &Ctx, land: &str) -> Result<Vec<String>, Error> {
    let parents = ctx.git(&["rev-list", "--parents", "-n", "1", land])?;
    let out = match parents.split_whitespace().nth(1) {
        Some(parent) => ctx.git(&["diff", "--name-only", "-z", "--no-renames", parent, land])?,
        None => ctx.git(&[
            "diff-tree",
            "--root",
            "-r",
            "--no-commit-id",
            "--name-only",
            "-z",
            "--no-renames",
            land,
        ])?,
    };
    Ok(out
        .split('\0')
        .filter_map(|p| p.strip_prefix(ctx.prefix.as_str()))
        .filter(|p| !p.is_empty())
        .map(str::to_string)
        .collect())
}

/// The spec graph `commit` holds, read from git's objects.
fn graph_at(ctx: &Ctx, commit: &str) -> Result<clm_model::Graph, Error> {
    let dir = format!("{}specs/", ctx.prefix);
    let blobs = clm_git::tree_blobs(&ctx.root, commit, &|p: &str| p.starts_with(&dir))?;
    let files: clm_model::Files = blobs
        .into_iter()
        .filter_map(|(path, bytes)| {
            Some((
                PathBuf::from(path.strip_prefix(ctx.prefix.as_str())?),
                bytes,
            ))
        })
        .collect();
    Ok(clm_model::load_files(&files).0)
}

/// `clm verify --bisect <sha>`: the search for a commit whose newest record is red, and a
/// fresh red record for it naming the culprit and its components.
pub(crate) fn bisect_red(ctx: &Ctx, state: &State, sha: &str) -> Result<Outcome, Error> {
    let (full, end) = state.resolve(ctx, sha)?;
    let Some(Body::Verified {
        passed: false,
        since,
        lands,
        mode,
        run,
        ..
    }) = state.newest(&full).map(|r| &r.body)
    else {
        return Err(Error::Refused(format!(
            "no red `verified` record names {sha}, so there is nothing to bisect"
        )));
    };
    let green = state.last_green(end);
    let found = first_red(ctx, state, green, end)?;
    let culprit = found.culprit;
    let components = components(
        ctx,
        &full,
        culprit.as_deref(),
        found.may_hold.as_deref().unwrap_or(lands),
    )?;
    let body = Body::Verified {
        commit: full.clone(),
        passed: false,
        since: since.clone(),
        lands: lands.clone(),
        culprit: culprit.clone(),
        components: components.clone(),
        mode: *mode,
        run: run.clone(),
    };
    let note = match &culprit {
        Some(c) => recheck_note(ctx, state, c)?,
        None => found.note,
    };
    let appended = clm_ledger::append(&ctx.root, body, ctx.env)?;
    let mut outcome = run::report(
        ctx,
        &full,
        false,
        lands.len(),
        culprit.as_deref(),
        &components,
    )?;
    outcome.notes.extend(note);
    if !appended.pushed {
        outcome
            .notes
            .push("the record is committed locally; `clm reconnect` pushes it".to_string());
    }
    Ok(outcome)
}
