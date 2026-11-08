//! `doc_only` (`ledger/doc-only`): the paths `code_key` leaves out, listed once here
//! (`light-ci-for-doc-prs`), and `legacy_doc_only` (`ledger/legacy-doc-only`): that list as it
//! was when the `landed` records without a `code_key` were written (`legacy-code-key`). Built
//! without the `git` feature too, for `check.affected` and wasm.

/// A trailing `/` is a prefix, any other entry a whole path. Safe to skip, not what's code,
/// so anything new gets a new key.
const DOC_ONLY: [&str; 8] = [
    "specs/",
    "changes/",
    "backlog/",
    "docs/",
    "ARCHITECTURE.md",
    "CAPABILITIES.md",
    "README.md",
    "CLAUDE.md",
];

/// Whether `path`, relative to the repository root, is doc-only: `code_key` leaves it out.
pub fn doc_only(path: &str) -> bool {
    DOC_ONLY.iter().any(|entry| match entry.strip_suffix('/') {
        Some(_) => path.starts_with(entry),
        None => path == *entry,
    })
}

/// The doc-only list the keyless `landed` records' `checked` keys were computed with. Frozen:
/// it never follows `DOC_ONLY`, since no later change to the list can re-check a tree that
/// has landed.
const LEGACY_DOC_ONLY: [&str; 7] = [
    "specs/",
    "changes/",
    "backlog/",
    "docs/",
    "ARCHITECTURE.md",
    "README.md",
    "CLAUDE.md",
];

/// The one prefix under `docs/` that was code then.
const LEGACY_CODE_UNDER_DOCS: &str = "docs/guide/";

/// Whether `path` was doc-only by the list as it was when the `landed` records without a
/// `code_key` were written: today's, but with everything under `docs/` but `docs/guide/`
/// doc-only. Calls nothing live, so a change to `doc_only` leaves it as it is.
pub fn legacy_doc_only(path: &str) -> bool {
    !path.starts_with(LEGACY_CODE_UNDER_DOCS)
        && LEGACY_DOC_ONLY
            .iter()
            .any(|entry| match entry.strip_suffix('/') {
                Some(_) => path.starts_with(entry),
                None => path == *entry,
            })
}
