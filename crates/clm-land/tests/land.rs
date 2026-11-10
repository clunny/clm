mod common;

use clm_ledger::{Body, LandKind};
use common::{Fixture, refused};

#[test]
fn code_branch_lands() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    let origin = f.path("origin.git");
    assert_eq!(f.parents(&origin, &main), [old, head.clone()]);
    assert_eq!(f.lines("check.log"), [f.tree(&origin, &main)]);
    // The check runs outside CI, handed the repository's `.clm/land` as `CLM_LAND_DIR`.
    let target = f.work().join(".clm/land");
    assert_eq!(
        f.lines("check-env.log"),
        [format!("CI=unset CLM_LAND_DIR={}", target.display())]
    );
    // The full `clm check` ran on the merged tree.
    assert_eq!(
        f.lines("clm-check.log"),
        [format!("{} ", f.tree(&origin, &main))]
    );
    assert_eq!(
        f.checked(),
        [clm_ledger::code_key(&f.work(), &main).unwrap()]
    );
    match &f.landed()[..] {
        [
            Body::Landed {
                commit,
                kind,
                review,
                code_key,
                ..
            },
        ] => {
            assert_eq!(commit, &main);
            assert_eq!(*kind, LandKind::Code);
            assert!(!*review);
            // The land names the code key its `checked` record names.
            assert_eq!(code_key.as_deref(), Some(&f.checked()[0][..]));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(out.lines, [format!("landed {main} code")]);
    let message = f.git(&origin, &["log", "-1", "--format=%B", &main]);
    assert_eq!(message, "Merge impl/a into main");
    // The gate ran on the head, then on the merge commit.
    let gate: Vec<String> = f
        .lines("gate.log")
        .iter()
        .map(|l| l.split(' ').next().unwrap().to_owned())
        .collect();
    assert_eq!(gate, [head, main]);
    f.assert_no_scratch();
}

#[test]
fn already_checked_code() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    let head = f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    let key = clm_ledger::code_key(&f.work(), "origin/main").unwrap();
    f.append(
        &f.work(),
        Body::Checked {
            code_key: key,
            command: f.check_command(),
            basis: None,
        },
    );
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(
        f.lines("clm-check.log"),
        [format!(
            "{} --no-detectors",
            f.tree(&f.path("origin.git"), &main)
        )]
    );
    assert_eq!(f.parents(&f.path("origin.git"), &main)[1], head);
}

#[test]
fn unapproved_hard_branch() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("gate/approval"), "{why}");
    assert!(why.contains("clm approve spec/x"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn land_prints_review() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let fp = f.fingerprint("spec/x");
    f.append(
        &f.work(),
        Body::Approved {
            fingerprint: fp.clone(),
            branch: "spec/x".into(),
            change: Some("x".into()),
        },
    );
    let out = clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} spec review")]);
    match &f.landed()[..] {
        [
            Body::Landed {
                kind,
                review,
                changes,
                fingerprint,
                ..
            },
        ] => {
            assert_eq!(*kind, LandKind::Spec);
            assert!(*review);
            assert_eq!(changes, &["x"]);
            assert_eq!(fingerprint, &fp);
        }
        other => panic!("{other:?}"),
    }
    let message = f.git(&f.path("origin.git"), &["log", "-1", "--format=%B", &main]);
    assert_eq!(message, "Merge spec/x into main\n\nChange: x");
}

#[test]
fn stale_approval() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("gate-out", "spec review\n");
    f.write("needs-approval", "");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let old_fingerprint = f.fingerprint("spec/x");
    f.append(
        &f.work(),
        Body::Approved {
            fingerprint: old_fingerprint.clone(),
            branch: "spec/x".into(),
            change: Some("x".into()),
        },
    );
    // A content change moves the fingerprint on.
    let work = f.work();
    f.git(&work, &["checkout", "-q", "spec/x"]);
    f.commit_files(&[("changes/x/change.md", "x, amended\n")], "amend");
    f.git(&work, &["push", "-q", "origin", "spec/x"]);
    f.git(&work, &["checkout", "-q", "main"]);
    assert_ne!(f.fingerprint("spec/x"), old_fingerprint);
    let old = f.remote("main");
    let why = refused(clm_land::land(&work, Some("spec/x"), &f.env(&[])));
    assert!(why.contains("gate/approval"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn main_moves() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("move-main", "");
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let moved = f.lines("moved").remove(0);
    let main = f.remote("main");
    let origin = f.path("origin.git");
    assert_eq!(f.parents(&origin, &main), [moved, head]);
    assert_eq!(
        f.lines("check.log").last().unwrap(),
        &f.tree(&origin, &main)
    );
    assert_eq!(f.lines("check.log").len(), 2);
    // The first attempt's check passed, so its record stands.
    assert_eq!(f.checked().len(), 2);
}

#[test]
fn conflict() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let main = f.advance_main("b.txt", "other\n");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("clm rebase"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), main);
}

#[test]
fn check_fails() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
    f.assert_no_scratch();
}

#[test]
fn from_main() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    let why = refused(clm_land::land(&f.work(), None, &f.env(&[])));
    assert!(why.contains("`main` can't be landed"), "{why}");
}

#[test]
fn unpushed_impl_commits_refused() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "impl/a"]);
    f.commit_files(&[("c.txt", "c\n")], "unpushed");
    f.git(&work, &["checkout", "-q", "main"]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&work, Some("impl/a"), &f.env(&[])));
    assert!(why.contains("push"), "{why}");
    assert_eq!(f.remote("main"), old);
}

#[test]
fn impl_branch_removed() {
    // req:land/landed-branch-cleanup@58d9f2ab
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(branch_warnings(&out).is_empty(), "{:?}", out.warnings);
    let origin = f.path("origin.git");
    assert_eq!(f.git(&origin, &["branch", "--list", "impl/a"]), "");
    // The local branch, checked out nowhere, goes too.
    assert_eq!(f.git(&f.work(), &["branch", "--list", "impl/a"]), "");
}

#[test]
fn moved_head_kept() {
    // req:land/landed-branch-cleanup@58d9f2ab
    let f = Fixture::new("");
    f.write("move-head", "impl/a");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let moved = f.lines("head-moved").remove(0);
    assert_eq!(f.remote("impl/a"), moved);
    assert!(!branch_warnings(&out).is_empty(), "{:?}", out.warnings);
}

#[test]
fn other_branch_kept() {
    // req:land/landed-branch-cleanup@58d9f2ab
    let f = Fixture::new("");
    let head = f.branch("feature/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("feature/a"), &f.env(&[])).unwrap();
    assert_eq!(f.remote("feature/a"), head);
}

#[test]
fn spec_land_that_needs_no_review() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    f.branch("spec/y", &[("changes/y/change.md", "y\n")]);
    let out = clm_land::land(&f.work(), Some("spec/y"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(out.lines, [format!("landed {main} spec")]);
    match &f.landed()[..] {
        [Body::Landed { kind, review, .. }] => {
            assert_eq!(*kind, LandKind::Spec);
            assert!(!*review);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn checked_with_another_command() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    let key = clm_ledger::code_key(&f.work(), "origin/main").unwrap();
    f.append(
        &f.work(),
        Body::Checked {
            code_key: key,
            command: "some other check".into(),
            basis: None,
        },
    );
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("check.log").len(), 1);
}

#[test]
fn clm_check_fails() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("clm-check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("clm check"), "{why}");
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
}

#[test]
fn specs_changed_on_both_sides() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    // Main changes the same file, at a different line, so git merges it cleanly.
    let main = f.advance_main("specs/x/SPEC.md", "# x\n\n\n\n\nmain\n");
    let why = refused(clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])));
    assert!(why.contains("clm rebase"), "{why}");
    assert!(f.lines("check.log").is_empty());
    assert_eq!(f.remote("main"), main);
}

#[test]
fn main_keeps_moving() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.write("move-main", "always");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("keeps moving"), "{why}");
    assert_eq!(f.lines("check.log").len(), 10);
    assert!(f.landed().is_empty());
}

#[test]
fn origin_branch_ahead_of_local() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    // Another clone pushes on; the local `impl/a` is behind it.
    let other = f.path("other");
    f.git(&other, &["fetch", "-q", "origin", "impl/a"]);
    f.git(&other, &["checkout", "-q", "-b", "impl/a", "FETCH_HEAD"]);
    std::fs::write(other.join("c.txt"), "c\n").unwrap();
    f.git(&other, &["add", "-A"]);
    f.git(&other, &["commit", "-q", "-m", "more"]);
    f.git(&other, &["push", "-q", "origin", "impl/a"]);
    let newer = f.git(&other, &["rev-parse", "HEAD"]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let main = f.remote("main");
    assert_eq!(f.parents(&f.path("origin.git"), &main)[1], newer);
}

#[test]
fn unpushed_propose_lands_from_local() {
    // req:land/land@f4bd0a7c
    let f = Fixture::new("");
    f.branch("propose/a", &[("b.txt", "b\n")]);
    let work = f.work();
    f.git(&work, &["checkout", "-q", "propose/a"]);
    let local = f.commit_files(&[("c.txt", "c\n")], "unpushed");
    f.git(&work, &["checkout", "-q", "main"]);
    clm_land::land(&work, Some("propose/a"), &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main")[1], local);
}

#[test]
fn clm_in_a_subdirectory() {
    // req:land/land@f4bd0a7c
    let f = Fixture::nested();
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.lines("check-dir.log"), ["proj"]);
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn clm_in_a_subdirectory_builds_under_its_own_clm_directory() {
    // req:land/shared-land-target@2c8695d7
    let f = Fixture::nested();
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    let target = f.path("work/proj/.clm/land");
    assert_eq!(
        f.lines("check-env.log"),
        [format!("CI=unset CLM_LAND_DIR={}", target.display())]
    );
    // The land tree and the lock are under that `.clm/` too (land/land-tree@e95a603c), and
    // nothing of the land shows in `git status`.
    let tree = f.land_tree().join("proj").display().to_string();
    assert_eq!(f.lines("check-pwd.log"), [tree]);
    assert_eq!(f.lines("check-lock.log"), ["held"]);
    assert!(!f.path("work/.clm").exists());
    assert_eq!(f.git(&f.path("work"), &["status", "--porcelain"]), "");
}

#[test]
fn land_from_a_linked_worktree() {
    // req:land/shared-land-target@2c8695d7
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let linked = f.path("linked");
    f.git(
        &f.work(),
        &["worktree", "add", "-q", linked.to_str().unwrap(), "impl/a"],
    );
    let target = f.work().join(".clm/land");
    let expected = format!("CI=unset CLM_LAND_DIR={}", target.display());
    clm_land::land(&linked, None, &f.env(&[])).unwrap();
    assert_eq!(f.lines("check-env.log"), std::slice::from_ref(&expected));
    // In the main worktree's land tree, not one under the linked worktree
    // (land/land-tree@e95a603c).
    assert_eq!(
        f.lines("check-pwd.log"),
        [f.land_tree().display().to_string()]
    );
    assert!(!linked.join(".clm").exists());
    // The check ran holding the clone's lock on `.clm/land.lock`.
    assert_eq!(f.lines("check-lock.log"), ["held"]);
    // The land set neither cargo variable: the check saw this process's own.
    let ambient = |name: &str| std::env::var(name).unwrap_or_else(|_| "unset".into());
    assert_eq!(
        f.lines("check-cargo.log"),
        [format!(
            "{} {}",
            ambient("CARGO_TARGET_DIR"),
            ambient("CARGO_INCREMENTAL")
        )]
    );
    // And held it through `clm check` (land/land-target-lock@66ccebd0).
    assert_eq!(f.lines("clm-check-lock.log"), ["held"]);
}

#[test]
fn lock_held_through_the_gate_on_the_merge_and_the_push() {
    // req:land/land-target-lock@66ccebd0
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    // `origin`'s hook logs whether the lock is held when `main` is pushed.
    f.hook(Some(
        r#"lock="$DIR/work/.clm/land.lock"
while read -r _ _ ref; do
  if [ "$ref" = refs/heads/main ]; then
    if flock -n "$lock" true; then echo free; else echo held; fi >> "$DIR/push-lock.log"
  fi
done"#,
    ));
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    // The gate on the head and the gate on the merge commit, both under the lock.
    assert_eq!(f.lines("gate-lock.log"), ["held", "held"]);
    assert_eq!(f.lines("push-lock.log"), ["held"]);
    assert!(out.lines[0].starts_with("landed "), "{:?}", out.lines);
    // Released afterwards.
    let lock = std::fs::File::open(f.work().join(".clm/land.lock")).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while lock.try_lock().is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "the lock is still held"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

#[test]
fn caller_target_changes_nothing() {
    // req:land/shared-land-target@2c8695d7
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let env = f.env(&[("CARGO_TARGET_DIR", "/elsewhere")]);
    clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    // The land neither reads nor sets it: the check holds the lock and gets the same place
    // (land/land-target-lock@66ccebd0).
    let target = f.work().join(".clm/land");
    assert_eq!(
        f.lines("check-env.log"),
        [format!("CI=unset CLM_LAND_DIR={}", target.display())]
    );
    assert_eq!(f.lines("check-lock.log"), ["held"]);
    assert_eq!(
        f.lines("check-pwd.log"),
        [f.land_tree().display().to_string()]
    );
}

#[test]
fn lock_released_after_a_failing_check() {
    // req:land/land-target-lock@66ccebd0
    let f = Fixture::new("");
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert_eq!(f.lines("check-lock.log"), ["held"]);
    let lock = std::fs::File::open(f.work().join(".clm/land.lock")).unwrap();
    // A child another test's thread forks shares the land's descriptor until it execs, for
    // a millisecond or so; a lock the land kept would stay held.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while lock.try_lock().is_err() {
        assert!(
            std::time::Instant::now() < deadline,
            "the failed land didn't release the lock"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    drop(lock);
    std::fs::remove_file(f.path("check-fails")).unwrap();
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
}

#[test]
fn conflict_takes_no_lock() {
    // req:land/land-target-lock@66ccebd0
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.advance_main("b.txt", "other\n");
    let home = f.work().join(".clm");
    std::fs::create_dir_all(&home).unwrap();
    let held = std::fs::File::create(home.join("land.lock")).unwrap();
    held.lock().unwrap();
    // It would wait for ever were it to take the lock.
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(why.contains("clm rebase"), "{why}");
}

#[test]
fn copy_failure_refused() {
    // req:land/land-binary-copy@7e2d6c4e
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    // A `TMPDIR` that is a file, so no directory can be made in it.
    let tmp = f.path("not-a-dir");
    f.write("not-a-dir", "");
    let env = f.env(&[("TMPDIR", tmp.to_str().unwrap()), ("CLM_GATE", "")]);
    let why = refused(clm_land::land(&f.work(), Some("impl/a"), &env));
    assert!(why.contains(tmp.to_str().unwrap()), "{why}");
    assert_eq!(f.remote("main"), old);
    assert!(f.records().is_empty());
    assert!(f.lines("gate.log").is_empty());
}

#[test]
fn build_target_freed() {
    // req:land/land-frees-build-target@67aa5013
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let linked = f.linked("linked", "impl/a");
    let target = linked.join("target");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("file"), "x").unwrap();
    let out = clm_land::land(&linked, None, &f.env(&[])).unwrap();
    assert!(!target.exists());
    assert!(linked.join("b.txt").exists(), "the worktree went too");
    let canonical = std::fs::canonicalize(&linked).unwrap().join("target");
    assert_eq!(
        out.lines.last().unwrap(),
        &format!("removed {}", canonical.display()),
        "{:?}",
        out.lines
    );
}

#[test]
fn main_worktree_target_kept() {
    // req:land/land-frees-build-target@67aa5013
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.git(&f.work(), &["checkout", "-q", "impl/a"]);
    let target = f.work().join("target");
    std::fs::create_dir_all(&target).unwrap();
    std::fs::write(target.join("file"), "x").unwrap();
    let out = clm_land::land(&f.work(), None, &f.env(&[])).unwrap();
    assert!(target.join("file").exists());
    assert!(
        !out.lines.iter().any(|l| l.starts_with("removed")),
        "{:?}",
        out.lines
    );
}

#[test]
fn spec_land_target_kept() {
    // req:land/land-frees-build-target@67aa5013
    let f = Fixture::new("");
    f.write("gate-out", "spec\n");
    f.branch("spec/y", &[("specs/y/SPEC.md", "# y\n")]);
    let linked = f.linked("linked", "spec/y");
    std::fs::create_dir_all(linked.join("target")).unwrap();
    clm_land::land(&linked, None, &f.env(&[])).unwrap();
    assert!(linked.join("target").exists());
}

#[test]
fn failed_land_target_kept() {
    // req:land/land-frees-build-target@67aa5013
    let f = Fixture::new("");
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let linked = f.linked("linked", "impl/a");
    std::fs::create_dir_all(linked.join("target")).unwrap();
    refused(clm_land::land(&linked, None, &f.env(&[])));
    assert!(linked.join("target").exists());
}

#[test]
fn other_branch_target_kept() {
    // req:land/land-frees-build-target@67aa5013
    let f = Fixture::new("");
    f.branch("spec/z", &[("specs/z/SPEC.md", "# z\n")]);
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let linked = f.linked("linked", "spec/z");
    std::fs::create_dir_all(linked.join("target")).unwrap();
    clm_land::land(&linked, Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
    assert!(linked.join("target").exists());
}

#[test]
fn failed_delete_warns() {
    // req:land/land-frees-build-target@67aa5013
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let linked = f.linked("linked", "impl/a");
    let stuck = linked.join("target/stuck");
    std::fs::create_dir_all(&stuck).unwrap();
    std::fs::write(stuck.join("file"), "x").unwrap();
    // A directory whose entries can't be removed.
    std::fs::set_permissions(&stuck, std::fs::Permissions::from_mode(0o555)).unwrap();
    let out = clm_land::land(&linked, None, &f.env(&[]));
    std::fs::set_permissions(&stuck, std::fs::Permissions::from_mode(0o755)).unwrap();
    let out = out.unwrap();
    assert!(stuck.join("file").exists(), "the delete didn't fail");
    assert!(
        !out.lines.iter().any(|l| l.starts_with("removed")),
        "{:?}",
        out.lines
    );
    assert!(
        branch_warnings(&out).iter().any(|w| w.contains("target")),
        "{:?}",
        out.warnings
    );
}

#[test]
fn check_runs_outside_git() {
    // req:land/scratch-outside-git@cca0dc01
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    // `pwd -P` printed it canonical.
    let dirs = f.lines("check-pwd.log");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    let dir = std::path::PathBuf::from(&dirs[0]);
    assert_eq!(dir, f.land_tree());
    assert!(
        !dir.components().any(|c| c.as_os_str() == ".git"),
        "{}",
        dir.display()
    );
    f.assert_no_scratch();
}

#[test]
fn scratch_removed_after_a_failing_check() {
    // req:land/scratch-outside-git@cca0dc01
    let f = Fixture::new("");
    f.write("check-fails", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert_eq!(
        f.lines("check-pwd.log"),
        [f.land_tree().display().to_string()]
    );
    // TMPDIR is empty again, and git lists the main worktree and the land tree alone.
    f.assert_no_scratch();
    assert_eq!(f.worktrees().len(), 2, "{:?}", f.worktrees());
}

#[test]
fn a_land_that_compiles_nothing_uses_a_temporary_worktree() {
    // req:land/scratch-outside-git@cca0dc01
    let f = Fixture::new("");
    f.branch("spec/x", &[("specs/x/SPEC.md", "# x\n")]);
    let key = clm_ledger::code_key(&f.work(), "origin/main").unwrap();
    f.append(
        &f.work(),
        Body::Checked {
            code_key: key,
            command: f.check_command(),
            basis: None,
        },
    );
    clm_land::land(&f.work(), Some("spec/x"), &f.env(&[])).unwrap();
    let tmp = std::fs::canonicalize(f.tmpdir()).unwrap();
    let dirs = f.lines("clm-check-pwd.log");
    assert_eq!(dirs.len(), 1, "{dirs:?}");
    // Gone now, so it can't be canonicalised here.
    let dir = std::path::PathBuf::from(&dirs[0]);
    assert!(
        dir.starts_with(&tmp),
        "{} not under {}",
        dir.display(),
        tmp.display()
    );
    assert!(!dir.starts_with(f.land_tree()), "{}", dir.display());
    f.assert_no_scratch();
    // No land tree was made for it.
    assert_eq!(f.worktrees().len(), 1, "{:?}", f.worktrees());
}

#[test]
fn one_tree_for_every_land() {
    // req:land/land-tree@e95a603c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    let tree = f.land_tree().display().to_string();
    assert_eq!(f.lines("check-pwd.log"), [tree.clone(), tree.clone()]);
    // And `clm check` ran there too.
    assert_eq!(f.lines("clm-check-pwd.log"), [tree.clone(), tree.clone()]);
    let list = f.git(&f.work(), &["worktree", "list", "--porcelain"]);
    let entry = list
        .split("\n\n")
        .find(|e| e.starts_with(&format!("worktree {tree}\n")))
        .unwrap_or_else(|| panic!("{list}"));
    assert!(entry.lines().any(|l| l == "detached"), "{entry}");
    f.assert_no_scratch();
}

#[test]
fn leftovers_cleaned() {
    // req:land/land-tree@e95a603c
    let f = Fixture::new("");
    // Fails when an earlier check's file is still there, ignored or not.
    f.write(
        "check-prints",
        "if [ -e left ] || [ -e target/left ]; then echo left over; exit 1; fi\n\
         touch left; mkdir -p target; touch target/left\n",
    );
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    // A file the gate's run left in a tracked path changed is reset too.
    std::fs::write(f.land_tree().join("a.txt"), "edited\n").unwrap();
    clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    assert_eq!(
        std::fs::read_to_string(f.land_tree().join("a.txt")).unwrap(),
        "a\n"
    );
}

#[test]
fn tree_repaired_after_clm_directory_removed() {
    // req:land/land-tree@e95a603c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    std::fs::remove_dir_all(f.work().join(".clm")).unwrap();
    clm_land::land(&f.work(), Some("impl/b"), &f.env(&[])).unwrap();
    let tree = f.land_tree().display().to_string();
    assert_eq!(f.lines("check-pwd.log"), [tree.clone(), tree]);
    f.assert_no_scratch();
}

#[test]
fn stale_directory_replaced() {
    // req:land/land-tree@e95a603c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    let stale = f.work().join(".clm/land-tree");
    std::fs::create_dir_all(&stale).unwrap();
    std::fs::write(stale.join("stale"), "x").unwrap();
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert_eq!(
        f.lines("check-pwd.log"),
        [f.land_tree().display().to_string()]
    );
    assert!(!stale.join("stale").exists());
    assert_eq!(f.parents(&f.path("origin.git"), "main").len(), 2);
}

#[test]
fn nothing_untracked_in_a_project_that_doesnt_ignore_target() {
    // req:land/land-tree@e95a603c
    // The fixture's `.gitignore` holds only `clm init`'s lines.
    let f = Fixture::new("");
    let work = f.work();
    assert_eq!(
        std::fs::read_to_string(work.join(".gitignore")).unwrap(),
        "clm.local.toml\n.clm/\n.claude/worktrees/\n"
    );
    f.write("check-builds", "");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    clm_land::land(&work, Some("impl/a"), &f.env(&[])).unwrap();
    assert!(work.join(".clm/land/built").exists());
    f.write("check-fails", "");
    refused(clm_land::land(&work, Some("impl/b"), &f.env(&[])));
    assert!(work.join(".clm/land-logs/impl/b.log").exists());
    assert_eq!(f.git(&work, &["status", "--porcelain"]), "");
    assert!(!work.join("target").exists());
}

#[test]
fn broken_tree_repaired_without_touching_the_main_worktree() {
    // req:land/land-tree@e95a603c
    let f = Fixture::new("");
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f.branch("impl/b", &[("c.txt", "c\n")]);
    clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    // Listed and present, but its `.git` file gone: git run there finds the main worktree.
    std::fs::remove_file(f.land_tree().join(".git")).unwrap();
    let work = f.work();
    std::fs::write(work.join("untracked"), "mine\n").unwrap();
    let main_head = f.rev("HEAD");
    clm_land::land(&work, Some("impl/b"), &f.env(&[])).unwrap();
    assert!(
        work.join("untracked").exists(),
        "the main worktree was cleaned"
    );
    assert_eq!(f.rev("HEAD"), main_head);
    assert_eq!(f.git(&work, &["branch", "--show-current"]), "main");
    assert_eq!(f.lines("check-pwd.log").len(), 2);
    assert!(f.land_tree().join(".git").is_file());
}

/// A fixture whose check leaves a sparse file of `size` bytes in `CLM_LAND_DIR`, with
/// `[land] target_cap_gb = 1` when `cap` is set.
fn capped(cap: bool, size: &str) -> Fixture {
    let f = Fixture::new("");
    if cap {
        f.land_setting("target_cap_gb = 1");
    }
    f.write("check-sparse", size);
    f.branch("impl/a", &[("b.txt", "b\n")]);
    f
}

#[test]
fn target_over_the_cap_cleared() {
    // req:land/land-target-cap@69c8919e
    // req:land/shared-target-cap@07ebae61
    let f = capped(true, "2000000000");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.lines.last().unwrap().starts_with("landed "), "{out:?}");
    assert!(
        out.warnings
            .contains(&"cleared .clm/land (2 GB, cap 1 GB)".to_string()),
        "{:?}",
        out.warnings
    );
    assert!(!f.work().join(".clm/land").exists());
}

#[test]
fn target_under_the_cap_kept() {
    // req:land/land-target-cap@69c8919e
    let f = capped(false, "10");
    let out = clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])).unwrap();
    assert!(out.warnings.iter().all(|w| !w.contains("cleared")));
    assert!(f.work().join(".clm/land/big").exists());
}

#[test]
fn failed_check_keeps_the_target() {
    // req:land/land-target-cap@69c8919e
    let f = capped(true, "2000000000");
    f.write("check-fails", "");
    refused(clm_land::land(&f.work(), Some("impl/a"), &f.env(&[])));
    assert!(f.work().join(".clm/land/big").exists());
}

/// The land's warnings but the effort step's (`land/records-effort`), which refuses in this
/// fixture: it isn't a CLM repository.
fn branch_warnings(out: &clm_land::Output) -> Vec<&String> {
    out.warnings
        .iter()
        .filter(|w| {
            !w.starts_with("effort: ")
                && !w.starts_with("effort not recorded: ")
                && !w.starts_with("land priority not worked out")
                && !w.starts_with("check ")
        })
        .collect()
}
