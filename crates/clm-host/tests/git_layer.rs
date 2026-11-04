//! `clm-host` runs `git` and `gh` only through `clm-git`; no crate but `clm-git`,
//! `clm-host` and `clm-guard` runs `gh`, and `clm-guard` runs `gh` once, for the
//! release guard's run lookup (host/git-through-layer).

use std::path::{Path, PathBuf};

/// Built from parts, so this file never holds the calls it looks for. The scenario's
/// `Command::new("git")` and `Command::new("gh")` are both caught by the first;
/// the second catches `Command` imported to be run under another name.
const CALLS: [&str; 2] = [concat!("Command", "::new"), concat!("process", "::Command")];

/// The crates that may call `gh`: the layer, this adapter and `clm-guard`, whose one call
/// `the_release_guard_lookup_only` checks.
const MAY_CALL_GH: [&str; 3] = ["clm-git", "clm-host", "clm-guard"];

#[test]
fn runs_processes_only_through_the_layer() {
    // req:host/git-through-layer@c7361079
    let krate = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest: toml::Table = std::fs::read_to_string(krate.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert!(
        manifest["dependencies"]
            .as_table()
            .unwrap()
            .contains_key("clm-git"),
        "clm-host doesn't list clm-git"
    );
    let files = rust_files(&krate.join("src"));
    assert!(!files.is_empty(), "no source files read");
    let found: Vec<_> = files
        .iter()
        .filter(|file| runs_process(&std::fs::read_to_string(file).unwrap()))
        .collect();
    assert!(found.is_empty(), "processes run outside clm-git: {found:?}");
}

#[test]
fn no_other_crate_calls_gh() {
    // req:host/git-through-layer@c7361079
    let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let (mut expected, mut scanned) = (0, 0);
    let mut found = Vec::new();
    for entry in std::fs::read_dir(crates).unwrap() {
        let dir = entry.unwrap().path();
        let name = dir.file_name().unwrap().to_string_lossy().into_owned();
        if MAY_CALL_GH.contains(&name.as_str()) || !dir.join("Cargo.toml").is_file() {
            continue;
        }
        expected += 1;
        let files = rust_files(&dir.join("src"));
        scanned += usize::from(!files.is_empty());
        for file in files {
            if calls_gh(&std::fs::read_to_string(&file).unwrap()) {
                found.push(file.display().to_string());
            }
        }
    }
    assert!(expected > 3, "only {expected} crates found");
    assert_eq!(scanned, expected, "a crate's sources weren't read");
    assert!(found.is_empty(), "calls gh outside the layer: {found:?}");
}

/// `text` with its whitespace removed, so a call rustfmt splits across lines still matches.
fn squeezed(text: &str) -> String {
    text.split_whitespace().collect()
}

/// Whether `text` runs a process itself.
fn runs_process(text: &str) -> bool {
    let text = squeezed(text);
    CALLS.iter().any(|call| text.contains(call))
}

/// The literal `"gh"`, built so this file doesn't hold it.
const GH: &str = concat!("\"g", "h\"");

/// Whether `c` can be part of an identifier.
fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether a crate outside the exempt ones runs `gh`: it names `clm-git`'s `gh`, or
/// holds a `"gh"` literal, whatever surrounds it.
fn calls_gh(text: &str) -> bool {
    let text = squeezed(text);
    names_gh(&text) || text.contains(GH)
}

/// How many `run(` or `run_within(` calls in `text`, with or without a path before them, have
/// the literal `"gh"` as their second top-level argument.
fn gh_runs(text: &str) -> usize {
    let text = squeezed(text);
    ["run(", "run_within("]
        .iter()
        .flat_map(|call| text.match_indices(call))
        .filter(|(at, _)| starts_call(&text[..*at]))
        .filter(|(at, call)| {
            let args = top_level_args(&text[at + call.len()..]);
            args.get(1).is_some_and(|arg| *arg == GH)
        })
        .count()
}

/// Whether a `run(` or `run_within(` right after the squeezed text `before` is a function
/// call: not a method (`.run(`) and not the end of a longer name (`prerun(`, `prerun_within(`,
/// `fn run(`), though a keyword such as `return` may run into it once whitespace is gone.
fn starts_call(before: &str) -> bool {
    const KEYWORDS: [&str; 6] = ["return", "match", "if", "while", "in", "break"];
    match before.chars().next_back() {
        Some('.') => false,
        Some(c) if ident(c) => KEYWORDS.iter().any(|word| {
            before
                .strip_suffix(word)
                .is_some_and(|rest| !rest.chars().next_back().is_some_and(ident))
        }),
        _ => true,
    }
}

/// The top-level arguments of the call whose list starts at `text`, just after its `(`:
/// split at commas outside brackets, strings and character literals, up to the closing `)`.
fn top_level_args(text: &str) -> Vec<&str> {
    let bytes = text.as_bytes();
    let (mut depth, mut start, mut i) = (0usize, 0, 0);
    let mut args = Vec::new();
    while i < bytes.len() {
        match bytes[i] {
            b'"' => {
                i += 1;
                while i < bytes.len() && bytes[i] != b'"' {
                    i += if bytes[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'\'' if bytes.get(i + 1) == Some(&b'\\') => {
                i += 3;
                while i < bytes.len() && bytes[i] != b'\'' {
                    i += 1;
                }
            }
            b'\'' if bytes.get(i + 2) == Some(&b'\'') => i += 2,
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' if depth == 0 => break,
            b')' | b']' | b'}' => depth -= 1,
            b',' if depth == 0 => {
                args.push(&text[start..i]);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    args.push(&text[start..i.min(text.len())]);
    args
}

/// Whether the squeezed `text` reaches `clm-git`'s `gh`: by the path `clm_git::gh`, or by
/// importing `gh`, a glob or the crate under another name with `use clm_git...;`.
fn names_gh(text: &str) -> bool {
    const PATH: &str = concat!("clm_git", "::gh");
    const USE: &str = "useclm_git";
    let named = text
        .match_indices(PATH)
        .any(|(at, _)| !text[at + PATH.len()..].chars().next().is_some_and(ident));
    let imported = text.match_indices(USE).any(|(at, _)| {
        let rest = &text[at + USE.len()..];
        let statement = &rest[..rest.find(';').unwrap_or(rest.len())];
        statement.starts_with("as")
            || statement.contains('*')
            || statement
                .split(|c: char| !ident(c))
                .any(|word| word == "gh")
    });
    named || imported
}

#[test]
fn the_release_guard_lookup_only() {
    // req:host/git-through-layer@c7361079
    let hooks = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("clm-guard/src");
    let files = rust_files(&hooks);
    assert!(!files.is_empty(), "no clm-guard source files read");
    let mut runs = 0;
    for file in &files {
        let text = std::fs::read_to_string(file).unwrap();
        let tight = squeezed(&text);
        assert!(!names_gh(&tight), "{} calls clm-git's gh", file.display());
        runs += gh_runs(&text);
    }
    assert_eq!(runs, 1, "clm-guard runs gh {runs} times");
}

#[test]
fn a_run_of_gh_is_caught() {
    // req:host/git-through-layer@c7361079
    let one_line = concat!("clm_git::run(&dir, \"g", "h\", &args, options)");
    let split = concat!(
        "let _ = tx.send(clm_git::run(\n",
        "    &dir,\n    \"g",
        "h\",\n    &args,\n    options,\n));"
    );
    let within = concat!(
        "clm_git::run_within(&dir, \"g",
        "h\", &args, options, limit)"
    );
    let within_split = concat!(
        "let _ = tx.send(clm_git::run_within(\n",
        "    &dir,\n    \"g",
        "h\",\n    &args,\n    options,\n    limit,\n));"
    );
    for text in [one_line, split, within, within_split] {
        assert!(calls_gh(text), "the matcher missed {text:?}");
        assert_eq!(gh_runs(text), 1, "the counter missed {text:?}");
    }
    assert_eq!(gh_runs(concat!("t == \"g", "h\"")), 0);
    for call in ["run", "run_within"] {
        let cases = [
            (format!("{call}(f(a, b), {GH})"), 1),
            (format!("{call}(dir, \"a,b\", {GH})"), 0),
            (format!("{call}(dir, \"git\", &[{GH}])"), 0),
            (format!("fn {call}(dir, {GH})"), 0),
            (format!("pre{call}(&dir, {GH})"), 0),
            (format!("tok.{call}(cmd, {GH})"), 0),
            (format!("return {call}(dir, {GH})"), 1),
            (format!("{call}(['\\'', '\"'], {GH})"), 1),
        ];
        for (text, runs) in cases {
            assert_eq!(gh_runs(&text), runs, "{text:?}");
        }
    }
    assert!(calls_gh(concat!("if t == \"g", "h\" {}")));
}

#[test]
fn the_matchers_see_split_renamed_and_globbed_calls() {
    assert!(calls_gh(concat!("let x = clm_git", "::gh(root, &[]);")));
    assert!(calls_gh(concat!("clm_git ", ":: gh(root, &[]);")));
    assert!(calls_gh(concat!(
        "use clm_git",
        "::{\n    git,\n    gh,\n};"
    )));
    assert!(calls_gh(concat!("use clm_git", " as g;")));
    assert!(calls_gh(concat!("use clm_git", "::*;")));
    assert!(calls_gh(concat!("clm_git::run(root, \"g", "h\", &[], o)")));
    assert!(!calls_gh(concat!("use clm_git", "::{git, ghost};")));
    assert!(!calls_gh(concat!("clm_git", "::gh_token()")));
    assert!(!calls_gh(concat!(
        "clm_git",
        "::run(root, \"git\", &[], o)"
    )));
    assert!(runs_process(concat!(
        "std::process::Command\n    ",
        "::new(\"gh\")"
    )));
    assert!(runs_process(concat!(
        "use std::{\n    process",
        "::Command,\n};"
    )));
    assert!(!runs_process(concat!("std::process", "::exit(1)")));
}

/// Every `.rs` file under `dir`, recursively; none when `dir` doesn't exist.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries {
        let path = entry.unwrap().path();
        if path.is_dir() {
            out.extend(rust_files(&path));
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
    out
}
