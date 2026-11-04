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
    // req:gate/needs-review-of@988ab216
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
    // req:gate/needs-review-of@988ab216
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
    // req:gate/needs-review-of@988ab216
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

#[test]
fn the_gate_decides_review_by_needs_review_of() {
    // req:gate/needs-review-of@988ab216
    // `evaluate_at` asks `needs_review_of` of every branch but a replayed rename, so the gate and the Worker give one answer, and
    // the pure module still runs no process.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let lib = std::fs::read_to_string(dir.join("lib.rs")).unwrap();
    assert!(
        lib.contains("=> needs_review_of(&paths"),
        "lib.rs doesn't decide by it"
    );
    let policy = std::fs::read_to_string(dir.join("policy.rs")).unwrap();
    assert!(policy.contains("pub fn needs_review_of("));
    for banned in ["clm_git", "Command", "std::process", "crate::", "super::"] {
        assert!(!policy.contains(banned), "src/policy.rs mentions {banned}");
    }
}
