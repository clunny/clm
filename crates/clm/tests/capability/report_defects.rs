//! `report-defects`: a defect in CLM itself, reported from inside a session.

use std::fs;

use crate::{Repo, printed};

#[test]
fn file_a_defect() {
    // req:capability/report-defects@b238d613
    let repo = Repo::init();
    repo.write("clm.toml", "host = \"none\"\ndefects = \"backlog\"\n");
    repo.commit("defects to the backlog");
    let report = "`clm check` failed a marker that names a requirement.\n";

    let out = repo.clm_stdin(
        &["defect", "--title", "Check misfired", "--component", "a"],
        report,
    );
    assert!(out.status.success(), "{}", printed(&out));
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        "backlog/check-misfired.md"
    );
    let item = fs::read_to_string(repo.root.join("backlog/check-misfired.md")).unwrap();
    assert!(item.contains("\nkind: bug\n"), "{item}");
    assert!(item.contains("\ncomponents: [a]\n"), "{item}");
    assert!(item.ends_with(report), "{item}");

    // The item is one the repository's checks accept.
    let check = repo.clm(&["check"]);
    assert!(check.status.success(), "{}", printed(&check));
}
