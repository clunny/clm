//! This repository's instructions, `CLAUDE.md` and the `AGENTS.md` it imports when that exists,
//! push branches through `clm branch push`, never `git push` (clm/claude-md-pushes-with-clm).

// clm:reads CLAUDE.md
// clm:reads AGENTS.md

use std::fs;
use std::path::Path;

/// Err naming the file when `files` (name, text) holds `git push`, or when no text names
/// `clm branch push`.
fn push_violation(files: &[(&str, &str)]) -> Result<(), String> {
    for (name, text) in files {
        if text.contains("git push") {
            return Err(format!("{name} names git push"));
        }
    }
    if !files.iter().any(|(_, t)| t.contains("clm branch push")) {
        return Err("no instruction file contains clm branch push".into());
    }
    Ok(())
}

// req:clm/claude-md-pushes-with-clm@af3228bd
#[test]
fn claude_md_pushes_through_clm() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let claude = fs::read_to_string(root.join("CLAUDE.md")).unwrap();
    let agents = fs::read_to_string(root.join("AGENTS.md")).ok();
    let mut files = vec![("CLAUDE.md", claude.as_str())];
    if let Some(a) = &agents {
        files.push(("AGENTS.md", a.as_str()));
    }
    assert_eq!(push_violation(&files), Ok(()));
}

// req:clm/claude-md-pushes-with-clm@af3228bd
#[test]
fn agents_md_holds_the_instructions() {
    let ok = [
        ("CLAUDE.md", "@AGENTS.md"),
        ("AGENTS.md", "use clm branch push"),
    ];
    assert_eq!(push_violation(&ok), Ok(()));
}

// req:clm/claude-md-pushes-with-clm@af3228bd
#[test]
fn git_push_in_either_file_fails() {
    let bad_agents = [
        ("CLAUDE.md", "clm branch push"),
        ("AGENTS.md", "run git push"),
    ];
    assert_eq!(
        push_violation(&bad_agents),
        Err("AGENTS.md names git push".to_string())
    );
    let bad_claude = [("CLAUDE.md", "git push"), ("AGENTS.md", "clm branch push")];
    assert_eq!(
        push_violation(&bad_claude),
        Err("CLAUDE.md names git push".to_string())
    );
}
