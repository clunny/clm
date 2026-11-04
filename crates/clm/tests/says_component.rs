//! What `clm` prints says component, ancestors and placeholder component, never the
//! old words (clm/says-component, `component-words`).

// clm:reads README.md

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The old words, as whole words of their own (`component-words`).
const OLD_WORDS: &[&str] = &["node", "nodes", "spine", "frontier stub"];

/// What keeps an old word, each with its reason (`component-words`): the exact text
/// of a line, or of a literal, that holds it.
const KEPT: &[(&str, &str)] = &[
    (
        "`node <id>` and `file <path>` lines",
        "`clm affected`'s lines, which the check command and `clm land` parse, and the help that describes them",
    ),
    (
        "node {node}",
        "`clm affected`'s line for an affected component, a contract between programs",
    ),
];

#[derive(Debug, PartialEq)]
struct Hit {
    place: String,
    line: usize,
    word: String,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `text` holds `word` standing alone: no letter, digit or `_` beside it, not
/// joined by `-`, `/`, `.` or `:` to more of an ID, a path or a key, and not inside a
/// `{...}` placeholder. Ignores case; a several-word `word` matches across any run of spaces.
fn holds_word(text: &str, word: &str) -> bool {
    let chars: Vec<char> = text.to_lowercase().chars().collect();
    let parts: Vec<Vec<char>> = word
        .split_whitespace()
        .map(|w| w.chars().collect())
        .collect();
    // A hyphen joins only inside an ID or a path (`check/node-size`); in prose it is a
    // boundary (`per-node`).
    let in_path = text
        .split_whitespace()
        .any(|t| t.to_lowercase().contains(word) && (t.contains('/') || t.contains(':')));
    let joins = |c: char| matches!(c, '/' | '.' | ':') || (c == '-' && in_path);
    (0..chars.len()).any(|start| {
        let mut at = start;
        for (i, part) in parts.iter().enumerate() {
            if i > 0 {
                let gap = chars[at..].iter().take_while(|c| **c == ' ').count();
                if gap == 0 {
                    return false;
                }
                at += gap;
            }
            if !chars[at..].starts_with(part) {
                return false;
            }
            at += part.len();
        }
        if start > 0 {
            let before = chars[start - 1];
            if is_word(before) {
                return false;
            }
            if joins(before) && start > 1 && is_word(chars[start - 2]) {
                return false;
            }
        }
        if at < chars.len() {
            let after = chars[at];
            if is_word(after) {
                return false;
            }
            if joins(after) && at + 1 < chars.len() && is_word(chars[at + 1]) {
                return false;
            }
        }
        let open = chars[..start].iter().rposition(|c| *c == '{');
        let close_before = chars[..start].iter().rposition(|c| *c == '}');
        let inside =
            open.is_some_and(|o| close_before.is_none_or(|c| c < o)) && chars[at..].contains(&'}');
        !inside
    })
}

/// Each line of `text` that holds an old word, kept by nothing and not marked
/// `clm:remove-by`.
fn check(place: &str, text: &str, first_line: usize) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.contains("clm:remove-by") || KEPT.iter().any(|(kept, _)| line.contains(kept)) {
            continue;
        }
        for word in OLD_WORDS {
            if holds_word(line, word) {
                hits.push(Hit {
                    place: place.to_string(),
                    line: first_line + i,
                    word: (*word).to_string(),
                });
            }
        }
    }
    hits
}

/// The string literals of Rust source, each with the line it starts on: comments and
/// character literals are skipped, raw strings are read whole.
fn literals(src: &str) -> Vec<(usize, String)> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut line = 1;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            i += 2;
            while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                if chars[i] == '\n' {
                    line += 1;
                }
                i += 1;
            }
            i += 2;
        } else if c == '\'' {
            // A char literal (`'"'`, `'\''`) or a lifetime.
            if chars.get(i + 1) == Some(&'\\') {
                // An escaped char literal (`'\\''`, `'\\u{22}'`): up to the closing quote.
                i += 2;
                while i < chars.len() && chars[i] != '\'' {
                    i += 1;
                }
                i += 1;
            } else if chars.get(i + 2) == Some(&'\'') {
                i += 3;
            } else {
                i += 1;
            }
        } else if c == 'r'
            && matches!(chars.get(i + 1), Some('"' | '#'))
            && !is_ident_before(&chars, i)
        {
            let mut j = i + 1;
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) != Some(&'"') {
                i += 1;
                continue;
            }
            j += 1;
            let start_line = line;
            let mut text = String::new();
            loop {
                if j >= chars.len() {
                    break;
                }
                if chars[j] == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                    j += 1 + hashes;
                    break;
                }
                if chars[j] == '\n' {
                    line += 1;
                }
                text.push(chars[j]);
                j += 1;
            }
            out.push((start_line, text));
            i = j;
        } else if c == '"' {
            let start_line = line;
            let mut text = String::new();
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                if chars[i] == '\\' {
                    i += 1;
                    if chars.get(i) == Some(&'\n') {
                        // A line continuation drops the break and the next line's indent.
                        line += 1;
                        i += 1;
                        while chars.get(i).is_some_and(|c| c.is_whitespace()) {
                            if chars[i] == '\n' {
                                line += 1;
                            }
                            i += 1;
                        }
                        continue;
                    }
                    if let Some(e) = chars.get(i) {
                        text.push(match e {
                            'n' => '\n',
                            't' => '\t',
                            other => *other,
                        });
                    }
                    i += 1;
                    continue;
                }
                if chars[i] == '\n' {
                    line += 1;
                }
                text.push(chars[i]);
                i += 1;
            }
            i += 1;
            out.push((start_line, text));
        } else {
            i += 1;
        }
    }
    out
}

fn is_ident_before(chars: &[char], i: usize) -> bool {
    i > 0 && is_word(chars[i - 1])
}

fn files_under(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            files_under(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn run_help(args: &[&str]) -> String {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .arg("--help")
        .output()
        .unwrap();
    assert!(out.status.success(), "clm {args:?} --help");
    String::from_utf8(out.stdout).unwrap()
}

/// The subcommands a help text lists under `Commands:`, but `help`.
fn subcommands(help: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.starts_with("Commands:") {
            in_commands = true;
        } else if in_commands {
            if !line.starts_with(' ') {
                break;
            }
            if line.starts_with("   ") {
                continue;
            }
            if let Some(name) = line.split_whitespace().next()
                && name != "help"
            {
                names.push(name.to_string());
            }
        }
    }
    names
}

/// The help of `clm` and of every command and subcommand, each under its name.
fn all_help() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut queue: Vec<Vec<String>> = vec![Vec::new()];
    while let Some(path) = queue.pop() {
        let args: Vec<&str> = path.iter().map(String::as_str).collect();
        let text = run_help(&args);
        for sub in subcommands(&text) {
            let mut next = path.clone();
            next.push(sub);
            queue.push(next);
        }
        let mut name = vec!["clm".to_string()];
        name.extend(path);
        out.push((format!("{} --help", name.join(" ")), text));
    }
    out.sort();
    out
}

fn report(hits: &[Hit]) -> String {
    hits.iter()
        .map(|h| format!("{}:{}: {}", h.place, h.line, h.word))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_scanner_finds_old_words_only_where_they_stand_alone() {
    // req:clm/says-component@3db77145
    for text in [
        "claims `impl/<node>`",
        "Node ID",
        "the nodes ready",
        "per-node contracts",
        "a spine-scoped bundle",
        "make it a Frontier  Stub",
        "unknown node.",
    ] {
        assert_eq!(check("x", text, 1).len(), 1, "{text}");
    }
    for text in [
        "check/node-size",
        "model/parse-node-specs",
        "format!(\"no {node} here\")",
        "{nodes}",
        "import node:fs",
        "nodeId and unnodes and nodes_x",
        "Nodes.js is not a word, node.js is joined",
        "a clm:remove-by line says node",
    ] {
        assert!(check("x", text, 1).is_empty(), "{text}");
    }
}

#[test]
fn the_literal_reader_takes_strings_whole() {
    // req:clm/says-component@3db77145
    let src = "// \"not this\"\nfn f() {\n    let a = \"one\\n two\";\n    let b = 'x';\n    let c = '\"';\n    let d = r#\"raw \"quoted\" text\"#;\n    /* \"nor this\" */\n    let e = \"a \\\n        b\";\n    let g = format!(\"{x}\");\n}\n";
    let got = literals(src);
    let texts: Vec<&str> = got.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(
        texts,
        ["one\n two", "raw \"quoted\" text", "a b", "{x}"],
        "{got:?}"
    );
    assert_eq!(got[0].0, 3);
    assert_eq!(got[1].0, 6);
}

#[test]
fn the_help_of_every_command_says_component() {
    // req:clm/says-component@3db77145
    let mut hits = Vec::new();
    for (name, text) in all_help() {
        hits.extend(check(&name, &text, 1));
    }
    assert!(hits.is_empty(), "old words in the help:\n{}", report(&hits));
}

#[test]
fn arguments_that_take_a_component_are_named_component() {
    // req:clm/says-component@3db77145
    for args in [
        &["context"][..],
        &["hashes"],
        &["size"],
        &["pending"],
        &["claim"],
        &["run"],
    ] {
        let help = run_help(args);
        assert!(
            help.contains("<COMPONENT>") || help.contains("[COMPONENT]"),
            "{args:?}:\n{help}"
        );
        assert!(help.contains("omponent ID"), "{args:?}:\n{help}");
    }
    let context = run_help(&["context"]);
    assert!(
        context.contains("for a component") && context.contains("ancestors"),
        "{context}"
    );
    let claim = run_help(&["claim"]);
    assert!(claim.contains("impl/<component>"), "{claim}");
    let workaround = run_help(&["workaround"]);
    assert!(
        workaround.contains("backlog/workaround-<component>-<slug>-<n>.md")
            && workaround.contains("<component>/<slug>"),
        "{workaround}"
    );
}

#[test]
fn readme_scopes_context_and_contracts_to_components() {
    // req:clm/says-component@3db77145
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let readme = fs::read_to_string(root.join("README.md")).unwrap();
    let hits = check("README.md", &readme, 1);
    assert!(hits.is_empty(), "old words:\n{}", report(&hits));
    assert!(
        readme.contains("component and its ancestors"),
        "the context a build gets"
    );
    assert!(readme.contains("per-component contracts"), "contracts");
}

#[test]
fn the_messages_in_the_source_say_component() {
    // req:clm/says-component@3db77145
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    files_under(&src, &mut files);
    assert!(!files.is_empty());
    let mut hits = Vec::new();
    for file in files {
        if file.extension().is_none_or(|e| e != "rs") {
            continue;
        }
        let text = fs::read_to_string(&file).unwrap();
        let rel = format!(
            "crates/clm/src/{}",
            file.strip_prefix(&src).unwrap().to_string_lossy()
        );
        for (line, literal) in literals(&text) {
            for hit in check(&rel, &literal, line) {
                // A hidden flag's attribute keeps the old name as the exact literal.
                let exempt = ["node", "nodes"].contains(&literal.as_str());
                if !exempt {
                    hits.push(hit);
                }
            }
        }
    }
    assert!(hits.is_empty(), "old words in literals:\n{}", report(&hits));
}

#[test]
fn an_unknown_component_is_named_in_the_messages() {
    // req:clm/says-component@3db77145
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("specs")).unwrap();
    fs::write(
        root.join("specs/SPEC.md"),
        "---\ntitle: T\nstatus: active\nowners: []\n---\n\n## Contract\nc\n",
    )
    .unwrap();
    let run = |args: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(args)
            .arg("--root")
            .arg(root)
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8(out.stderr).unwrap() + &String::from_utf8(out.stdout).unwrap(),
        )
    };
    let (code, text) = run(&["context", "zz"]);
    assert_eq!(code, Some(2));
    assert!(text.contains("clm context: no component `zz`"), "{text}");
    let (code, text) = run(&["hashes", "zz"]);
    assert_eq!(code, Some(2));
    assert!(
        text.contains("clm hashes: no valid component `zz`"),
        "{text}"
    );
    let (code, text) = run(&["size", "zz"]);
    assert_eq!(code, Some(1));
    assert!(text.contains("unknown component `zz`"), "{text}");
}
