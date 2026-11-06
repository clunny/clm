//! `upgrade-clm`: a project is brought up to date with the `clm` it runs.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::{Repo, printed};

/// The corpus entry of the earliest published release, the project that release set up.
fn first_release() -> PathBuf {
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../clm-upgrade/corpus");
    let version = |p: &PathBuf| -> Vec<u32> {
        let name = p.file_name().unwrap().to_str().unwrap();
        name.trim_start_matches('v')
            .trim_end_matches(".tar.gz")
            .split('.')
            .map(|n| n.parse().unwrap())
            .collect()
    };
    fs::read_dir(corpus)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_str().unwrap().ends_with(".tar.gz"))
        .min_by_key(version)
        .expect("a corpus entry")
}

/// A repository holding the project as the first published release set it up.
fn first_release_project() -> Repo {
    let repo = Repo::empty();
    let status = Command::new("tar")
        .arg("-xzf")
        .arg(first_release())
        .arg("-C")
        .arg(&repo.root)
        .status()
        .unwrap();
    assert!(status.success());
    repo.commit("as the first release set it up");
    repo
}

#[test]
fn skipping_releases() {
    // req:capability/upgrade-clm@7ef3a507
    let repo = first_release_project();

    // The latest `clm` sees what the release wrote differ from what it expects.
    let before = repo.clm(&["upgrade"]);
    assert!(before.status.success(), "{}", printed(&before));
    assert!(
        printed(&before).lines().any(|l| l.contains("advised")),
        "{}",
        printed(&before)
    );

    // Taking every edit: the files nobody edited are rewritten and the stamp is written.
    let apply = repo.clm(&["upgrade", "--apply"]);
    assert!(apply.status.success(), "{}", printed(&apply));
    assert!(
        fs::read_to_string(repo.root.join("clm.toml"))
            .unwrap()
            .contains("upgraded ="),
        "the stamp is not written"
    );

    let after = repo.clm(&["upgrade"]);
    assert!(after.status.success(), "{}", printed(&after));
    assert_eq!(printed(&after), "up to date\n");
    let check = repo.clm(&["check"]);
    assert!(check.status.success(), "{}", printed(&check));
}

#[test]
fn migration_pending_refuses_check_but_not_status_or_defect() {
    // req:capability/upgrade-clm@7ef3a507
    // The shipped `clm` holds no required migration to leave pending, so the refusal is
    // reached through the other state that blocks: a stamp naming a migration this `clm`
    // doesn't know (backlog/workaround-capability-upgrade-clm-1).
    let repo = first_release_project();
    let toml = fs::read_to_string(repo.root.join("clm.toml")).unwrap();
    repo.write(
        "clm.toml",
        &format!("upgraded = \"99990101-from-the-future\"\n{toml}"),
    );
    repo.commit("stamped by a newer clm");

    let check = repo.clm(&["check"]);
    let text = printed(&check);
    assert_eq!(check.status.code(), Some(4), "{text}");
    assert!(text.contains("99990101-from-the-future"), "{text}");

    let status = repo.clm(&["status"]);
    assert!(status.status.success(), "{}", printed(&status));
    // `defect` runs rather than being refused: whatever it answers, it is not exit 4.
    let defect = repo.clm(&["defect", "--title", "T", "--component", "a"]);
    assert_ne!(defect.status.code(), Some(4), "{}", printed(&defect));
}
