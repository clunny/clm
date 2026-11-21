//! The messages `clm survey --components` prints and the files `clm init` writes say
//! component, never the old words (`component-words`).
//!
//! None of its tests reads CLM's own `ARCHITECTURE.md` any more, but the root's
//! `doc_readers.rs` names this file as a reader of it, so it keeps the marker (init's Design):
//!
//! clm:reads ARCHITECTURE.md

use std::fs;
use std::path::Path;

use clm_init::{Options, check_nodes, init};
use tempfile::TempDir;

mod common;
use common::written_files;

/// The old words of `text` (`node`, `nodes`, `spine`, `frontier stub`, in any case) standing as words of their own:
/// no letter, digit or `_` beside them, not joined to an ID, path or key by `-`, `/`, `.`
/// or `:`, and outside a `{...}` placeholder.
fn old_words(text: &str) -> Vec<String> {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    let mut found = Vec::new();
    let mut depth = 0;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if (c == '{' || c == '}') && chars.get(i + 1) == Some(&c) {
            i += 2; // `{{` and `}}` are escaped braces, not placeholders.
            continue;
        }
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ => {}
        }
        let word = |w: &str| chars[i..].iter().take(w.len()).collect::<String>() == w;
        // Longest first, so `nodes` is one word.
        let old = ["frontier stub", "nodes", "node", "spine"]
            .into_iter()
            .find(|w| word(w));
        let Some(old) = old.filter(|_| depth == 0) else {
            i += 1;
            continue;
        };
        let len = old.len();
        let at = |k: Option<usize>| k.and_then(|k| chars.get(k)).copied();
        let alnum = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        // Joined by `-`, `/`, `.` or `:` to more of an ID, path or key: the glue has a
        // letter, digit or `_` on its far side.
        let joined = |glue: Option<char>, far: Option<char>| {
            glue.is_some_and(|g| "-/.:".contains(g)) && alnum(far)
        };
        let before = at(i.checked_sub(1));
        let after = at(Some(i + len));
        if !alnum(before)
            && !alnum(after)
            && !joined(before, at(i.checked_sub(2)))
            && !joined(after, at(Some(i + len + 1)))
        {
            found.push(chars[i..i + len].iter().collect());
        }
        i += len;
    }
    found
}

/// `file:line: word` for each old word in the templates, read whole line by line.
fn template_gaps(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, text) in files {
        for (n, line) in text.lines().enumerate() {
            if line.contains(concat!("clm:remove", "-by")) {
                continue;
            }
            for w in old_words(line) {
                out.push(format!("{path}:{}: {w}", n + 1));
            }
        }
    }
    out
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    clm_git::git(dir.path(), &["init", "-q"]).unwrap();
    dir
}

#[test]
fn a_new_repository_says_component() {
    // req:init/says-component@ea3e27f2
    let dir = repo();
    let opts = Options {
        langs: ["rust", "typescript", "python"].map(String::from).to_vec(),
        detectors: true,
        host: None,
    };
    init(dir.path(), &opts, &clm_config::process_env).unwrap();
    let claude = fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
    assert!(claude.contains("<component-id>/<slug>"), "{claude}");
    let spec = fs::read_to_string(dir.path().join("specs/SPEC.md")).unwrap();
    assert!(
        spec.contains("How ") && spec.contains(" is split into components."),
        "{spec}"
    );
    let written = written_files(dir.path());
    assert!(written.len() >= 9, "{written:?}");
    assert_eq!(template_gaps(&written), Vec::<String>::new());
    // The scanner names the file, line and word, and lets IDs and placeholders through.
    assert_eq!(
        template_gaps(&[("t.md".into(), "ok\nthe spine here, a Node\n".into())]),
        ["t.md:2: spine", "t.md:2: node"]
    );
    assert_eq!(
        template_gaps(&[("t.md".into(), "node-reconciliation {node} a/nodes x\n".into())]),
        Vec::<String>::new()
    );
}

#[test]
fn a_duplicate_in_the_map() {
    // req:init/says-component@ea3e27f2
    let dir = repo();
    let map = "components:\n  - { id: a, paths: [] }\n  - { id: a, paths: [] }\n";
    let found = check_nodes(dir.path(), Path::new("map.yaml"), map).unwrap();
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].message.contains("component `a` appears twice"));
    assert!(old_words(&found[0].message).is_empty());
    let unknown = "components:\n  - { id: a, depends_on: [z] }\n";
    let err = check_nodes(dir.path(), Path::new("map.yaml"), unknown).unwrap_err();
    assert!(err.contains("unknown component `z`"), "{err}");
    assert!(old_words(&err).is_empty(), "{err}");
}
