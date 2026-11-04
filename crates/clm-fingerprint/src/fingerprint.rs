//! The fingerprint of two trees' entries (`tree-diff-fingerprint`): a pure function, so
//! `clm fingerprint` and the `web` Worker compute the same value. It runs no process and
//! builds for `wasm32-unknown-unknown`.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

/// One entry of a tree listing, as `git ls-tree -r` lists it: a mode, a type (`blob`,
/// `tree`, `commit`), a full object ID and a path, kept as bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub mode: String,
    pub kind: String,
    pub id: String,
    pub path: Vec<u8>,
}

/// The entries of `git ls-tree -r -z` output: `<mode> <type> <id>\t<path>\0` per entry.
/// A record that doesn't have that shape is skipped.
pub fn parse_ls_tree(listing: &[u8]) -> Vec<Entry> {
    listing
        .split(|&b| b == 0)
        .filter_map(|record| {
            let tab = record.iter().position(|&b| b == b'\t')?;
            let (meta, path) = (
                std::str::from_utf8(&record[..tab]).ok()?,
                &record[tab + 1..],
            );
            let mut fields = meta.split(' ');
            let (mode, kind, id) = (fields.next()?, fields.next()?, fields.next()?);
            Some(Entry {
                mode: mode.to_string(),
                kind: kind.to_string(),
                id: id.to_string(),
                path: path.to_vec(),
            })
        })
        .collect()
}

/// The lowercase hex SHA-256, over each path whose entry differs between `base` and
/// `head` or exists in one only, sorted by its bytes, of the path, a NUL, its `base` entry
/// as `<mode> <type> <id>` or `-` when absent, a NUL, its `head` entry the same way, and a
/// NUL; `empty` when no path differs. `tree` entries are ignored, and so is every path
/// under `specs/` when `head` adds a change (`derived-specs-fingerprint`): there `specs/`
/// is regenerated from the authored files, and `land/derived-specs` refuses a hand edit.
pub fn fingerprint_of(base: &[Entry], head: &[Entry]) -> String {
    let skip_specs = adds_change(base, head);
    let by_path = |entries: &[Entry]| -> BTreeMap<Vec<u8>, String> {
        entries
            .iter()
            .filter(|e| e.kind != "tree")
            .filter(|e| !(skip_specs && e.path.starts_with(b"specs/")))
            .map(|e| (e.path.clone(), format!("{} {} {}", e.mode, e.kind, e.id)))
            .collect()
    };
    let (base, head) = (by_path(base), by_path(head));
    let mut paths: Vec<&Vec<u8>> = base.keys().chain(head.keys()).collect();
    paths.sort();
    paths.dedup();
    let mut hasher = Sha256::new();
    let mut any = false;
    for path in paths {
        let (b, h) = (base.get(path), head.get(path));
        if b == h {
            continue;
        }
        any = true;
        for part in [path.as_slice(), side(b), side(h)] {
            hasher.update(part);
            hasher.update([0]);
        }
    }
    if !any {
        return "empty".to_string();
    }
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn side(entry: Option<&String>) -> &[u8] {
    entry.map_or(b"-", |e| e.as_bytes())
}

/// Whether `head` adds a change over `base`: it holds a `changes/<id>/change.md` that
/// `base` lacks, or a new `changes/archive/<id>/change.md` while `base` lacks
/// `changes/<id>/change.md` too, so an archive move adds none. `<id>` is one path
/// component and never `archive`.
pub fn adds_change(base: &[Entry], head: &[Entry]) -> bool {
    !added_changes(base, head).is_empty()
}

/// The `change.md` paths by which `head` adds a change over `base`, as [`adds_change`]
/// defines it, in `head`'s order.
pub fn added_changes(base: &[Entry], head: &[Entry]) -> Vec<Vec<u8>> {
    let base: BTreeSet<&[u8]> = base
        .iter()
        .filter(|e| e.kind != "tree")
        .map(|e| e.path.as_slice())
        .collect();
    head.iter()
        .filter(|e| e.kind != "tree" && !base.contains(e.path.as_slice()))
        .map(|e| e.path.clone())
        .filter(|path| {
            if let Some(id) = change_id(path, b"changes/archive/") {
                let live = [b"changes/".as_slice(), id, b"/change.md"].concat();
                return !base.contains(live.as_slice());
            }
            change_id(path, b"changes/").is_some_and(|id| id != b"archive")
        })
        .collect()
}

/// The `<id>` of `<prefix><id>/change.md`, when `<id>` is one non-empty path component.
fn change_id<'a>(path: &'a [u8], prefix: &[u8]) -> Option<&'a [u8]> {
    let id = path.strip_prefix(prefix)?.strip_suffix(b"/change.md")?;
    (!id.is_empty() && !id.contains(&b'/')).then_some(id)
}
