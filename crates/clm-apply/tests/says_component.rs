//! The messages of `clm apply` say component, never the old words (`component-words`).

use std::fs;
use std::path::{Path, PathBuf};

use clm_apply::{apply, validate};
use clm_model::load;
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
            for w in old_words(&lit) {
                out.push(format!("{path}:{line}: {w}"));
            }
        }
    }
    out
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            rust_files(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

fn source_files() -> Vec<(String, String)> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&crate_dir.join("src"), &mut files);
    files
        .into_iter()
        .map(|p| {
            let rel = p.strip_prefix(crate_dir).unwrap().display().to_string();
            (
                format!("crates/clm-apply/{rel}"),
                fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

#[test]
fn apply_source_says_component() {
    // req:apply/says-component@7418473d
    let files = source_files();
    assert!(!files.is_empty());
    assert_eq!(gaps(&files), Vec::<String>::new());
    // The scanner names the file, line and word, and lets IDs, keys, placeholders and
    // exempt lines through.
    let fixture = |src: &str| vec![("f.rs".to_string(), src.to_string())];
    assert_eq!(
        gaps(&fixture("fn f() {\n let _ = \"a Node `x`\";\n}")),
        ["f.rs:2: node"]
    );
    assert_eq!(
        gaps(&fixture("let _ = '\\''; let _ = \"the root node.\";")),
        ["f.rs:1: node"]
    );
    assert_eq!(
        gaps(&fixture("let _ = \"unexpected {{ in node: x\";")),
        ["f.rs:1: node"]
    );
    assert_eq!(
        gaps(&fixture("let _ = \"the Spine and a frontier stub\";")),
        ["f.rs:1: spine", "f.rs:1: frontier stub"]
    );
    assert_eq!(
        gaps(&fixture("let _ = \"the nodes here\";")),
        ["f.rs:1: nodes"]
    );
    for ok in [
        "let _ = \"apply/create-node\";",
        "let _ = \"{node} and {}\";",
        "let _ = \"node_size\"; // node here",
        "let _ = \"{{ {node} }} x\"; let _ = '\\''; let _ = \"ok\";",
        "let _ = \"model/node-assets a.node.b\";",
        "let _ = 'n'; let _ = \"component\";",
        &format!("let _ = \"a node\"; // {}", concat!("clm:remove", "-by x")),
    ] {
        assert_eq!(gaps(&fixture(ok)), Vec::<String>::new(), "{ok}");
    }
}

const ROOT: &str = "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nroot\n\n## Design\nroot\n\n## Requirements\n";
const A: &str = "---\ntitle: A\nstatus: unbuilt\nowners: []\ndepends_on: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n";

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

#[test]
fn creating_a_component_that_exists() {
    // req:apply/says-component@7418473d
    let dir = TempDir::new().unwrap();
    let d = dir.path();
    write(d, "specs/SPEC.md", ROOT);
    write(d, "specs/a/SPEC.md", A);
    write(d, "changes/c/change.md", "---\nstatus: proposed\n---\n");
    write(
        d,
        "changes/c/deltas/a.md",
        "---\ncomponent: a\ncreate: true\n---\n## Frontmatter\n```yaml\ntitle: A\nstatus: unbuilt\nowners: []\n```\n\n## Contract\nc\n\n## Design\nd\n",
    );
    let (graph, _) = load(d);
    let violations = validate(&graph, &graph.changes["c"]);
    let before = fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap();
    let refused = apply(d, "c").unwrap_err();
    assert!(
        refused
            .iter()
            .any(|v| v.message.contains("component `a` already exists")),
        "{refused:?}"
    );
    assert_eq!(
        fs::read_to_string(d.join("specs/a/SPEC.md")).unwrap(),
        before
    );
    assert!(
        violations
            .iter()
            .any(|v| v.message.contains("component `a` already exists")),
        "{violations:?}"
    );
}
