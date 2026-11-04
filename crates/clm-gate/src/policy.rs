//! Whether a diff changes the approval policy (`approval-setting-is-spec`), and whether a
//! spec branch needs review (`needs-review`): pure functions, so the gate and the `web`
//! Worker give one answer. They run no process and build for `wasm32-unknown-unknown`.

use clm_model::ChangeFrontmatter;

/// The human layers and the keys a signed repository trusts: a branch that edits any of
/// them needs review (`needs-review`).
const NEEDS_REVIEW: [&str; 3] = ["ARCHITECTURE.md", "CAPABILITIES.md", "docs/allowed_signers"];

/// Whether a `spec` branch needs a human's review by `needs-review`'s rule, given the paths
/// its diff adds, modifies or deletes, whether `approval-setting` counts `clm.toml` as
/// changed, and the `change.md` frontmatter at `HEAD` of each change one of whose files it
/// adds or modifies (`None` where that file is missing, isn't UTF-8 or doesn't parse).
pub fn needs_review_of(
    paths: &[&str],
    approval_changed: bool,
    changes: &[Option<ChangeFrontmatter>],
) -> bool {
    paths.iter().any(|p| path_needs_review(p))
        || approval_changed
        || changes
            .iter()
            .any(|fm| fm.as_ref().is_none_or(change_needs_review))
}

/// Whether a `change.md` with this frontmatter makes its branch need review: it declares a
/// burden (`needs-review`).
pub(crate) fn change_needs_review(fm: &ChangeFrontmatter) -> bool {
    !fm.burdens.is_empty()
}

/// Whether editing `path` alone makes a spec branch need review: a human layer, or the
/// trusted keys, any entry under that path included, since the ledger reads the path's
/// presence as the signed era.
pub(crate) fn path_needs_review(path: &str) -> bool {
    NEEDS_REVIEW.iter().any(|f| {
        path == *f
            || path
                .strip_prefix(f)
                .is_some_and(|rest| rest.starts_with('/'))
    })
}

/// Whether `approval-setting` counts `clm.toml` as a spec change, given its bytes on each
/// side of a diff (`None` where the file doesn't exist): the top-level `approval` values,
/// compared as parsed TOML with a missing file or key counting as unset, differ, or either
/// side isn't UTF-8 or doesn't parse as TOML, so the gate fails closed.
pub fn approval_changed(before: Option<&[u8]>, after: Option<&[u8]>) -> bool {
    // `None` when the side can't be read as TOML; `Some(None)` when `approval` is unset.
    let approval = |bytes: Option<&[u8]>| -> Option<Option<toml::Value>> {
        let Some(bytes) = bytes else {
            return Some(None);
        };
        std::str::from_utf8(bytes)
            .ok()?
            .parse::<toml::Table>()
            .ok()
            .map(|table| table.get("approval").cloned())
    };
    let (before, after) = (approval(before), approval(after));
    before.is_none() || after.is_none() || before != after
}
