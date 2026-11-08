//! `code_key` (`ledger/code-key`): a commit's tree with the doc-only paths left out, hashed;
//! a keyless land's legacy key, the same with the list as it was (`legacy-code-key`); and
//! `code_key_with`, a key with one file taken from another commit.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::Error;

/// SHA-256, as hex, over `git ls-tree -r -z <rev>` without its doc-only entries, each entry
/// followed by its NUL as `ls-tree` writes it.
pub fn code_key(root: &Path, rev: &str) -> Result<String, Error> {
    key_without(root, rev, crate::doc_only)
}

/// `rev`'s code key with `path`'s entry as `from`'s tree holds it (`ledger/code-key-with`):
/// `from`'s `ls-tree` line for `path` replaces `rev`'s, or is inserted in byte order of paths
/// when `rev` has none, or leaves the entry out when `from` has none, `path` matched exactly
/// against each listing. Computed from the two listings, so no git object is written. A path `doc_only` names isn't in the key at all, so
/// the key is `rev`'s own.
pub fn code_key_with(root: &Path, rev: &str, path: &str, from: &str) -> Result<String, Error> {
    if crate::doc_only(path) {
        return code_key(root, rev);
    }
    // Both listings in full, searched by path, so `path` matches one file exactly and is never
    // read as a pathspec.
    let find = |entries: &[Vec<u8>]| entries.binary_search_by(|e| path_of(e).cmp(path.as_bytes()));
    let mut entries = listing(root, rev)?;
    let mut base = listing(root, from)?;
    let taken = find(&base).ok().map(|n| base.swap_remove(n));
    match (find(&entries), taken) {
        (Ok(n), Some(line)) => entries[n] = line,
        (Ok(n), None) => {
            entries.remove(n);
        }
        (Err(n), Some(line)) => entries.insert(n, line),
        (Err(_), None) => {}
    }
    Ok(hash(&entries, crate::doc_only))
}

/// The key a `landed` record written before the `code_key` field was checked under: as
/// `code_key`, but leaving out what `legacy_doc_only` names.
pub(crate) fn legacy_code_key(root: &Path, rev: &str) -> Result<String, Error> {
    key_without(root, rev, crate::legacy_doc_only)
}

fn key_without(root: &Path, rev: &str, skip: fn(&str) -> bool) -> Result<String, Error> {
    Ok(hash(&listing(root, rev)?, skip))
}

/// The entries `git ls-tree -r -z <rev>` lists, in its order: byte order of paths.
fn listing(root: &Path, rev: &str) -> Result<Vec<Vec<u8>>, Error> {
    let tree = clm_git::run(
        root,
        "git",
        &["ls-tree", "-r", "-z", rev],
        clm_git::Options::default(),
    )?;
    Ok(tree
        .split(|b| *b == 0)
        .filter(|e| !e.is_empty())
        .map(<[u8]>::to_vec)
        .collect())
}

/// The path of an `ls-tree` entry: the bytes after its tab.
fn path_of(entry: &[u8]) -> &[u8] {
    entry
        .iter()
        .position(|b| *b == b'\t')
        .map_or(&[], |tab| &entry[tab + 1..])
}

/// SHA-256, as hex, over the entries `skip` doesn't name, each followed by its NUL.
fn hash(entries: &[Vec<u8>], skip: fn(&str) -> bool) -> String {
    let mut hash = Sha256::new();
    for entry in entries {
        if skip(&String::from_utf8_lossy(path_of(entry))) {
            continue;
        }
        hash.update(entry);
        hash.update([0]);
    }
    hash.finalize().iter().map(|b| format!("{b:02x}")).collect()
}
