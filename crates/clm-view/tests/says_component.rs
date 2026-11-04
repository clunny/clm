//! `says-component`: the string literals of `crates/clm-view/src/` use no old word.

use std::fs;
use std::path::Path;

/// The old words `component-words` defines: `node`, `nodes`, `spine`, `frontier stub`.
const OLD: [&str; 4] = ["nodes", "node", "spine", "frontier stub"];

/// Data the `web` app and the panel read, which keep their names (`component-words`): the
/// SVG's class names and the blockers model's `"node"` kind.
const KEPT: [(&str, &str); 2] = [("svg.rs", "class=\\\"node"), ("blockers.rs", "node")];

/// The string literals of `text` with their 1-based lines, comments left out.
fn literals(text: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = text.chars().collect();
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
            '\'' if chars.get(i + 1) == Some(&'"') && chars.get(i + 2) == Some(&'\'') => i += 2,
            '"' => {
                let (start, mut s) = (line, String::new());
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        s.push(chars[i]);
                        i += 1;
                    }
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    s.push(chars[i]);
                    i += 1;
                }
                out.push((start, s));
            }
            _ => {}
        }
        i += 1;
    }
    out
}

/// Each old word standing as a word of its own in `s`: no letter, digit or `_` beside it,
/// not joined to more of an ID, path or key by `-`, `/`, `.` or `:`, and not in a `{...}`.
fn old_words(s: &str) -> Vec<&'static str> {
    let lower = s.to_lowercase();
    let b = lower.as_bytes();
    let joined = |c: u8| c.is_ascii_alphanumeric() || b"_-/.:".contains(&c);
    let mut found = Vec::new();
    for word in OLD {
        for (at, _) in lower.match_indices(word) {
            let end = at + word.len();
            // A `.` or `:` ending a sentence is not a join: only a following word character is.
            let before = at > 0 && joined(b[at - 1]);
            let after = end < b.len()
                && (b[end].is_ascii_alphanumeric()
                    || b"_-/".contains(&b[end])
                    || (b".:".contains(&b[end])
                        && b.get(end + 1).is_some_and(|c| c.is_ascii_alphanumeric())));
            let open = lower[..at].rfind('{');
            let in_braces = open.is_some_and(|o| !lower[o..at].contains('}'));
            if !before && !after && !in_braces && !found.contains(&word) {
                found.push(word);
            }
        }
    }
    found
}

// req:view/says-component@a40c940b
#[test]
fn clm_view_literals_use_no_old_word() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bad = Vec::new();
    for entry in fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        for (line, lit) in literals(&text) {
            let marked = text
                .lines()
                .nth(line - 1)
                .is_some_and(|l| l.contains("clm:remove-by"));
            if marked {
                continue;
            }
            if KEPT
                .iter()
                .any(|(f, k)| *f == name && (lit == *k || (*k != "node" && lit.contains(k))))
            {
                continue;
            }
            for word in old_words(&lit) {
                bad.push(format!(
                    "crates/clm-view/src/{name}:{line}: `{word}` in {lit:?}"
                ));
            }
        }
    }
    assert!(
        bad.is_empty(),
        "old words in string literals:\n{}",
        bad.join("\n")
    );
}

// req:view/says-component@a40c940b
#[test]
fn scanner_finds_a_standalone_old_word() {
    assert_eq!(old_words("unknown node `x`"), ["node"]);
    assert!(old_words("model/node-assets and {node} and check/node-size").is_empty());
}
