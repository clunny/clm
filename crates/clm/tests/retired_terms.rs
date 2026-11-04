//! A term a completed rename retired stays out of the text CLM ships or works from
//! (clm/retired-terms).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The old names completed renames replaced (`plain-terms-renames`). A change that
/// completes a rename adds the names it retires.
const RETIRED: &[&str] = &[
    "clm sync",
    "clm checkpoint",
    "clm page",
    "sync ok",
    "sync incomplete",
    "sync failed",
    "checkpoint ok",
    "checkpoint incomplete",
    "architecture requirement",
    "architecture requirements",
    "frontier stub",
    "frontier stubs",
    "--node",
    "--nodes",
    "node_size",
    "spine",
    "clm friction",
    "`kind: human`",
    "specd",
    "friction",
];

#[derive(Debug, PartialEq)]
struct Hit {
    path: String,
    line: usize,
    term: String,
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `term` occurs in `line` as a whole word, ignoring case, with any run of
/// spaces between a multi-word term's words.
fn contains_term(line: &str, term: &str) -> bool {
    let line: Vec<char> = line.to_lowercase().chars().collect();
    let words: Vec<Vec<char>> = term
        .to_lowercase()
        .split_whitespace()
        .map(|w| w.chars().collect())
        .collect();
    if words.is_empty() {
        return false;
    }
    (0..line.len()).any(|start| {
        if start > 0 && is_word(line[start - 1]) {
            return false;
        }
        let mut at = start;
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                let gap = line[at..].iter().take_while(|c| **c == ' ').count();
                if gap == 0 {
                    return false;
                }
                at += gap;
            }
            if !line[at..].starts_with(word) {
                return false;
            }
            at += word.len();
        }
        at == line.len() || !is_word(line[at])
    })
}

/// Every retired term found in `files` (path and text), in file and line order.
fn check(terms: &[&str], files: &[(String, String)]) -> Vec<Hit> {
    let mut hits = Vec::new();
    for (path, text) in files {
        for (i, line) in text.lines().enumerate() {
            if line.contains("clm:remove-by") {
                continue;
            }
            for term in terms {
                if contains_term(line, term) {
                    hits.push(Hit {
                        path: path.clone(),
                        line: i + 1,
                        term: (*term).to_string(),
                    });
                }
            }
        }
    }
    hits
}

fn walk(dir: &Path, root: &Path, skip: &[&str], out: &mut Vec<(String, String)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for path in entries {
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if skip.contains(&rel.as_str()) {
            continue;
        }
        if path.is_dir() {
            walk(&path, root, skip, out);
        } else if let Ok(text) = fs::read_to_string(&path) {
            out.push((rel, text));
        }
    }
}

/// The files CLM ships or works from: history (`changes/archive/`, `docs/decisions/`,
/// `backlog/`) is never read.
fn gather(root: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for dir in ["docs/guide", "plugin", "specs", "changes", "web/src"] {
        walk(&root.join(dir), root, &["changes/archive"], &mut out);
    }
    for file in [
        "docs/spec-format.md",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "README.md",
    ] {
        if let Ok(text) = fs::read_to_string(root.join(file)) {
            out.push((file.to_string(), text));
        }
    }
    out
}

fn help(args: &[&str]) -> String {
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

/// The help of `clm` and of every command and subcommand, each as a pseudo-file.
fn gather_help() -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut queue: Vec<Vec<String>> = vec![Vec::new()];
    while let Some(path) = queue.pop() {
        let args: Vec<&str> = path.iter().map(String::as_str).collect();
        let text = help(&args);
        for sub in subcommands(&text) {
            let mut next = path.clone();
            next.push(sub);
            queue.push(next);
        }
        let mut name = vec!["clm".to_string()];
        name.extend(path);
        out.push((format!("{} --help", name.join(" ")), text));
    }
    out
}

fn files(items: &[(&str, &str)]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|(p, t)| ((*p).to_string(), (*t).to_string()))
        .collect()
}

#[test]
fn a_retired_term_in_a_skill_is_reported() {
    // req:clm/retired-terms@7ddad81b
    let fixture = files(&[(
        "plugin/skills/x/SKILL.md",
        "fine\nmake it a   Grommet  Stub today\n",
    )]);
    assert_eq!(
        check(&["grommet stub"], &fixture),
        vec![Hit {
            path: "plugin/skills/x/SKILL.md".into(),
            line: 2,
            term: "grommet stub".into()
        }]
    );
}

#[test]
fn a_line_marked_remove_by_is_exempt() {
    // req:clm/retired-terms@7ddad81b
    let text = concat!(
        "`clm frob` still runs `clm twiddle` // clm:remove",
        "-by 20261201-x\n`clm frob` here\n"
    );
    let fixture = files(&[("docs/guide/a.md", text)]);
    let hits = check(&["clm frob"], &fixture);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].line, 2);
}

#[test]
fn history_is_not_read() {
    // req:clm/retired-terms@7ddad81b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    for rel in [
        "changes/archive/20260101-a/change.md",
        "docs/decisions/d.md",
        "backlog/b.md",
        "docs/guide/g.md",
        "changes/20261101-live/change.md",
    ] {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "frobbed\n").unwrap();
    }
    let hits = check(&["frobbed"], &gather(root));
    let mut paths: Vec<&str> = hits.iter().map(|h| h.path.as_str()).collect();
    paths.sort();
    assert_eq!(
        paths,
        ["changes/20261101-live/change.md", "docs/guide/g.md"]
    );
}

#[test]
fn only_whole_words_match() {
    // req:clm/retired-terms@7ddad81b
    let fixture = files(&[("docs/guide/a.md", "frobnicate\nxfrob\nfrob_x\nfrob2\n")]);
    assert!(check(&["frob"], &fixture).is_empty());
    let fixture = files(&[("docs/guide/a.md", "grommetstub\n")]);
    assert!(check(&["grommet stub"], &fixture).is_empty());
    let fixture = files(&[("docs/guide/a.md", "a frob-scoped bundle\n(frob)\nFROB\n")]);
    let lines: Vec<usize> = check(&["frob"], &fixture).iter().map(|h| h.line).collect();
    assert_eq!(lines, [1, 2, 3]);
}

#[test]
fn the_help_of_every_command_is_read() {
    // req:clm/retired-terms@7ddad81b
    let all = gather_help();
    let paths: Vec<&str> = all.iter().map(|(p, _)| p.as_str()).collect();
    assert!(paths.contains(&"clm --help"));
    assert!(paths.contains(&"clm land --help"));
    assert!(paths.iter().any(|p| p.split(' ').count() > 3), "{paths:?}");
}

#[test]
fn the_repository_holds_no_retired_term() {
    // req:clm/node-names-retired@0f17de38
    // req:clm/architecture-term-retired@7e356236
    // req:clm/old-state-and-key-names-retired@ab16e190
    // req:clm/retired-terms@7ddad81b
    let root: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut all = gather(&root);
    all.extend(gather_help());
    let hits = check(RETIRED, &all);
    let report: Vec<String> = hits
        .iter()
        .map(|h| format!("{}:{}: {}", h.path, h.line, h.term))
        .collect();
    assert!(report.is_empty(), "retired terms:\n{}", report.join("\n"));
}

#[test]
fn every_named_path_is_read() {
    // req:clm/retired-terms@7ddad81b
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let named = [
        "docs/guide/a/b.md",
        "plugin/skills/x/SKILL.md",
        "specs/n/SPEC.md",
        "changes/c/change.md",
        "web/src/app/x.ts",
        "docs/spec-format.md",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "README.md",
    ];
    for rel in named {
        let path = root.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, "frobbed\n").unwrap();
    }
    let hits = check(&["frobbed"], &gather(root));
    let mut paths: Vec<&str> = hits.iter().map(|h| h.path.as_str()).collect();
    paths.sort();
    let mut want = named.to_vec();
    want.sort();
    assert_eq!(paths, want);
}

#[test]
fn the_old_command_names_are_on_the_list() {
    // req:clm/old-command-names-retired@bb2bff6f
    for term in [
        "clm sync",
        "clm checkpoint",
        "clm page",
        "sync ok",
        "sync incomplete",
        "sync failed",
        "checkpoint ok",
        "checkpoint incomplete",
    ] {
        assert!(RETIRED.contains(&term), "{term}");
    }
    // An old name after `clm` and a space is a hit; the new names are not.
    let fixture = files(&[(
        "docs/guide/a.md",
        "run `clm sync`\nrun `clm rebase`\nit ends sync ok\n",
    )]);
    let lines: Vec<usize> = check(RETIRED, &fixture).iter().map(|h| h.line).collect();
    assert_eq!(lines, [1, 3]);
}

#[test]
fn the_old_architecture_term_is_on_the_list() {
    // req:clm/architecture-term-retired@7e356236
    assert!(RETIRED.contains(&"architecture requirement"));
    assert!(RETIRED.contains(&"architecture requirements"));
    let fixture = files(&[(
        "docs/guide/a.md",
        "the architecture requirement\nthe Architecture   Requirements\nan architectural constraint\n",
    )]);
    let lines: Vec<usize> = check(RETIRED, &fixture).iter().map(|h| h.line).collect();
    assert_eq!(lines, [1, 2]);
}

#[test]
fn architecture_holds_constraint_blocks() {
    // req:clm/architecture-term-retired@7e356236
    let text =
        fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ARCHITECTURE.md"))
            .unwrap();
    assert!(text.contains("\n## Constraints\n"));
    assert!(text.contains("\n### Constraint: "));
    assert!(!text.contains("\n### Requirement: "));
}

#[test]
fn the_old_flags_and_words_are_on_the_list() {
    // req:clm/node-names-retired@0f17de38
    for term in [
        "--node",
        "--nodes",
        "node_size",
        "spine",
        "frontier stub",
        "frontier stubs",
    ] {
        assert!(RETIRED.contains(&term), "{term}");
    }
    // The new flag and the bare words for a component stay clear of the list.
    assert!(!RETIRED.contains(&"node") && !RETIRED.contains(&"nodes"));
    let fixture = files(&[(
        "docs/guide/a.md",
        "pass --node x\npass --component x\nthe [node_size] table\na node and nodes\nthe Spine and Frontier  Stub\n",
    )]);
    let lines: Vec<usize> = check(RETIRED, &fixture).iter().map(|h| h.line).collect();
    assert_eq!(lines, [1, 3, 5, 5]);
}

/// Runs `clm` in an empty directory, where clap refuses an unknown argument before
/// anything reads a repository.
fn clm_refuses(args: &[&str]) -> String {
    let tmp = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(tmp.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{args:?} was accepted");
    assert_eq!(
        fs::read_dir(tmp.path()).unwrap().count(),
        0,
        "{args:?} wrote"
    );
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn the_old_flags_and_subcommand_are_refused() {
    // req:clm/node-names-retired@0f17de38
    let node = ["--no", "de"].concat();
    let nodes = ["--no", "des"].concat();
    let err = clm_refuses(&["defect", "--title", "x", &node, "gate"]);
    assert!(err.contains("unexpected argument"), "{err}");
    let err = clm_refuses(&["proposals", &node, "gate"]);
    assert!(err.contains("unexpected argument"), "{err}");
    let err = clm_refuses(&["survey", &nodes, "map.yaml"]);
    assert!(err.contains("unexpected argument"), "{err}");
    let old = ["fric", "tion"].concat();
    let err = clm_refuses(&[&old, "a/x", "--title", "x"]);
    assert!(err.contains("unrecognized subcommand"), "{err}");
}

#[test]
fn the_old_state_and_key_names_are_on_the_list() {
    // req:clm/old-state-and-key-names-retired@ab16e190
    for term in ["specd", "friction", "clm friction", "`kind: human`"] {
        assert!(RETIRED.contains(&term), "{term}");
    }
    assert!(!RETIRED.contains(&"burdens"));
    assert!(!RETIRED.contains(&"human"));
    let full = ["specd", "friction", "clm friction", "`kind: human`"];
    let fixture = files(&[(
        "docs/guide/a.md",
        "run `clm friction`\nit is specd\n`kind: human`\n`kind: human-task`\nhuman_burdens\nthe human lines\n",
    )]);
    let hits = check(&full, &fixture);
    let mut lines: Vec<usize> = hits.iter().map(|h| h.line).collect();
    lines.dedup();
    assert_eq!(lines, [1, 2, 3]);
}

#[test]
fn the_old_workaround_subcommand_is_refused() {
    // req:clm/old-state-and-key-names-retired@ab16e190
    let old = ["fric", "tion"].concat();
    let err = clm_refuses(&[&old, "a/x", "--title", "x"]);
    assert!(err.contains("unrecognized subcommand"), "{err}");
    assert!(err.contains(&old), "{err}");
}
