//! `wrap-up-session`: what a session leaves behind, cleaned up or named.

use std::fs;

use crate::{Repo, printed};

#[test]
fn end_of_session() {
    // req:capability/wrap-up-session@9ff03c5b
    let repo = Repo::init();
    repo.land_on_origin("true");

    // A build of `a`, in its own worktree, pushed and landed.
    let build = repo.root.join(".claude/worktrees/agent-1");
    let build_dir = build.to_str().unwrap();
    repo.git(&["worktree", "add", "-q", "-b", "impl/a", build_dir]);
    fs::create_dir_all(build.join("src")).unwrap();
    fs::write(build.join("src/a.rs"), "pub fn x() {}\n").unwrap();
    repo.git(&["-C", build_dir, "add", "-A"]);
    repo.git(&["-C", build_dir, "commit", "-q", "-m", "x"]);
    repo.git(&["-C", build_dir, "push", "-q", "-u", "origin", "impl/a"]);
    let land = repo.clm(&["land", "impl/a"]);
    assert!(land.status.success(), "{}", printed(&land));

    // The session's own work, committed but never pushed.
    repo.git(&["switch", "-q", "-c", "spec/next"]);
    repo.write(
        "backlog/idea.md",
        "---\ntitle: \"An idea\"\nkind: idea\ncomponents: [a]\n---\n\nz.\n",
    );
    repo.commit("an idea");

    assert!(build.exists());
    assert!(!repo.git(&["branch", "--list", "impl/a"]).trim().is_empty());

    let out = repo.clm(&["wrap-up"]);
    let text = printed(&out);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(out.status.code(), Some(1), "{text}");
    assert!(lines.contains(&"unpushed spec/next"), "{text}");
    assert_eq!(lines.last(), Some(&"wrap-up incomplete"), "{text}");
    // The landed build is cleaned up: its worktree and its branch are gone.
    assert!(!build.exists(), "{text}");
    let branches = repo.git(&["branch", "--list", "impl/a"]);
    assert!(branches.trim().is_empty(), "{branches}\n{text}");
    // The unpushed work is left as it was.
    assert_eq!(repo.git(&["branch", "--show-current"]).trim(), "spec/next");
}
