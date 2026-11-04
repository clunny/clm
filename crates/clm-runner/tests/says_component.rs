//! The text `clm runner` prints says component, never node
//! (`component-words`).

use std::fs;
use std::path::Path;

/// The string literals of Rust source `text`, with their 1-based line numbers. Comments and
/// character literals are skipped; escapes are kept as written.
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
            // A character literal holding a double quote: '"' or '\"'.
            '\'' if chars.get(i + 1) == Some(&'"') => i += 2,
            '\'' if chars.get(i + 1) == Some(&'\\') && chars.get(i + 2) == Some(&'"') => i += 3,
            'r' if chars.get(i + 1) == Some(&'"')
                || (chars.get(i + 1) == Some(&'#') && chars.get(i + 2) == Some(&'"')) =>
            {
                // A raw string: skip to its closing quote, with its `#` when it has one.
                let hashes = usize::from(chars[i + 1] == '#');
                let close: Vec<char> = std::iter::once('"')
                    .chain("#".repeat(hashes).chars())
                    .collect();
                i += 2 + hashes;
                let (start, mut s) = (line, String::new());
                while i < chars.len() && chars[i..].iter().take(close.len()).ne(close.iter()) {
                    if chars[i] == '\n' {
                        line += 1;
                    }
                    s.push(chars[i]);
                    i += 1;
                }
                i += close.len() - 1;
                out.push((start, s));
            }
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

/// The old words in a literal: `node` or `nodes` standing alone, outside `{...}` placeholders.
fn old_words(literal: &str) -> Vec<String> {
    let mut plain = String::new();
    let mut depth = 0;
    // An escape such as `\n` is not part of the word beside it.
    let literal = literal.replace("\\n", " ").replace("\\t", " ");
    for c in literal.chars() {
        match c {
            '{' => depth += 1,
            '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    let chars: Vec<char> = plain.chars().collect();
    let word = |c: char| c.is_alphanumeric() || c == '_';
    let joined = |c: char| matches!(c, '-' | '/' | '.' | ':');
    let (mut found, mut i) = (Vec::new(), 0);
    while i < chars.len() {
        if !word(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && word(chars[i]) {
            i += 1;
        }
        let w: String = chars[start..i].iter().collect();
        // Joined to more of an ID, a path or a key: a joiner with a word character beyond it.
        let before = start.checked_sub(2).map(|j| (chars[j], chars[j + 1]));
        let after = chars.get(i).zip(chars.get(i + 1));
        let bare = !before.is_some_and(|(far, near)| word(far) && joined(near))
            && !after.is_some_and(|(near, far)| joined(*near) && word(*far));
        if bare && matches!(w.to_lowercase().as_str(), "node" | "nodes" | "spine") {
            found.push(w);
        }
    }
    if plain.to_lowercase().contains("frontier stub") {
        found.push("frontier stub".to_string());
    }
    found
}

/// `file:line: word` for each old word in the string literals of the `.rs` files in `dir`.
fn scan(dir: &Path) -> Vec<String> {
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "rs"))
        .collect();
    files.sort();
    let mut found = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        for (line, lit) in literals(&text) {
            for w in old_words(&lit) {
                // The line holding the word, within a literal that spans several.
                let offset = lit
                    .lines()
                    .position(|l| l.to_lowercase().contains(&w.to_lowercase()))
                    .unwrap_or(0);
                found.push(format!("{}:{}: {w}", file.display(), line + offset));
            }
        }
    }
    found
}

#[test]
fn the_printed_text_says_component() {
    // req:orchestrator.runner/says-component@9b1ad0f2
    let found = scan(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src"));
    assert!(found.is_empty(), "old words:\n{}", found.join("\n"));
}

#[test]
fn the_scanner_reports_old_words_and_spares_ids() {
    // req:orchestrator.runner/says-component@9b1ad0f2
    let source = "let a = \"no nodes\";\n// a node\nlet b = \"model/node-assets {nodes}\";\nlet c = \"a Node's cost\";\n";
    let lits = literals(source);
    assert_eq!(lits.len(), 3);
    assert_eq!(
        old_words("a node. nodes: x\\nnodes"),
        ["node", "nodes", "nodes"]
    );
    assert_eq!(old_words(&lits[0].1), ["nodes"]);
    assert!(old_words(&lits[1].1).is_empty());
    assert_eq!(lits[2].0, 4);
    assert_eq!(old_words(&lits[2].1), ["Node"]);
}
