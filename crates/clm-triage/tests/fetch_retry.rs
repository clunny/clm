//! `clm draft`'s fetch (`draft-fetch-retries-race`): a fake `git` first on this process's
//! `PATH` fails `fetch` naming `origin` as the clone's `.git/fake-mode` says. One test runs
//! both scenarios, since `PATH` is the process's.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;

use clm_triage::{Clock, Draft, DraftError, draft};
use common::*;

const FAKE: &str = r#"#!/bin/sh
real=$(PATH="$FAKE_REAL_PATH" command -v git)
dir=$("$real" rev-parse --absolute-git-dir 2>/dev/null)
if [ "$1" = fetch ] && [ -n "$dir" ] && [ -f "$dir/fake-mode" ]; then
  case " $* " in *" origin "*)
    mode=$(cat "$dir/fake-mode")
    echo fetch >> "$dir/fake-calls"
    n=$(grep -c '^fetch$' "$dir/fake-calls")
    if [ "$mode" = always ] || { [ "$mode" = once ] && [ "$n" = 1 ]; }; then
      echo "error: cannot lock ref 'refs/remotes/origin/impl/a'" >&2
      exit 1
    fi
  esac
fi
exec "$real" "$@"
"#;

fn with_item() -> Repo {
    let r = repo();
    write(
        &r.work,
        "backlog/s.md",
        "---\ntitle: T\nkind: bug\n---\nBody.\n",
    );
    push_main(&r.work, "items");
    r
}

fn run(r: &Repo) -> Result<Draft, DraftError> {
    let clock = Clock {
        now: 1_793_143_800,
        tz: Some("UTC".into()),
    };
    draft(&r.work, &["s".to_string()], None, None, 3, 4, &clock)
}

fn origin_holds_claim(r: &Repo) -> bool {
    git(&r.work, &["ls-remote", "origin", "refs/heads/propose/s"]).contains("propose/s")
}

#[test]
fn draft_fetches_through_fetch_retrying() {
    // req:orchestrator.triage/draft-fetch-retries-race@84e46d8b
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

    // A fetch that loses a ref race once goes on.
    let r = with_item();
    fs::write(r.work.join(".git/fake-mode"), "once").unwrap();
    let drafted = run(&r).expect("drafted");
    assert!(drafted.drafted.is_some());
    assert!(origin_holds_claim(&r));

    // A fetch that keeps failing is git's message, and holds nothing.
    let g = with_item();
    fs::write(g.work.join(".git/fake-mode"), "always").unwrap();
    match run(&g) {
        Err(DraftError::Failed(clm_triage::Error::Git(why))) => {
            assert!(why.contains("cannot lock ref"), "{why}")
        }
        other => panic!("{other:?}"),
    }
    assert!(!origin_holds_claim(&g));
    let builds = g.work.join(".git/clm/builds");
    assert!(!builds.exists() || fs::read_dir(builds).unwrap().next().is_none());
}
