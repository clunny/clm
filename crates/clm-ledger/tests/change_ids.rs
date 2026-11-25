//! `change_ids`, with or without the `git` feature (`ledger/change-ids`).

use clm_ledger::change_ids;

#[test]
fn the_ids_a_branchs_paths_name() {
    // req:ledger/change-ids@2fa5960d
    let paths = [
        "changes/x-y/change.md",
        "changes/x/change.md",
        "changes/x/change.md",
        "changes/archive/z/change.md",
        "changes/archive/change.md",
        "changes/w/deltas/change.md",
        "changes//change.md",
        "changes/v/notes.md",
        "specs/a/SPEC.md",
    ];
    assert_eq!(change_ids(paths), ["x", "x-y"]);
}
