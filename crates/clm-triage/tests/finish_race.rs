//! `clm draft --finish`'s deletes on `origin` (`finish-delete-loses-race`): a fake `git` first
//! on this process's `PATH` acts on `origin` or fails just before the first `push` naming it,
//! as the clone's `.git/fake-mode` says. One test runs every scenario, since `PATH` is the
//! process's.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

use clm_orchestrator::{PlaceKind, Placed, take_place};
use common::*;

const FAKE: &str = r#"#!/bin/sh
real=$(PATH="$FAKE_REAL_PATH" command -v git)
dir=$("$real" rev-parse --absolute-git-dir 2>/dev/null)
if [ "$1" = push ] && [ -n "$dir" ] && [ -f "$dir/fake-mode" ]; then
  case " $* " in *" origin "*)
    mode=$(cat "$dir/fake-mode")
    if [ "$mode" = fail ]; then
      echo "fatal: unable to access 'origin'" >&2
      exit 128
    fi
    if [ ! -f "$dir/fake-pushed" ]; then
      touch "$dir/fake-pushed"
      origin=$("$real" remote get-url origin)
      case "$mode" in
        delete-autosync)
          "$real" -C "$origin" update-ref -d refs/heads/autosync/spec/20261027-f ;;
        move-spec)
          export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
          tip=$("$real" -C "$origin" rev-parse refs/heads/spec/20261027-f)
          tree=$("$real" -C "$origin" rev-parse "$tip^{tree}")
          new=$("$real" -C "$origin" commit-tree "$tree" -p "$tip" -m moved)
          "$real" -C "$origin" update-ref refs/heads/spec/20261027-f "$new" ;;
      esac
    fi
  esac
fi
exec "$real" "$@"
"#;

const ID: &str = "20261027-f";

/// The "folded draft's branches deleted" fixture: place `ID`, `changes/<ID>/` on `main`,
/// `origin/spec/<ID>` and `origin/autosync/spec/<ID>` off `main`, and a clean worktree under
/// `.claude/worktrees/` with `spec/<ID>` checked out. Returns the repo and the worktree.
fn folded() -> (Repo, PathBuf) {
    let r = repo();
    let placed = take_place(&r.work, ID, PlaceKind::Draft, &[format!("spec/{ID}")], 3, 4).unwrap();
    assert_eq!(placed, Placed::New);
    let wt = r.work.join(".claude/worktrees/agent-5");
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            &format!("spec/{ID}"),
            wt.to_str().unwrap(),
            "origin/main",
        ],
    );
    let file = format!("changes/{ID}/notes.md");
    for branch in [format!("spec/{ID}"), format!("autosync/spec/{ID}")] {
        push_files(&r.work, &branch, &[(&file, &branch)]);
    }
    write(
        &r.work,
        &format!("changes/{ID}/change.md"),
        &change("applied"),
    );
    push_main(&r.work, "folded");
    (r, wt.canonicalize().unwrap())
}

fn finish(r: &Repo, mode: &str) -> Result<String, clm_triage::Error> {
    fs::write(r.work.join(".git/fake-mode"), mode).unwrap();
    clm_triage::finish(&r.work, ID, 4)
}

fn local_branch(r: &Repo) -> String {
    git(&r.work, &["branch", "--list", &format!("spec/{ID}")])
        .trim()
        .to_string()
}

#[test]
fn finish_goes_on_when_a_delete_loses_its_race() {
    // req:orchestrator.triage/finish-delete-loses-race@55d35194
    let bin = tempfile::tempdir().unwrap();
    let fake = bin.path().join("git");
    fs::write(&fake, FAKE).unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let real = std::env::var("PATH").unwrap();
    // SAFETY: this file's only test, so no other thread reads the environment.
    unsafe {
        std::env::set_var("FAKE_REAL_PATH", &real);
        std::env::set_var("PATH", format!("{}:{real}", bin.path().display()));
    }

    // The autosync branch is deleted after the fetch: skipped, the rest done.
    let (r, wt) = folded();
    let out = finish(&r, "delete-autosync").expect("exits 0");
    let spec = format!("spec/{ID}");
    let autosync = format!("autosync/spec/{ID}");
    assert!(out.contains(&format!("removed origin/{spec}")), "{out}");
    assert!(
        !out.lines().any(|l| l.contains(&autosync)),
        "no line names {autosync}: {out}"
    );
    assert!(!wt.exists(), "the worktree is gone");
    assert_eq!(local_branch(&r), "", "the local branch is gone");
    assert!(out.lines().any(|l| l == format!("removed {spec}")), "{out}");
    assert_eq!(remote_tip(&r, &spec), None);
    assert_eq!(remote_tip(&r, &autosync), None);

    // The spec branch is pushed to after the fetch: kept, the rest done.
    let (r, wt) = folded();
    let out = finish(&r, "move-spec").expect("exits 0");
    assert!(
        !out.lines().any(|l| l.contains(&format!("origin/{spec}"))),
        "no line names origin/{spec}: {out}"
    );
    assert!(remote_tip(&r, &spec).is_some(), "origin keeps {spec}");
    let moved = git(&r.origin, &["log", "-1", "--format=%s", &spec]);
    assert_eq!(moved.trim(), "moved", "at the new commit");
    assert_eq!(remote_tip(&r, &autosync), None);
    assert!(!wt.exists(), "the worktree is gone");

    // A delete git fails, the branch still at the tip read, is git's: exit 2.
    let (r, _wt) = folded();
    match finish(&r, "fail") {
        Err(clm_triage::Error::Git(why)) => assert!(why.contains("unable to access"), "{why}"),
        other => panic!("{other:?}"),
    }
}
