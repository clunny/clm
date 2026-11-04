//! `approval_changed` (gate/approval-changed): whether `clm.toml`'s `approval` changed.

use clm_gate::approval_changed;

const BEFORE: &[u8] = b"# who approves\napproval = \"other\"\n\n[land]\ncheck = \"a\"\n";
const AFTER: &[u8] = b"# who approves\napproval = \"any\"\n\n[land]\ncheck = \"a\"\n";

#[test]
fn scenario_the_value_changes() {
    // req:gate/approval-changed@bd437cee
    assert!(approval_changed(Some(BEFORE), Some(AFTER)));
    assert!(approval_changed(None, Some(AFTER)));
    assert!(approval_changed(Some(BEFORE), None));
}

#[test]
fn scenario_only_a_comment_changes() {
    // req:gate/approval-changed@bd437cee
    let after = b"# who may approve\napproval = \"other\"\n\n[land]\ncheck = \"b\"\n";
    assert!(!approval_changed(Some(BEFORE), Some(after)));
    assert!(!approval_changed(None, None));
}

#[test]
fn unset_key_and_missing_file_are_the_same() {
    // req:gate/approval-changed@bd437cee
    let unset = b"[land]\ncheck = \"a\"\n";
    assert!(!approval_changed(None, Some(unset)));
    assert!(!approval_changed(Some(unset), None));
    // A value `config` would reject is compared like any other.
    assert!(approval_changed(Some(unset), Some(b"approval = 1\n")));
    assert!(!approval_changed(
        Some(b"approval = 1\n"),
        Some(b"approval = 1 # same\n")
    ));
}

#[test]
fn scenario_a_side_doesnt_parse() {
    // req:gate/approval-changed@bd437cee
    let after = b"approval = \"other\"\n[land\n";
    let after2 = b"approval = \"other\"\n# caf\xe9\n";
    assert!(approval_changed(Some(BEFORE), Some(after)));
    assert!(approval_changed(Some(BEFORE), Some(after2)));
    // Either side, even when both are the same unreadable bytes.
    assert!(approval_changed(Some(after), Some(BEFORE)));
    assert!(approval_changed(Some(after), Some(after)));
}

#[test]
fn approval_changed_runs_no_process() {
    // req:gate/approval-changed@bd437cee
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/policy.rs"),
    )
    .unwrap();
    // `crate::` and `super::` would reach the crate's git helpers.
    for banned in ["clm_git", "Command", "std::process", "crate::", "super::"] {
        assert!(!src.contains(banned), "src/policy.rs mentions {banned}");
    }
}

#[test]
fn the_gate_decides_by_approval_changed() {
    // req:gate/approval-changed@bd437cee
    // The gate reads `clm.toml` only through `approval_changed`, so it has no second
    // comparison that could drift from the Worker's.
    let src = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs"),
    )
    .unwrap();
    assert!(
        src.contains("approval_changed(before"),
        "lib.rs doesn't call it"
    );
    assert!(!src.contains("toml::"), "lib.rs parses TOML itself");
}
