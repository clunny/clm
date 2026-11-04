//! The messages say component, never the old words (`component-words`).

use std::fs;
use std::path::PathBuf;

use clm_orchestrator::refusal;

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

/// The text between the double quotes of a non-comment line.
fn literals(line: &str) -> Vec<&str> {
    if line.trim_start().starts_with("//") {
        return Vec::new();
    }
    line.split('"').skip(1).step_by(2).collect()
}

#[test]
fn string_literals_hold_no_old_word() {
    // req:orchestrator/says-component@4aae7d81
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut found = Vec::new();
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let src = fs::read_to_string(&path).unwrap();
        for (n, line) in src.lines().enumerate() {
            for lit in literals(line) {
                for word in old_words(lit) {
                    found.push(format!("{}:{}: {word}", path.display(), n + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "old words: {found:#?}");
}

#[test]
fn unknown_component_refusal() {
    // req:orchestrator/says-component@4aae7d81
    assert_eq!(refusal("zz", None).as_deref(), Some("no component `zz`"));
}

#[test]
fn scanner_spots_old_words() {
    // req:orchestrator/says-component@4aae7d81
    assert_eq!(old_words("The Spine of a node"), vec!["node", "spine"]);
    assert_eq!(old_words("no such node. Node: x"), vec!["node", "node"]);
    assert!(old_words("node-size, a/node, {node}, spine_x, nodes.rs").is_empty());
}
