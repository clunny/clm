//! This repository's `CLAUDE.md` pushes branches through `clm branch push`, never `git push`
//! (clm/claude-md-pushes-with-clm).

// clm:reads CLAUDE.md

use std::fs;
use std::path::Path;

#[test]
fn claude_md_pushes_through_clm() {
    // req:clm/claude-md-pushes-with-clm@78576ed9
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
    assert!(!text.contains("git push"), "CLAUDE.md names git push");
    assert!(
        text.contains("clm branch push"),
        "CLAUDE.md lacks clm branch push"
    );
}
