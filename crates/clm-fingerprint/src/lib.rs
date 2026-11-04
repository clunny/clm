//! The fingerprint an approval is pinned to (see specs/gate/fingerprint/SPEC.md).

use std::path::Path;

mod fingerprint;

pub use fingerprint::{Entry, added_changes, adds_change, fingerprint_of, parse_ls_tree};

/// Fingerprints the changes `HEAD` makes relative to `base`: `fingerprint_of` over the
/// `git ls-tree -r -z` listings of their merge base and of `HEAD`, which no git config
/// changes (`tree-diff-fingerprint`).
pub fn fingerprint(repo: &Path, base: &str) -> Result<String, String> {
    let merge_base =
        clm_git::git(repo, &["merge-base", base, "HEAD"]).map_err(|e| e.to_string())?;
    let listing = |rev: &str| -> Result<Vec<Entry>, String> {
        clm_git::run(
            repo,
            "git",
            &["ls-tree", "-r", "-z", rev],
            Default::default(),
        )
        .map(|out| parse_ls_tree(&out))
        .map_err(|e| e.to_string())
    };
    Ok(fingerprint_of(&listing(&merge_base)?, &listing("HEAD")?))
}
