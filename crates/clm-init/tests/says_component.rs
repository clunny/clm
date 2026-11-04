//! The text `clm-init` prints and writes says component, never the old words
//! (`component-words`).

use std::fs;
use std::path::{Path, PathBuf};

use clm_init::{Options, check_nodes, init};
use tempfile::TempDir;

/// String literals of Rust source `src`, as (1-based line, text), skipping comments and
/// character literals. Raw strings are read up to their closing quote and hashes.
fn literals(src: &str) -> Vec<(usize, String)> {
    let b: Vec<char> = src.chars().collect();
    let (mut i, mut line, mut out) = (0, 1, Vec::new());
    while i < b.len() {
        let c = b[i];
        if c == '\n' {
            line += 1;
        }
        if c == '/' && b.get(i + 1) == Some(&'/') {
            while i < b.len() && b[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c == '/' && b.get(i + 1) == Some(&'*') {
            while i + 1 < b.len() && !(b[i] == '*' && b[i + 1] == '/') {
                if b[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
            continue;
        }
        if c == '\'' {
            // A char literal ('x', '\n', '"') or a lifetime.
            if b.get(i + 1) == Some(&'\\') {
                i += 3;
                while i < b.len() && b[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if b.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
            continue;
        }
        let raw = c == 'r' && matches!(b.get(i + 1), Some('"' | '#'));
        if c == '"' || raw {
            let mut j = i + usize::from(raw);
            let mut hashes = 0;
            while raw && b.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if b.get(j) != Some(&'"') {
                i += 1;
                continue;
            }
            j += 1;
            let (start, mut text) = (line, String::new());
            while let Some(&d) = b.get(j) {
                if d == '\n' {
                    line += 1;
                }
                if !raw && d == '\\' {
                    text.push(d);
                    if let Some(&e) = b.get(j + 1) {
                        text.push(e);
                        if e == '\n' {
                            line += 1;
                        }
                    }
                    j += 2;
                    continue;
                }
                if d == '"' && (0..hashes).all(|k| b.get(j + 1 + k) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                text.push(d);
                j += 1;
            }
            out.push((start, text));
            i = j;
            continue;
        }
        i += 1;
    }
    out
}

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

/// `file:line: word` for each old word in the string literals of `files` (path, text).
fn gaps(files: &[(String, String)]) -> Vec<String> {
    let mut out = Vec::new();
    for (path, text) in files {
        let lines: Vec<&str> = text.lines().collect();
        for (line, lit) in literals(text) {
            let exempt = lines
                .get(line - 1)
                .is_some_and(|l| l.contains(concat!("clm:remove", "-by")));
            if exempt {
                continue;
            }
            // A message that names the map's old key to say it is retired keeps it.
            if lit.contains("is retired") {
                continue;
            }
            for w in old_words(&lit) {
                out.push(format!("{path}:{line}: {w}"));
            }
        }
    }
    out
}

fn files_under(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            files_under(&p, ext, out);
        } else if p.extension().is_some_and(|e| e == ext) {
            out.push(p);
        }
    }
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

fn read(dir: &str, ext: &str) -> Vec<(String, String)> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    files_under(&crate_dir.join(dir), ext, &mut files);
    files
        .into_iter()
        .map(|p| {
            let rel = p.strip_prefix(crate_dir).unwrap().display().to_string();
            (
                format!("crates/clm-init/{rel}"),
                fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

fn repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    clm_git::git(dir.path(), &["init", "-q"]).unwrap();
    dir
}

#[test]
fn init_text_says_component() {
    // req:init/says-component@9a4c6628
    let sources = read("src", "rs");
    let templates = read("templates", "md");
    assert!(!sources.is_empty() && !templates.is_empty());
    assert_eq!(gaps(&sources), Vec::<String>::new());
    assert_eq!(template_gaps(&templates), Vec::<String>::new());
    // The scanner names the file, line and word, and lets IDs, placeholders and exempt
    // lines through.
    let fixture = |src: &str| vec![("f.rs".to_string(), src.to_string())];
    assert_eq!(
        gaps(&fixture("fn f() {\n let _ = \"a Node `x`\";\n}")),
        ["f.rs:2: node"]
    );
    assert_eq!(
        template_gaps(&[("t.md".into(), "ok\nthe spine here\n".into())]),
        ["t.md:2: spine"]
    );
    for ok in [
        "let _ = \"node-reconciliation {node}\";",
        &format!("let _ = \"a node\"; // {}", concat!("clm:remove", "-by x")),
    ] {
        assert_eq!(gaps(&fixture(ok)), Vec::<String>::new(), "{ok}");
    }
}

#[test]
fn a_new_repository_says_component() {
    // req:init/says-component@9a4c6628
    let dir = repo();
    init(dir.path(), &Options::default()).unwrap();
    let claude = fs::read_to_string(dir.path().join("CLAUDE.md")).unwrap();
    assert!(claude.contains("<component-id>/<slug>"), "{claude}");
    let spec = fs::read_to_string(dir.path().join("specs/SPEC.md")).unwrap();
    assert!(
        spec.contains("How ") && spec.contains(" is split into components."),
        "{spec}"
    );
    for file in [
        "CLAUDE.md",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "specs/SPEC.md",
    ] {
        let text = fs::read_to_string(dir.path().join(file)).unwrap();
        assert_eq!(
            template_gaps(&[(file.to_string(), text)]),
            Vec::<String>::new()
        );
    }
}

#[test]
fn a_duplicate_in_the_map() {
    // req:init/says-component@9a4c6628
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

#[test]
fn a_renamed_constraint() {
    // req:init/says-component@9a4c6628
    let dir = repo();
    init(dir.path(), &Options::default()).unwrap();
    let text = fs::read_to_string(dir.path().join("ARCHITECTURE.md")).unwrap();
    let list = text
        .split("## Enforced by clm")
        .nth(1)
        .and_then(|s| s.split("\n## ").next())
        .unwrap();
    let names: Vec<&str> = list
        .lines()
        .filter_map(|l| l.strip_prefix("- **`"))
        .filter_map(|l| l.split("`**").next())
        .collect();
    assert!(names.contains(&"ancestor-scoped-context"), "{names:?}");
    assert!(!list.contains(concat!("spine", "-scoped-context")));
    let own =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ARCHITECTURE.md"))
            .unwrap();
    for name in names {
        assert!(
            own.contains(&format!("### Constraint: {name}\n")),
            "{name} is no entry of CLM's own ARCHITECTURE.md"
        );
    }
}
