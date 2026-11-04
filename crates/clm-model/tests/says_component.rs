//! `says-component`: the string literals of `crates/clm-model/src/` use no old word.

use std::fs;
use std::path::Path;

/// The old words `component-words` defines: `node`, `nodes`, `spine`, `frontier stub`.
const OLD: [&str; 4] = ["nodes", "node", "spine", "frontier stub"];

/// Messages and the key that name an old key to say it is old (`component-keys`).
const KEPT: [&str; 5] = [
    "nodes",
    "sets `nodes`, its old key: use `components`",
    "sets both `components` and `nodes`, its old key: use `components`",
    "sets `node`, its old key: use `component`",
    "sets both `component` and `node`, its old key: use `component`",
];

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

// req:model/says-component@eaa8ed18
#[test]
fn clm_model_literals_use_no_old_word() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut bad = Vec::new();
    for entry in fs::read_dir(&src).unwrap() {
        let path = entry.unwrap().path();
        let text = fs::read_to_string(&path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        for (line, lit) in literals(&text) {
            if KEPT.contains(&lit.as_str()) {
                continue;
            }
            for word in old_words(&lit) {
                bad.push(format!(
                    "crates/clm-model/src/{name}:{line}: `{word}` in {lit:?}"
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

// req:model/says-component@eaa8ed18
#[test]
fn scanner_finds_a_standalone_old_word() {
    assert_eq!(old_words("unknown node `x`"), ["node"]);
    assert!(old_words("model/node-assets and {node} and check/node-size").is_empty());
}

use clm_model::{Files, load_files};
use std::path::PathBuf;

fn load(files: &[(&str, &str)]) -> Vec<clm_model::Violation> {
    let mut map = Files::new();
    for (p, t) in files {
        map.insert(PathBuf::from(p), t.as_bytes().to_vec());
    }
    load_files(&map).1
}

fn spec(extra: &str) -> String {
    format!(
        "---\ntitle: T\nstatus: unbuilt\nowners: []\n{extra}---\n\n## Contract\nc\n\n## Design\nd\n"
    )
}

const ROOT: &str = "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n";

// req:model/says-component@eaa8ed18
#[test]
fn an_unknown_component_in_an_item() {
    let item = "---\ntitle: X\nkind: bug\ncomponents: [ghost]\n---\nbody\n";
    let v = load(&[("specs/SPEC.md", ROOT), ("backlog/x.md", item)]);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].message.contains("unknown component `ghost`"), "{v:?}");
}

// req:model/says-component@eaa8ed18
#[test]
fn a_required_component_on_an_optional_one() {
    let a = spec("optional: true\n");
    let b = spec("depends_on: [a]\n");
    let v = load(&[
        ("specs/SPEC.md", ROOT),
        ("specs/a/SPEC.md", &a),
        ("specs/b/SPEC.md", &b),
    ]);
    assert_eq!(v.len(), 1, "{v:?}");
    assert!(v[0].message.contains("required component `b`"), "{v:?}");
    assert!(v[0].message.contains("optional component `a`"), "{v:?}");
}

// req:model/says-component@eaa8ed18
#[test]
fn literals_finds_string_literals_and_skips_comments() {
    let found = literals("let a = \"one node\"; // \"two\"\nlet b = \"three\";\n");
    assert_eq!(
        found,
        [(1, "one node".to_string()), (2, "three".to_string())]
    );
}
