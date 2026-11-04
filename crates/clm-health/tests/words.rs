//! The text `clm-health` prints and writes says component, never the old words
//! (`component-words`).

use std::fs;
use std::path::Path;

const OLD: [&str; 4] = ["node", "nodes", "spine", "stub"];

/// Each (line, text) of the string literals in `src`, a literal's lines kept apart. Comments are
/// skipped.
fn literals(src: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut in_str = false;
    for (i, line) in src.lines().enumerate() {
        let mut cur = String::new();
        let mut chars = line.chars().peekable();
        let mut prev = ' ';
        while let Some(c) = chars.next() {
            if in_str {
                if c == '\\' {
                    chars.next();
                } else if c == '"' {
                    in_str = false;
                    out.push((i + 1, std::mem::take(&mut cur)));
                } else {
                    cur.push(c);
                }
            } else if c == '/' && chars.peek() == Some(&'/') {
                break;
            } else if c == '"' && prev != '\'' {
                in_str = true;
            }
            prev = c;
        }
        if in_str {
            out.push((i + 1, cur));
        }
    }
    out
}

/// The first old word standing alone in `text`, outside `{...}`.
fn old_word(text: &str) -> Option<String> {
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
    let b: Vec<char> = plain.to_lowercase().chars().collect();
    let joined = |c: char| c.is_alphanumeric() || "_-/.:".contains(c);
    for w in OLD {
        let w: Vec<char> = w.chars().collect();
        for s in 0..b.len().saturating_sub(w.len() - 1) {
            if b[s..s + w.len()] == w[..]
                && (s == 0 || !joined(b[s - 1]))
                && b.get(s + w.len()).is_none_or(|&c| !joined(c))
            {
                return Some(w.iter().collect());
            }
        }
    }
    None
}

#[test]
fn no_old_word_in_the_crates_text() {
    // req:check.health/says-component@15cdb4fa
    let mut found = Vec::new();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let src = fs::read_to_string(&path).unwrap();
        for (line, text) in literals(&src) {
            if src.lines().nth(line - 1).unwrap().contains("clm:remove-by") {
                continue;
            }
            if let Some(w) = old_word(&text) {
                found.push(format!("{}:{line}: {w}", path.display()));
            }
        }
    }
    assert!(found.is_empty(), "old words: {found:#?}");
}

#[test]
fn scanner_finds_a_bare_word_and_spares_ids() {
    // req:check.health/says-component@15cdb4fa
    assert_eq!(old_word("1 nodes"), Some("nodes".into()));
    assert_eq!(old_word("the Node's build"), Some("node".into()));
    assert_eq!(old_word("model/parse-node-specs"), None);
    assert_eq!(old_word("workaround-{node}-{slug}"), None);
}
