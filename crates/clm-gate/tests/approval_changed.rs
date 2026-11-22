//! `approval_changed` (gate/approval-changed): whether `clm.toml`'s `approval` changed.

use clm_gate::approval_changed;

const BEFORE: &[u8] = b"# who approves\napproval = \"other\"\n\n[land]\ncheck = \"a\"\n";
const AFTER: &[u8] = b"# who approves\napproval = \"any\"\n\n[land]\ncheck = \"a\"\n";

#[test]
fn scenario_the_value_changes() {
    // req:gate/approval-changed@af4eae32
    assert!(approval_changed(Some(BEFORE), Some(AFTER)));
    assert!(approval_changed(None, Some(AFTER)));
    assert!(approval_changed(Some(BEFORE), None));
}

#[test]
fn scenario_only_a_comment_changes() {
    // req:gate/approval-changed@af4eae32
    let after = b"# who may approve\napproval = \"other\"\n\n[land]\ncheck = \"b\"\n";
    assert!(!approval_changed(Some(BEFORE), Some(after)));
    assert!(!approval_changed(None, None));
}

#[test]
fn unset_key_and_missing_file_are_the_same() {
    // req:gate/approval-changed@af4eae32
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
    // req:gate/approval-changed@af4eae32
    let after = b"approval = \"other\"\n[land\n";
    let after2 = b"approval = \"other\"\n# caf\xe9\n";
    assert!(approval_changed(Some(BEFORE), Some(after)));
    assert!(approval_changed(Some(BEFORE), Some(after2)));
    // Either side, even when both are the same unreadable bytes.
    assert!(approval_changed(Some(after), Some(BEFORE)));
    assert!(approval_changed(Some(after), Some(after)));
}
