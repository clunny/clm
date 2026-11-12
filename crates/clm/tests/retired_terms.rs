//! A term a completed rename retired stays out of the text CLM ships or works from
//! (clm/retired-terms): the scan is `scripts/retired-terms.sh`, and these tests run it.

// clm:reads specs/**
// clm:reads ARCHITECTURE.md
// clm:reads CAPABILITIES.md
// clm:reads CLAUDE.md
// clm:reads README.md
// clm:reads docs/guide/**
// clm:reads docs/spec-format.md
// clm:reads plugin/**
// clm:reads web/src/**
// clm:reads scripts/retired-terms.sh
// clm:reads scripts/retired-terms.txt

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The retired terms of the repository's list: every line that isn't blank or a comment.
fn retired() -> Vec<String> {
    let text = fs::read_to_string(repo_root().join("scripts/retired-terms.txt")).unwrap();
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// The scan's outcome: exit code, stdout and stderr.
struct Scan {
    code: i32,
    out: String,
    err: String,
}

/// Runs `scripts/retired-terms.sh args...` from `cwd`, as the check does from the root.
fn scan(cwd: &Path, args: &[&str]) -> Scan {
    let script = repo_root().join("scripts/retired-terms.sh");
    let out = Command::new("sh")
        .arg(script)
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    Scan {
        code: out.status.code().unwrap(),
        out: String::from_utf8(out.stdout).unwrap(),
        err: String::from_utf8(out.stderr).unwrap(),
    }
}

fn write(root: &Path, rel: &str, text: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// Runs the scan with a list holding `terms` (one per line) on `files` (path, text), written
/// into a fresh directory the scan runs in, and returns it.
fn scan_files(terms: &[&str], files: &[(&str, &str)]) -> Scan {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "list.txt", &format!("{}\n", terms.join("\n")));
    for (rel, text) in files {
        write(dir.path(), rel, text);
    }
    let paths: Vec<&str> = files.iter().map(|(p, _)| *p).collect();
    let mut args = vec!["--list", "list.txt"];
    args.extend(paths);
    scan(dir.path(), &args)
}

/// The terms of `terms` that are a bare word: no space, hyphen, underscore or backtick, and not
/// the coined `specd`. Plain English must not join the list on its own.
fn bare_words<'a>(terms: &[&'a str]) -> Vec<&'a str> {
    const COINED: &str = "specd";
    terms
        .iter()
        .copied()
        .filter(|t| *t != COINED && !t.contains([' ', '-', '_', '`']))
        .collect()
}

#[test]
fn a_retired_term_in_a_skill_is_reported() {
    // req:clm/retired-terms@b30c0e50
    let got = scan_files(
        &["grommet stub"],
        &[(
            "plugin/skills/x/SKILL.md",
            "fine\nmake it a   Grommet  Stub today\n",
        )],
    );
    assert_eq!(got.code, 1, "{}", got.err);
    assert_eq!(got.out, "plugin/skills/x/SKILL.md:2: grommet stub\n");

    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "list.txt", "grommet stub\n");
    write(
        dir.path(),
        "plugin/skills/x/SKILL.md",
        "make it a grommet stub\n",
    );
    let got = scan(dir.path(), &["--list", "list.txt", "plugin/skills/"]);
    assert_eq!(got.code, 1);
    assert_eq!(got.out, "plugin/skills/x/SKILL.md:1: grommet stub\n");
}

#[test]
fn a_line_marked_remove_by_is_exempt() {
    // req:clm/retired-terms@b30c0e50
    let text = concat!(
        "`clm frob` still runs `clm twiddle` // clm:remove",
        "-by 20261201-x\n`clm frob` here\n"
    );
    let got = scan_files(&["clm frob"], &[("docs/guide/a.md", text)]);
    assert_eq!(got.code, 1);
    assert_eq!(got.out, "docs/guide/a.md:2: clm frob\n");
    let exempt = scan_files(
        &["clm frob"],
        &[("docs/guide/a.md", &text[..text.find('\n').unwrap() + 1])],
    );
    assert_eq!((exempt.code, exempt.out.as_str()), (0, ""));
}

#[test]
fn history_is_left_alone() {
    // req:clm/retired-terms@b30c0e50
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "scripts/retired-terms.txt", "frobbed\n");
    for rel in [
        "changes/archive/20260101-a/change.md",
        "docs/decisions/d.md",
        "backlog/b.md",
        "changes/20261101-live/change.md",
    ] {
        write(dir.path(), rel, "frobbed\n");
    }
    let got = scan(dir.path(), &[]);
    assert_eq!((got.code, got.out.as_str()), (0, ""), "{}", got.err);
}

#[test]
fn every_named_path_is_read() {
    // req:clm/retired-terms@b30c0e50
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "scripts/retired-terms.txt", "frobbed\n");
    let named = [
        "docs/guide/a/b.md",
        "plugin/skills/x/SKILL.md",
        "specs/n/SPEC.md",
        "web/src/app/x.ts",
        "docs/spec-format.md",
        "ARCHITECTURE.md",
        "CAPABILITIES.md",
        "CLAUDE.md",
        "README.md",
    ];
    for rel in named {
        write(dir.path(), rel, "frobbed\n");
    }
    let got = scan(dir.path(), &[]);
    assert_eq!(got.code, 1, "{}", got.err);
    let mut lines: Vec<&str> = got.out.lines().collect();
    lines.sort_unstable();
    let mut want: Vec<String> = named.iter().map(|p| format!("{p}:1: frobbed")).collect();
    want.sort_unstable();
    assert_eq!(lines, want);
}

#[test]
fn only_whole_words_match() {
    // req:clm/retired-terms@b30c0e50
    let got = scan_files(
        &["frob"],
        &[("docs/guide/a.md", "frobnicate\nxfrob\nfrob_x\nfrob2\n")],
    );
    assert_eq!((got.code, got.out.as_str()), (0, ""));
    let got = scan_files(&["grommet stub"], &[("docs/guide/a.md", "grommetstub\n")]);
    assert_eq!((got.code, got.out.as_str()), (0, ""));
    let got = scan_files(
        &["frob"],
        &[("docs/guide/a.md", "a frob-scoped bundle\n(frob)\nFROB\n")],
    );
    assert_eq!(got.code, 1);
    assert_eq!(
        got.out,
        "docs/guide/a.md:1: frob\ndocs/guide/a.md:2: frob\ndocs/guide/a.md:3: frob\n"
    );
}

#[test]
fn a_multi_word_term_matches_across_spaces() {
    // req:clm/retired-terms@b30c0e50
    let got = scan_files(
        &["grommet stub"],
        &[("docs/guide/a.md", "GROMMET   stub\n")],
    );
    assert_eq!(got.code, 1);
    assert_eq!(got.out, "docs/guide/a.md:1: grommet stub\n");
}

#[test]
fn a_list_line_is_trimmed_and_comments_hold_none() {
    // req:clm/retired-terms@b30c0e50
    let dir = tempfile::tempdir().unwrap();
    write(
        dir.path(),
        "list.txt",
        "# a comment about frob\n\n  grommet \t stub \r\n--frob\n",
    );
    write(
        dir.path(),
        "a.md",
        "# a comment about frob\ngrommet stub\npass --frob\n",
    );
    let got = scan(dir.path(), &["--list", "list.txt", "a.md"]);
    assert_eq!(got.code, 1);
    assert_eq!(got.out, "a.md:2: grommet stub\na.md:3: --frob\n");
}

#[test]
fn hits_come_in_line_then_list_order() {
    // req:clm/retired-terms@b30c0e50
    let got = scan_files(
        &["beta", "alpha"],
        &[("b.md", "alpha beta\nbeta\n"), ("a.md", "alpha\n")],
    );
    assert_eq!(
        got.out,
        "b.md:1: beta\nb.md:1: alpha\nb.md:2: beta\na.md:1: alpha\n"
    );
}

#[test]
fn a_missing_list_is_refused() {
    // req:clm/retired-terms@b30c0e50
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "a.md", "text\n");
    let got = scan(dir.path(), &["--list", "nowhere.txt", "a.md"]);
    assert_eq!(got.code, 2);
    assert!(got.err.contains("nowhere.txt"), "{}", got.err);
    assert_eq!(got.out, "");
}

#[test]
fn no_plain_english_word_is_on_the_list() {
    // req:clm/retired-terms@b30c0e50
    assert_eq!(bare_words(&["clm frob", "--frob", "frob"]), ["frob"]);
    let terms = retired();
    let terms: Vec<&str> = terms.iter().map(String::as_str).collect();
    assert_eq!(bare_words(&terms), Vec::<&str>::new());
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

/// The help of `clm` and of every command and subcommand, each as a file in `dir` named for
/// its command with `-` between its words (`clm-land`). Returns the names written.
fn write_help(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
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
        let name = name.join("-");
        fs::write(dir.join(&name), text).unwrap();
        names.push(name);
    }
    names
}

#[test]
fn the_help_of_every_command_is_written_and_scanned() {
    // req:clm/retired-terms@b30c0e50
    let dir = tempfile::tempdir().unwrap();
    let names = write_help(dir.path());
    assert!(names.contains(&"clm".to_string()));
    assert!(names.contains(&"clm-land".to_string()));
    assert!(names.iter().any(|n| n.split('-').count() > 2), "{names:?}");
    // The scan reads a file of the directory.
    let list = dir.path().join("list.txt");
    fs::write(&list, "land\n").unwrap();
    let got = scan(dir.path(), &["--list", list.to_str().unwrap(), "clm-land"]);
    assert_eq!(got.code, 1, "{}", got.err);
}

#[test]
fn the_repository_holds_no_retired_term() {
    // req:clm/node-names-retired@2d40cbc7
    // req:clm/architecture-term-retired@dcff8eb7
    // req:clm/old-state-and-key-names-retired@381faa28
    // req:clm/retired-terms@b30c0e50
    let root = repo_root();
    let got = scan(&root, &[]);
    assert!(got.code == 0, "retired terms:\n{}{}", got.out, got.err);

    let dir = tempfile::tempdir().unwrap();
    write_help(dir.path());
    let got = scan(&root, &[dir.path().to_str().unwrap()]);
    assert!(
        got.code == 0,
        "retired terms in help:\n{}{}",
        got.out,
        got.err
    );
}

#[test]
fn the_old_command_names_are_on_the_list() {
    // req:clm/old-command-names-retired@94b2cc25
    let list = retired();
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
        assert!(list.iter().any(|t| t == term), "{term}");
    }
    // An old name after `clm` and a space is a hit; the new names are not.
    let got = scan_files(
        &list.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(
            "docs/guide/a.md",
            "run `clm sync`\nrun `clm rebase`\nit ends sync ok\n",
        )],
    );
    let lines: Vec<&str> = got.out.lines().collect();
    assert_eq!(
        lines,
        ["docs/guide/a.md:1: clm sync", "docs/guide/a.md:3: sync ok"]
    );
}

#[test]
fn the_old_architecture_term_is_on_the_list() {
    // req:clm/architecture-term-retired@dcff8eb7
    let list = retired();
    assert!(list.iter().any(|t| t == "architecture requirement"));
    assert!(list.iter().any(|t| t == "architecture requirements"));
    let got = scan_files(
        &list.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(
            "docs/guide/a.md",
            "the architecture requirement\nthe Architecture   Requirements\nan architectural constraint\n",
        )],
    );
    let lines: Vec<&str> = got.out.lines().collect();
    assert_eq!(
        lines,
        [
            "docs/guide/a.md:1: architecture requirement",
            "docs/guide/a.md:2: architecture requirements"
        ]
    );
}

#[test]
fn architecture_holds_constraint_blocks() {
    // req:clm/architecture-term-retired@dcff8eb7
    let text = fs::read_to_string(repo_root().join("ARCHITECTURE.md")).unwrap();
    assert!(text.contains("\n## Constraints\n"));
    assert!(text.contains("\n### Constraint: "));
    assert!(!text.contains("\n### Requirement: "));
}

#[test]
fn the_old_flags_and_words_are_on_the_list() {
    // req:clm/node-names-retired@2d40cbc7
    let list = retired();
    for term in [
        "--node",
        "--nodes",
        "node_size",
        "frontier stub",
        "frontier stubs",
    ] {
        assert!(list.iter().any(|t| t == term), "{term}");
    }
    // The new flag and the bare words for a component stay clear of the list.
    assert!(!list.iter().any(|t| t == "node" || t == "nodes"));
    assert!(
        !list.iter().any(|t| t == "spine"),
        "the old word for ancestors is plain English"
    );
    let got = scan_files(
        &list.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(
            "docs/guide/a.md",
            "pass --node x\npass --component x\nthe [node_size] table\na node and nodes\nthe spine and Frontier  Stub\n",
        )],
    );
    let lines: Vec<&str> = got
        .out
        .lines()
        .map(|l| &l[l.find(':').unwrap() + 1..])
        .collect();
    let numbers: Vec<&str> = lines.iter().map(|l| &l[..l.find(':').unwrap()]).collect();
    assert_eq!(numbers, ["1", "3", "5"]);
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
    // req:clm/node-names-retired@2d40cbc7
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
    // req:clm/old-state-and-key-names-retired@381faa28
    let list = retired();
    let forms = ["specd", "clm friction", "`kind: friction`", "`kind: human`"];
    for term in forms {
        assert!(list.iter().any(|t| t == term), "{term}");
    }
    for plain in ["burdens", "human", "friction"] {
        assert!(!list.iter().any(|t| t == plain), "{plain}");
    }
    let got = scan_files(
        &forms,
        &[(
            "docs/guide/a.md",
            "run `clm friction`\nit is specd\n`kind: human`\n`kind: human-task`\nhuman_burdens\nthe human lines\n`kind: friction`\n",
        )],
    );
    let mut lines: Vec<&str> = got
        .out
        .lines()
        .map(|l| l.split(':').nth(1).unwrap())
        .collect();
    lines.dedup();
    assert_eq!(lines, ["1", "2", "3", "7"]);
}

#[test]
fn the_plain_word_is_free() {
    // req:clm/old-state-and-key-names-retired@381faa28
    let list = retired();
    let got = scan_files(
        &list.iter().map(String::as_str).collect::<Vec<_>>(),
        &[(
            "docs/guide/a.md",
            "where most friction after install lies\nrun `clm friction` now\nuse `kind: friction` here\n",
        )],
    );
    let numbers: Vec<&str> = got
        .out
        .lines()
        .map(|l| l.split(':').nth(1).unwrap())
        .collect();
    assert_eq!(numbers, ["2", "3"]);
}

#[test]
fn binary_files_are_skipped() {
    // req:clm/retired-terms@b30c0e50
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "list.txt", "frob\n");
    fs::write(dir.path().join("b.bin"), b"frob\0binary").unwrap();
    let got = scan(dir.path(), &["--list", "list.txt", "b.bin"]);
    assert_eq!((got.code, got.out.as_str()), (0, ""), "{}", got.err);
}

#[test]
fn the_old_workaround_subcommand_is_refused() {
    // req:clm/old-state-and-key-names-retired@381faa28
    let old = ["fric", "tion"].concat();
    let err = clm_refuses(&[&old, "a/x", "--title", "x"]);
    assert!(err.contains("unrecognized subcommand"), "{err}");
    assert!(err.contains(&old), "{err}");
}
