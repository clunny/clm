//! The two keys the ledger's records name, computed without git from the Trees API's
//! listings: a branch's fingerprint (`clm fingerprint`, `tree-diff-fingerprint`) and a
//! commit's code key (`clm_ledger::code_key`). Tests run both against `clm`'s own on real
//! repos.

use std::collections::BTreeSet;

use clm_gate::Entry;
use serde_json::Value;
use sha2::{Digest, Sha256};

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// One entry of a commit's tree, as the Trees API lists it.
#[derive(Debug, Clone, Copy)]
pub struct TreeLine<'a> {
    pub mode: &'a str,
    /// `blob`, `tree` or `commit`.
    pub kind: &'a str,
    pub sha: &'a str,
    pub path: &'a str,
}

/// `clm_ledger::code_key` from a tree listing: SHA-256 over each entry `git ls-tree -r -z`
/// would print, in its order (by path), without trees or the paths `clm_ledger::doc_only`
/// leaves out (`code-key-doc-only`).
pub fn code_key<'a>(entries: impl IntoIterator<Item = TreeLine<'a>>) -> String {
    let mut lines: Vec<TreeLine> = entries
        .into_iter()
        .filter(|e| e.kind != "tree" && !clm_ledger::doc_only(e.path))
        .collect();
    lines.sort_by(|a, b| a.path.as_bytes().cmp(b.path.as_bytes()));
    let mut hash = Sha256::new();
    for e in lines {
        hash.update(format!("{} {} {}\t{}\0", e.mode, e.kind, e.sha, e.path));
    }
    hex(&hash.finalize())
}

/// A commit's tree as the Trees API lists it (`?recursive=1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// Every entry, trees too.
    pub entries: Vec<Entry>,
    /// GitHub left entries out.
    pub truncated: bool,
}

/// The Trees API's JSON as a [`Listing`]; `None` when it has no `tree`.
pub fn listing_of(v: &Value) -> Option<Listing> {
    let entries = v["tree"]
        .as_array()?
        .iter()
        .filter_map(|e| {
            Some(Entry {
                mode: e["mode"].as_str()?.into(),
                kind: e["type"].as_str()?.into(),
                id: e["sha"].as_str()?.into(),
                path: e["path"].as_str()?.as_bytes().to_vec(),
            })
        })
        .collect();
    Some(Listing {
        entries,
        truncated: v["truncated"] == true,
    })
}

impl Listing {
    /// The commit's code key, `None` when the listing is truncated.
    pub fn code_key(&self) -> Option<String> {
        if self.truncated {
            return None;
        }
        let lines = self.entries.iter().filter_map(|e| {
            Some(TreeLine {
                mode: &e.mode,
                kind: &e.kind,
                sha: &e.id,
                path: std::str::from_utf8(&e.path).ok()?,
            })
        });
        Some(code_key(lines))
    }

    /// The file entry at `path`.
    pub fn entry(&self, path: &str) -> Option<&Entry> {
        let path = path.as_bytes();
        self.entries
            .iter()
            .find(|e| e.kind != "tree" && e.path == path)
    }

    /// The listing has a file at `path`.
    pub fn has(&self, path: &str) -> bool {
        self.entry(path).is_some()
    }

    /// The paths, sorted, of the files whose entry differs from `base`'s, or that only one
    /// of the two lists.
    pub fn changed_paths(&self, base: &Listing) -> Vec<String> {
        let files = |l: &Listing| -> BTreeSet<(Vec<u8>, String, String)> {
            let files = l.entries.iter().filter(|e| e.kind != "tree");
            files
                .map(|e| (e.path.clone(), e.mode.clone(), e.id.clone()))
                .collect()
        };
        let (before, after) = (files(base), files(self));
        let mut paths: Vec<String> = before
            .symmetric_difference(&after)
            .map(|(path, ..)| String::from_utf8_lossy(path).into_owned())
            .collect();
        paths.dedup();
        paths
    }
}

/// `clm fingerprint` of the changes from `base` to `head` (`clm_gate::fingerprint_of`),
/// `None` when either listing is truncated, since an entry left out would change it.
pub fn fingerprint(base: &Listing, head: &Listing) -> Option<String> {
    (!base.truncated && !head.truncated)
        .then(|| clm_gate::fingerprint_of(&base.entries, &head.entries))
}
