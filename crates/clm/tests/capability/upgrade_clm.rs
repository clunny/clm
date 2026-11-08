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

    // The one edit left is the note's, which `/clm:upgrade` has the human take by hand: the
    // allow rules for unattended `clm` commands, and the settings file in the root's `paths`.
    let rules: Vec<String> = [
        "apply",
        "branch",
        "check",
        "claim",
        "defect",
        "gate",
        "land",
        "pr",
        "rebase",
        "verify",
        "workaround",
    ]
    .iter()
    .flat_map(|s| [format!("Bash(clm {s})"), format!("Bash(clm {s} *)")])
    .collect();
    repo.write(
        ".claude/settings.json",
        &serde_json::json!({ "permissions": { "allow": rules } }).to_string(),
    );
    let spec = fs::read_to_string(repo.root.join("specs/SPEC.md")).unwrap();
    let edited = spec.replacen(
        "\"scripts/clm-check.sh\"]",
        "\"scripts/clm-check.sh\", \".claude/settings.json\"]",
        1,
    );
    assert_ne!(edited, spec, "the root's paths line has changed shape");
    repo.write("specs/SPEC.md", &edited);
    repo.commit("take the allow rules by hand");

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

/// The ID `clm init` stamps: the last migration this `clm` holds, named by its directory.
fn latest_migration() -> String {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../clm-upgrade/src/migrations");
    let mut ids: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.starts_with(|c: char| c.is_ascii_digit()))
        .collect();
    ids.sort();
    ids.pop().expect("a migration")
}

#[test]
fn a_stamp_only_land_runs_no_check() {
    // req:land/settings-unchanged-is-checked@d27ea07f
    // The land that takes an upgrade changes `clm.toml` only by its `upgraded` stamp, and the
    // check it would run was already run on the same code.
    let repo = Repo::init();
    let log = repo.root.parent().unwrap().join("check-runs");
    repo.land_on_origin(&format!("echo ran >> {}", log.display()));
    repo.write("src/lib.sh", "greet() { echo hi; }\n");
    repo.push_branch("impl/greet", "greet");
    let first = repo.clm(&["land", "impl/greet"]);
    assert!(first.status.success(), "{}", printed(&first));
    assert_eq!(fs::read_to_string(&log).unwrap().lines().count(), 1);

    repo.git(&["pull", "-q", "--ff-only", "origin", "main"]);
    let toml = fs::read_to_string(repo.root.join("clm.toml")).unwrap();
    repo.write(
        "clm.toml",
        &format!("upgraded = \"{}\"\n{toml}", latest_migration()),
    );
    repo.push_branch("impl/stamp", "take the upgrade");
    let stamp = repo.clm(&["land", "impl/stamp"]);
    assert!(stamp.status.success(), "{}", printed(&stamp));
    assert_eq!(
        fs::read_to_string(&log).unwrap().lines().count(),
        1,
        "the stamp's land ran the check: {}",
        printed(&stamp)
    );
    assert_eq!(repo.origin_main(), crate::landed(&stamp));
}
