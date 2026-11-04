//! `code_key` (`ledger/code-key`): a commit's tree with the doc-only paths left out, hashed;
//! and a keyless land's legacy key, the same with the list as it was (`legacy-code-key`).

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::Error;

/// SHA-256, as hex, over `git ls-tree -r -z <rev>` without its doc-only entries, each entry
/// followed by its NUL as `ls-tree` writes it.
pub fn code_key(root: &Path, rev: &str) -> Result<String, Error> {
    key_without(root, rev, crate::doc_only)
}

/// The key a `landed` record written before the `code_key` field was checked under: as
/// `code_key`, but leaving out what `legacy_doc_only` names.
pub(crate) fn legacy_code_key(root: &Path, rev: &str) -> Result<String, Error> {
    key_without(root, rev, crate::legacy_doc_only)
}

fn key_without(root: &Path, rev: &str, skip: fn(&str) -> bool) -> Result<String, Error> {
    let tree = clm_git::run(
        root,
        "git",
        &["ls-tree", "-r", "-z", rev],
        clm_git::Options::default(),
    )?;
    let mut hash = Sha256::new();
    for entry in tree.split(|b| *b == 0).filter(|e| !e.is_empty()) {
        let path = entry
            .iter()
            .position(|b| *b == b'\t')
            .map(|tab| String::from_utf8_lossy(&entry[tab + 1..]));
        if path.is_some_and(|p| skip(&p)) {
            continue;
        }
        hash.update(entry);
        hash.update([0]);
    }
    Ok(hash.finalize().iter().map(|b| format!("{b:02x}")).collect())
}
