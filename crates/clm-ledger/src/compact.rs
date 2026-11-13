//! `clm ledger compact` (`ledger/compact`).

use std::collections::{BTreeSet, HashSet};
use std::path::Path;

use crate::{Body, Error, Record, branch};

/// What `compact` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Compacted {
    /// How many records it removed.
    pub removed: usize,
    /// Each land without `code_key` whose commit isn't in this clone, so its key couldn't be
    /// computed and every `checked` record was kept; each land that needed review whose commit isn't in
    /// this clone, so the approval it names was kept; and a compacted branch `origin` didn't
    /// take, which stays local until a later push sends it.
    pub warnings: Vec<String>,
}

/// Rewrites the caller's own ledger without the `approved` records whose fingerprint any
/// writer's `landed` record names, except one a land that needed review, whose first parent is signed, names
/// (`verify` reads it), the `commented` and `changes_requested` records whose fingerprint a
/// `landed` record names, and the `checked` records older than the newest `landed` one whose
/// code key no land's code key (its legacy key for a land recorded without one) matches, as
/// a new commit pushed as `append` pushes. When a
/// land without `code_key` has its commit missing from this clone, so its key can't be
/// computed, it keeps every `checked` record and warns, since dropping a check is the one
/// mistake it can't undo; a missing commit of a land that needed review keeps its approval the same way.
/// An `effort` record goes when the fold holds a later one with its `id`, and a `verified`
/// record when the fold holds a newer one of the caller's for its commit.
/// Fetches first, as best it can, so it sees the latest lands; a line that doesn't parse is
/// kept.
pub fn compact(root: &Path, env: &dyn Fn(&str) -> Option<String>) -> Result<Compacted, Error> {
    let writer = crate::writer(root, env)?;
    // Offline, the fold is what was last fetched, which prunes no more than it should.
    let _ = clm_git::fetch(root);
    let fold = crate::fold(root)?;
    let landed: Vec<&Record> = fold.lands.values().flatten().collect();
    let mut fingerprints = HashSet::new();
    let mut land_keys: HashSet<String> = HashSet::new();
    // Lands written before the field: their legacy keys are computed only when needed.
    let mut keyless = BTreeSet::new();
    for record in &landed {
        let Body::Landed {
            fingerprint,
            commit,
            code_key,
            ..
        } = &record.body
        else {
            continue;
        };
        fingerprints.insert(fingerprint.as_str());
        match code_key {
            Some(key) => {
                land_keys.insert(key.clone());
            }
            None => {
                keyless.insert(commit.as_str());
            }
        }
    }
    // Every fingerprint a land names: `fingerprints` loses the ones whose approval stays.
    let landed_fingerprints = fingerprints.clone();
    let mut warnings = Vec::new();
    // `verify` reads the approval that cleared a land that needed review whose first parent is signed, so
    // it stays. Checked only for the caller's own approvals a land names.
    let mine: HashSet<&str> = fold
        .records
        .iter()
        .filter(|r| r.writer == writer)
        .filter_map(|r| match &r.body {
            Body::Approved { fingerprint, .. } => Some(fingerprint.as_str()),
            _ => None,
        })
        .collect();
    for record in &landed {
        let Body::Landed {
            fingerprint,
            commit,
            review: true,
            ..
        } = &record.body
        else {
            continue;
        };
        if !mine.contains(fingerprint.as_str()) || !fingerprints.contains(fingerprint.as_str()) {
            continue;
        }
        if !present(root, commit) {
            warnings.push(format!(
                "kept the approval of {fingerprint}: landed {commit} isn't in this clone, so \
                 whether `clm ledger verify` reads it can't be told; fetch it and compact again"
            ));
            fingerprints.remove(fingerprint.as_str());
        } else if let Some(first) = crate::verify::first_parent(root, commit)?
            && crate::signed(root, &first)?
        {
            fingerprints.remove(fingerprint.as_str());
        }
    }
    let newest = landed.iter().map(|r| r.at.as_str()).max();
    let older = |record: &Record| newest.is_some_and(|n| record.at.as_str() < n);
    // Listing a tree per keyless land costs only when one of the caller's checks could go.
    let candidate = fold.records.iter().any(|r| {
        r.writer == writer
            && older(r)
            && matches!(&r.body, Body::Checked { code_key, .. } if !land_keys.contains(code_key))
    });
    let mut prune_checks = true;
    if candidate {
        for commit in keyless {
            if present(root, commit) {
                land_keys.insert(crate::code_key::legacy_code_key(root, commit)?);
            } else {
                prune_checks = false;
                warnings.push(format!(
                    "kept every `checked` record: landed {commit} isn't in this clone, so its \
                     code key can't be computed; fetch it and compact again"
                ));
            }
        }
    }
    // The legacy keys are in now: a composed check compaction keeps keeps its basis.
    keep_bases(&fold, &mut land_keys, newest);
    let superseded = |record: &Record| match &record.body {
        Body::Approved { fingerprint, .. } => fingerprints.contains(fingerprint.as_str()),
        // The diff they were about is on `main`, whatever their branch.
        Body::Commented { fingerprint, .. } | Body::ChangesRequested { fingerprint, .. } => {
            landed_fingerprints.contains(fingerprint.as_str())
        }
        Body::Checked { code_key, .. } => {
            prune_checks && older(record) && !land_keys.contains(code_key)
        }
        // The fold holds another count of the same session and branch, a later one whoever
        // wrote it.
        Body::Effort { .. } => fold
            .efforts
            .get(&record.id)
            .is_some_and(|latest| latest != record),
        // Only the caller's newest result for each commit stays; one the fold didn't see,
        // appended since, is newer than all it saw.
        Body::Verified { commit, .. } => fold.verifications.get(commit).is_some_and(|all| {
            all.iter().any(|r| r.id == record.id)
                && all
                    .iter()
                    .find(|r| r.writer == writer)
                    .is_some_and(|newest| newest.id != record.id)
        }),
        Body::Landed { .. }
        | Body::Subscribed { .. }
        | Body::Unsubscribed { .. }
        | Body::Proposed { .. }
        | Body::Withdrawn { .. }
        | Body::Answered { .. }
        | Body::Decided { .. } => false,
    };
    let name = branch::ref_of(&writer);
    let mut removed = 0;
    let committed = branch::rewrite(root, &name, "ledger: compact", |current| {
        let text = String::from_utf8_lossy(current);
        let mut kept = String::new();
        removed = 0;
        for line in text.lines() {
            // A `reviewed` line is a kind gone with yes/no review (`yes-no-review`).
            let reviewed = serde_json::from_str::<serde_json::Value>(line)
                .is_ok_and(|v| v["kind"] == "reviewed");
            if reviewed || serde_json::from_str::<Record>(line).is_ok_and(|r| superseded(&r)) {
                removed += 1;
            } else {
                kept.push_str(line);
                kept.push('\n');
            }
        }
        (removed > 0).then(|| kept.into_bytes())
    })?;
    if committed && !branch::push(root, &name)? {
        let short = name.trim_start_matches("refs/heads/");
        warnings.push(format!(
            "the compacted ledger isn't on origin yet: git push origin {short}"
        ));
    }
    Ok(Compacted { removed, warnings })
}

/// Adds to `keys` the code keys of the basis of every composed `checked` record compaction
/// keeps (its key is in `keys`, or it is not older than `newest`, the newest land's time), and
/// theirs in turn: `verify` audits a composed check by its basis (`composed-check`).
fn keep_bases(fold: &crate::Fold, keys: &mut HashSet<String>, newest: Option<&str>) {
    loop {
        let before = keys.len();
        for record in fold.checks.values().flatten() {
            if let Body::Checked {
                code_key,
                basis: Some(basis),
                ..
            } = &record.body
                && (keys.contains(code_key) || newest.is_none_or(|n| record.at.as_str() >= n))
            {
                keys.insert(basis.head.clone());
                keys.insert(basis.base.clone());
            }
        }
        if keys.len() == before {
            return;
        }
    }
}

/// Whether `commit` is in this clone.
fn present(root: &Path, commit: &str) -> bool {
    clm_git::git(root, &["cat-file", "-e", &format!("{commit}^{{commit}}")]).is_ok()
}
