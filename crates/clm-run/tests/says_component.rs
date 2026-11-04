//! The prompt and messages `clm run` writes say component (`component-words`).

use std::fs;
use std::path::PathBuf;

use clm_run::{Job, prompt};

/// Old words in `text`, as `component-words` defines them: `node`, `nodes`, `spine` and
/// `stub` standing alone, with no letter, digit, `_`, `-`, `/`, `.` or `:` beside them,
/// and not inside a `{...}` placeholder.
fn old_words(text: &str) -> Vec<&'static str> {
    let mut plain = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let chars: Vec<char> = plain.to_lowercase().chars().collect();
    let word_char = |c: char| c.is_alphanumeric() || c == '_';
    // A neighbour joins the word to more of an ID, path or key: a word character, or one
    // of `-/.:` with a word character beyond it.
    let joined = |near: Option<char>, beyond: Option<char>| match near {
        Some(c) if word_char(c) => true,
        Some(c) if "-/.:".contains(c) => beyond.is_some_and(word_char),
        _ => false,
    };
    let mut out = Vec::new();
    for word in ["nodes", "node", "spines", "spine", "stubs", "stub"] {
        let w: Vec<char> = word.chars().collect();
        for i in 0..(chars.len() + 1).saturating_sub(w.len()) {
            if chars[i..i + w.len()] != w[..] {
                continue;
            }
            let at = |j: Option<usize>| j.and_then(|j| chars.get(j).copied());
            let before = at(i.checked_sub(1));
            let before2 = at(i.checked_sub(2));
            let after = at(Some(i + w.len()));
            let after2 = at(Some(i + w.len() + 1));
            if !joined(before, before2) && !joined(after, after2) {
                out.push(word);
            }
        }
    }
    out
}

/// The string literals of `src`, each with the line it starts on: text between double
/// quotes, across lines, with `\` escapes kept as written. Comments, character literals
/// and raw strings are skipped over as the compiler would read them.
fn literals(src: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = src.chars().collect();
    let (mut out, mut i, mut line) = (Vec::new(), 0, 1);
    while i < chars.len() {
        match chars[i] {
            '\n' => line += 1,
            '/' if chars.get(i + 1) == Some(&'/') => {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '\'' => {
                // A character literal such as `'"'` or `'\''`; a lifetime has no closing quote
                // two or three characters on.
                if chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\'') {
                    i += 4;
                    continue;
                }
                if chars.get(i + 2) == Some(&'\'') {
                    i += 3;
                    continue;
                }
            }
            '"' => {
                let (start, mut text) = (line, String::new());
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' {
                        text.push('\\');
                        i += 1;
                    }
                    if chars.get(i) == Some(&'\n') {
                        line += 1;
                    }
                    text.extend(chars.get(i));
                    i += 1;
                }
                out.push((start, text));
            }
            _ => {}
        }
        i += 1;
    }
    out
}

#[test]
fn string_literals_hold_no_old_word() {
    // req:orchestrator.run/says-component@49a0408e
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap();
        for (line, lit) in literals(&src) {
            for word in old_words(&lit) {
                found.push(format!("{}:{line}: {word}", path.display()));
            }
        }
    }
    assert!(found.is_empty(), "old words: {found:#?}");
}

#[test]
fn scanner_spots_old_words_across_lines() {
    // req:orchestrator.run/says-component@49a0408e
    assert_eq!(old_words("The Spine of a node"), vec!["node", "spine"]);
    assert!(old_words("node-size, a/node, {node}, spine_x, nodes.rs").is_empty());
    let src = "let a = '\"'; // a \"node\"\nlet b = \"one \\\n  node here\";\n";
    let found = literals(src);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].0, 2);
    assert_eq!(old_words(&found[0].1), vec!["node"]);
}

#[test]
fn a_refused_id_says_component() {
    // req:orchestrator.run/says-component@49a0408e
    let dir = tempfile::tempdir().unwrap();
    let err = clm_run::finish(dir.path(), "a/b", 4).unwrap_err();
    assert!(
        format!("{err:?}").contains("`a/b` isn't a component ID"),
        "{err:?}"
    );
}

#[test]
fn a_build_prompt_says_component() {
    // req:orchestrator.run/says-component@49a0408e
    let p = prompt(&Job {
        node: "a".into(),
        spec: "specs/a/SPEC.md".into(),
        ..Job::default()
    });
    for needle in [
        "Bring component `a`'s code up to its spec.",
        "which claims component `a`,",
        "`## Fixes in other components`",
        "`components` name both components",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    // Decision and constraint names (`node-reconciliation`) keep their spelling, and the
    // scanner leaves them alone; anything else is an old word.
    assert!(old_words(&p).is_empty(), "old words in {p}");
}
