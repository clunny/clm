//! `unattended-runner`: a runner started on the person's machine drains the queue with no
//! session open.

use std::fs;
use std::os::unix::fs::PermissionsExt;

use crate::{Repo, printed};

/// A fake `claude`, the one program of the machine beyond the boundary. `plugin list` answers
/// that the `clm` plugin is enabled; a headless `-p` session does what a build session does in
/// its worktree: it switches to the node's branch, writes the tests citing the pending
/// requirements named in `FAKE_MARKERS`, pushes and lands with `clm land`.
const FAKE_CLAUDE: &str = r##"#!/bin/sh
if [ "$1" = plugin ]; then
  echo '[{"id":"clm@clm","scope":"user","enabled":true}]'
  exit 0
fi
git switch -q impl/a || exit 1
mkdir -p tests
for m in $FAKE_MARKERS; do
  echo "# $m" > "tests/t_${m##*/}.sh"
  echo true >> "tests/t_${m##*/}.sh"
done
git add -A
git commit -q -m "build a" || exit 1
git push -q origin impl/a || exit 1
"$FAKE_CLM" land impl/a 2>&1 | grep '^landed ' | head -n 1
"##;

#[test]
fn runner_drains_the_queue() {
    // req:capability/unattended-runner@6fe5ce65
    let repo = Repo::init();
    // The fixture's capability is already proven, so only node `a` is pending.
    let capability = &repo.hash("capability", "capability:c")[..8];
    repo.write(
        "tests/c.sh",
        &format!("# {}{capability}\ntrue\n", concat!("req", ":capability/c@")),
    );
    repo.commit("c proven");
    repo.land_on_origin("true");

    // The machine: a `claude` on `PATH`, with no one at the keyboard.
    let bin = repo.root.parent().unwrap().join("fake-bin");
    fs::create_dir_all(&bin).unwrap();
    let claude = bin.join("claude");
    fs::write(&claude, FAKE_CLAUDE).unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());
    let markers = ["x", "y"]
        .iter()
        .map(|s| {
            let version = &repo.hash("a", &format!("requirement:{s}"))[..8];
            format!("{}a/{s}@{version}", concat!("req", ":"))
        })
        .collect::<Vec<_>>()
        .join(" ");

    let out = repo.clm_env(
        &["runner", "--permission-mode", "bypassPermissions", "--once"],
        "",
        &[
            ("PATH", &path),
            ("FAKE_CLM", env!("CARGO_BIN_EXE_clm")),
            ("FAKE_MARKERS", &markers),
        ],
    );
    let text = printed(&out);
    assert!(out.status.success(), "{text}");
    assert!(text.contains("finished a: landed "), "{text}");

    // The node was built and landed, and nothing is left to build.
    repo.git(&["pull", "-q", "--ff-only", "origin", "main"]);
    let pending = repo.clm(&["pending"]);
    assert!(pending.status.success(), "{}", printed(&pending));
    assert_eq!(
        String::from_utf8_lossy(&pending.stdout).trim(),
        "",
        "{text}"
    );
    let next = repo.clm(&["next"]);
    assert_eq!(String::from_utf8_lossy(&next.stdout).trim(), "");
    let ledger = repo.ledger();
    assert!(ledger.iter().any(|r| r[0] == "landed"), "{ledger:?}");
}
