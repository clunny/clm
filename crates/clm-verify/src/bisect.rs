//! `verify-bisect`: the land that broke a red commit, and the components it touched.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clm_ledger::Body;

use crate::chain::State;
use crate::{Ctx, Error, Outcome, run};

/// The position in the chain of the first land that fails `[verify] check` among those after
/// the green commit `green` up to `end`, which is known to fail: a binary search, each probe
/// run as the verification is, under the check lock afresh.
pub(crate) fn first_red(
    ctx: &Ctx,
    state: &State,
    green: Option<usize>,
    end: usize,
) -> Result<usize, Error> {
    let first = green.map_or(0, |g| g + 1);
    let (mut lo, mut hi) = (first, end);
    let green = green.map(|g| state.chain[g].as_str());
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if run::check(ctx, &state.chain[mid], green, None)?.passed {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    Ok(lo)
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
    let culprit = state.chain[first_red(ctx, state, green, end)?].clone();
    let components = components(ctx, &full, Some(&culprit), lands)?;
    let body = Body::Verified {
        commit: full.clone(),
        passed: false,
        since: since.clone(),
        lands: lands.clone(),
        culprit: Some(culprit.clone()),
        components: components.clone(),
        mode: *mode,
        run: run.clone(),
    };
    let appended = clm_ledger::append(&ctx.root, body, ctx.env)?;
    let mut outcome = run::report(ctx, &full, false, lands.len(), Some(&culprit), &components)?;
    if !appended.pushed {
        outcome
            .notes
            .push("the record is committed locally; `clm reconnect` pushes it".to_string());
    }
    Ok(outcome)
}
