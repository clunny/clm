//! `needs_review_of` (gate/needs-review-of): whether a spec branch's diff needs review.

use clm_gate::needs_review_of;
use clm_model::{ChangeFrontmatter, parse_change_frontmatter};

fn change(frontmatter: &str) -> Option<ChangeFrontmatter> {
    let fm = parse_change_frontmatter(&format!("---\n{frontmatter}---\n\n## Intent\n\nx\n"));
    assert!(fm.is_some(), "fixture doesn't parse: {frontmatter}");
    fm
}

#[test]
fn scenario_a_human_layer() {
    // req:gate/needs-review-of@a4b7f8da
    assert!(needs_review_of(&["ARCHITECTURE.md"], false, &[]));
    assert!(needs_review_of(&["CAPABILITIES.md"], false, &[]));
    assert!(needs_review_of(&["docs/allowed_signers"], false, &[]));
    assert!(needs_review_of(&[], true, &[]));
    // A path under one of them counts; a sibling that only shares a prefix doesn't.
    assert!(needs_review_of(&["docs/allowed_signers/alice"], false, &[]));
    assert!(!needs_review_of(&["docs/allowed_signers.md"], false, &[]));
    assert!(!needs_review_of(&["specs/ARCHITECTURE.md"], false, &[]));
}

#[test]
fn scenario_burdens_and_an_unreadable_change() {
    // req:gate/needs-review-of@a4b7f8da
    let paths = ["changes/x/change.md"];
    let burdened = change("status: proposed\nhuman_burdens: [b]\n");
    assert!(needs_review_of(&paths, false, &[burdened]));
    assert!(needs_review_of(&paths, false, &[None]));
    // One such change among others is enough.
    let plain = change("status: proposed\n");
    assert!(needs_review_of(&paths, false, &[plain, None]));
}

#[test]
fn scenario_nothing_else() {
    // req:gate/needs-review-of@a4b7f8da
    let paths = [
        "specs/a/SPEC.md",
        "docs/decisions/d.md",
        "CLAUDE.md",
        "changes/x/change.md",
    ];
    let plain = change("status: applied\n");
    assert!(!needs_review_of(&paths, false, &[plain]));
    assert!(!needs_review_of(
        &paths,
        false,
        &[change("status: applied\nhuman_burdens: []\n")]
    ));
    assert!(!needs_review_of(&[], false, &[]));
}
