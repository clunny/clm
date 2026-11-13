//! `clm affected`, run on git repositories.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn clm_affected(root: &Path, base: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(["affected", "--base", base, "--root"])
        .arg(root)
        .output()
        .unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn git(root: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn commit_all(root: &Path, message: &str) {
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "--allow-empty", "-m", message]);
}

const BACKLOG_ITEM: &str = "---\ntitle: Y\nkind: bug\ncomponents: [a]\n---\n\nY.\n";

const DECISION: &str = "---\ntitle: X\ndate: 2026-10-27\nstatus: accepted\n---\n\nX.\n";

fn spec(paths: &str, depends_on: &str) -> String {
    format!(
        "---\ntitle: N\nstatus: active\nowners: []\npaths: {paths}\ndepends_on: {depends_on}\n---\n"
    )
}

/// A repository on `main` with `clm_toml`: a root holding `Cargo.lock`, `a`, `b` on `a`,
/// `c` on `b`, `d` on nothing, each holding `crates/<id>/**` and a `src/lib.rs`, then
/// switched to a branch `work`.
fn repo(clm_toml: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    write(root, "clm.toml", clm_toml);
    write(
        root,
        "specs/SPEC.md",
        &spec("[\"Cargo.lock\", \"clm.toml\"]", "[]"),
    );
    write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[]"));
    write(root, "specs/b/SPEC.md", &spec("[\"crates/b/**\"]", "[a]"));
    write(root, "specs/c/SPEC.md", &spec("[\"crates/c/**\"]", "[b]"));
    write(root, "specs/d/SPEC.md", &spec("[\"crates/d/**\"]", "[]"));
    write(root, "Cargo.lock", "lock\n");
    for node in ["a", "b", "c", "d"] {
        write(root, &format!("crates/{node}/src/lib.rs"), "fn f() {}\n");
    }
    write(root, "tools/build.sh", "true\n");
    commit_all(root, "start");
    git(root, &["switch", "-q", "-c", "work"]);
    dir
}

#[test]
fn prints_the_changed_nodes_dependants_and_files() {
    // req:check.affected/affected-components@f4c3029d
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node a\nnode b\nnode c\nfile crates/a/src/lib.rs\nfile crates/b/src/lib.rs\nfile crates/c/src/lib.rs\n"
    );
}

#[test]
fn nothing_changed_prints_nothing() {
    // req:check.affected/affected-components@f4c3029d
    let dir = repo("");
    let out = clm_affected(dir.path(), "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn uncommitted_untracked_and_deleted() {
    // req:check.affected/affected-files@83ad81e2
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/a/src/old.rs", "fn o() {}\n");
    commit_all(root, "old");
    git(root, &["branch", "-f", "main"]);
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    write(root, "crates/a/src/new.rs", "fn n() {}\n");
    write(root, ".gitignore", "ignored.rs\n");
    write(root, "crates/d/ignored.rs", "fn i() {}\n");
    fs::remove_file(root.join("crates/a/src/old.rs")).unwrap();
    git(root, &["add", ".gitignore"]);
    git(root, &["commit", "-q", "-m", "ignore"]);
    // `.gitignore` is the root's, so commit it on main too, leaving the working tree changes.
    git(root, &["branch", "-f", "main"]);
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = stdout(&out);
    assert!(text.starts_with("node a\n"), "{text}");
    assert!(text.contains("file crates/a/src/lib.rs\n"), "{text}");
    assert!(text.contains("file crates/a/src/new.rs\n"), "{text}");
    assert!(!text.contains("old.rs"), "{text}");
    assert!(!text.contains("node d"), "{text}");
}

#[test]
fn a_file_no_node_holds_prints_all() {
    // req:check.affected/affected-all@c4a42acf
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    write(root, "tools/build.sh", "false\n");
    commit_all(root, "both");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "all tools/build.sh\n");
}

#[test]
fn the_settings_glob_prints_all() {
    // req:check.affected/affected-all@c4a42acf
    let dir = repo("[affected]\nall = [\"Cargo.lock\"]\n");
    let root = dir.path();
    write(root, "Cargo.lock", "lock 2\n");
    commit_all(root, "lock");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "all Cargo.lock\n");
}

#[test]
fn a_deleted_file_no_node_holds_prints_all() {
    // req:check.affected/affected-all@c4a42acf
    let dir = repo("");
    let root = dir.path();
    git(root, &["rm", "-q", "tools/build.sh"]);
    commit_all(root, "rm");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "all tools/build.sh\n");
}

#[test]
fn settings_that_cant_load_exit_1() {
    // req:check.affected/affected-all@c4a42acf
    let dir = repo("[affected]\nall = 3\n");
    let out = clm_affected(dir.path(), "main");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("clm.toml") && err.contains("affected"),
        "{err}"
    );
}

#[test]
fn doc_only_changes_print_nothing() {
    // req:check.affected/doc-only-selects-nothing@d8831f2e
    let dir = repo("");
    let root = dir.path();
    write(root, "docs/decisions/x.md", DECISION);
    write(root, "backlog/y.md", BACKLOG_ITEM);
    write(
        root,
        "specs/a/SPEC.md",
        &spec("[\"crates/a/**\"]", "[]").replace("N\n", "A\n"),
    );
    commit_all(root, "docs");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn doc_only_changes_beside_code_print_what_the_code_does() {
    // req:check.affected/doc-only-selects-nothing@d8831f2e
    let dir = repo("");
    let root = dir.path();
    write(root, "backlog/y.md", BACKLOG_ITEM);
    write(root, "crates/d/src/lib.rs", "fn g() {}\n");
    commit_all(root, "docs and d");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "node d\nfile crates/d/src/lib.rs\n");
}

#[test]
fn compares_with_the_merge_base() {
    // req:check.affected/affected-base@3d9c4e19
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    git(root, &["switch", "-q", "main"]);
    write(root, "crates/d/src/lib.rs", "fn g() {}\n");
    commit_all(root, "d");
    git(root, &["switch", "-q", "work"]);
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let text = stdout(&out);
    assert!(text.starts_with("node a\n"), "{text}");
    assert!(!text.contains("node d"), "{text}");
}

#[test]
fn the_base_defaults_to_origin_main() {
    // req:check.affected/affected-base@3d9c4e19
    let dir = repo("");
    let root = dir.path();
    // `origin/main` is behind `main`, which already holds `b`'s change.
    git(root, &["update-ref", "refs/remotes/origin/main", "main"]);
    write(root, "crates/b/src/lib.rs", "fn g() {}\n");
    commit_all(root, "b");
    git(root, &["branch", "-f", "main"]);
    write(root, "crates/d/src/lib.rs", "fn g() {}\n");
    commit_all(root, "d");
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(["affected", "--root"])
        .arg(root)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node b\nnode c\nnode d\nfile crates/b/src/lib.rs\nfile crates/c/src/lib.rs\nfile crates/d/src/lib.rs\n"
    );
}

#[test]
fn selects_through_the_markers_of_the_files_the_scan_reads() {
    // req:check.affected/affected-files@83ad81e2
    let dir = repo("");
    let root = dir.path();
    // Cites `a`, so selected though `d` owns it.
    write(
        root,
        "crates/d/tests/cites.rs",
        concat!("// req", ":a/x@12345678\n"),
    );
    // A removal marker isn't a `req:` marker: its owner `a` selects it.
    write(
        root,
        "crates/a/src/keep.rs",
        concat!("// clm:remove", "-by 20261027-x\n"),
    );
    // Markdown and non-UTF-8 files aren't read.
    write(root, "crates/a/README.md", "# A\n");
    fs::write(root.join("crates/a/blob.bin"), [0xff, 0xfe, 0x00]).unwrap();
    commit_all(root, "files");
    git(root, &["branch", "-f", "main"]);
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node a\nnode b\nnode c\nfile crates/a/src/keep.rs\nfile crates/a/src/lib.rs\nfile crates/b/src/lib.rs\nfile crates/c/src/lib.rs\nfile crates/d/tests/cites.rs\n"
    );
}

#[test]
fn a_git_failure_exits_2() {
    // req:check.affected/affected-base@3d9c4e19
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &spec("[]", "[]"));
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn an_unknown_base_exits_1_naming_it() {
    // req:check.affected/affected-base@3d9c4e19
    let dir = repo("");
    let out = clm_affected(dir.path(), "nope");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("nope"),
        "{out:?}"
    );
}

#[test]
fn a_base_with_no_merge_base_exits_1_naming_it() {
    // req:check.affected/affected-base@3d9c4e19
    let dir = repo("");
    let root = dir.path();
    git(root, &["switch", "-q", "--orphan", "lonely"]);
    commit_all(root, "lonely");
    git(root, &["switch", "-q", "work"]);
    let out = clm_affected(root, "lonely");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("lonely"),
        "{out:?}"
    );
}

#[test]
fn an_invalid_graph_exits_1_with_the_error() {
    // req:check.affected/affected-base@3d9c4e19
    let dir = repo("");
    let root = dir.path();
    write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[c]"));
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "cycle");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("cycle"),
        "{out:?}"
    );
}

/// The marker's text, built so this file isn't itself a reader.
const READS: &str = concat!("clm", ":reads ");

fn clm_affected_with(root: &Path, base: &str, extra: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .args(["affected", "--base", base, "--root"])
        .arg(root)
        .args(extra)
        .output()
        .unwrap()
}

/// `repo`, plus a component `e` that nothing depends on holding two readers, committed on
/// `main` and merged into `work`.
fn readers_repo() -> TempDir {
    let dir = repo("");
    let root = dir.path();
    git(root, &["switch", "-q", "main"]);
    write(root, "specs/e/SPEC.md", &spec("[\"crates/e/**\"]", "[]"));
    write(
        root,
        "crates/e/tests/r.rs",
        &format!("// {READS}ARCHITECTURE.md\n"),
    );
    write(
        root,
        "crates/e/tests/s.rs",
        &format!("// {READS}specs/**\n"),
    );
    write(root, "ARCHITECTURE.md", "# A\n");
    commit_all(root, "readers");
    git(root, &["switch", "-q", "work"]);
    git(root, &["merge", "-q", "main"]);
    dir
}

#[test]
fn a_reader_of_a_doc_only_file_is_printed_alone() {
    // req:check.affected/affected-readers@94edd10f
    // req:check.affected/doc-only-selects-nothing@d8831f2e
    let dir = readers_repo();
    let root = dir.path();
    write(root, "ARCHITECTURE.md", "# B\n");
    commit_all(root, "doc");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "file crates/e/tests/r.rs\n");
}

#[test]
fn readers_join_the_lines_of_a_code_change() {
    // req:check.affected/affected-readers@94edd10f
    let dir = readers_repo();
    let root = dir.path();
    write(root, "ARCHITECTURE.md", "# B\n");
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "both");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node a\nnode b\nnode c\nfile crates/a/src/lib.rs\nfile crates/b/src/lib.rs\nfile crates/c/src/lib.rs\nfile crates/e/tests/r.rs\n"
    );
}

#[test]
fn readers_alone_print_no_node_or_all_line() {
    // req:check.affected/affected-readers@94edd10f
    let dir = readers_repo();
    let root = dir.path();
    write(root, "ARCHITECTURE.md", "# B\n");
    write(root, "tools/build.sh", "false\n");
    commit_all(root, "doc and a file no component holds");
    let out = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "file crates/e/tests/r.rs\n");
}

#[test]
fn readers_alone_ignore_the_graph_and_the_settings() {
    // The Design's rule: the question is only which tests read what changed.
    let dir = readers_repo();
    let root = dir.path();
    write(root, "clm.toml", "[affected]\nall = 3\n");
    write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[c]"));
    write(root, "ARCHITECTURE.md", "# B\n");
    commit_all(root, "doc, a cycle and bad settings");
    let out = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "file crates/e/tests/r.rs\nfile crates/e/tests/s.rs\n"
    );
}

#[test]
fn readers_alone_with_owned_code_changed_and_a_reader_in_an_affected_component() {
    // req:check.affected/affected-readers@94edd10f
    let dir = readers_repo();
    let root = dir.path();
    // `a`'s reader sorts before the selected `crates/a/src/lib.rs`, and `a` is affected.
    write(
        root,
        "crates/a/src/aa.rs",
        &format!("// {READS}ARCHITECTURE.md\n"),
    );
    commit_all(root, "reader in a");
    git(root, &["branch", "-f", "main"]);
    write(root, "ARCHITECTURE.md", "# B\n");
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    let full = clm_affected(root, "main");
    assert_eq!(
        stdout(&full),
        "node a\nnode b\nnode c\nfile crates/a/src/aa.rs\nfile crates/a/src/lib.rs\nfile crates/b/src/lib.rs\nfile crates/c/src/lib.rs\nfile crates/e/tests/r.rs\n"
    );
    let only = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(
        stdout(&only),
        "file crates/a/src/aa.rs\nfile crates/e/tests/r.rs\n"
    );
}

#[test]
fn no_reader_matches() {
    // req:check.affected/affected-readers@94edd10f
    let dir = readers_repo();
    let root = dir.path();
    write(root, "backlog/y.md", BACKLOG_ITEM);
    write(root, "tools/build.sh", "false\n");
    commit_all(root, "unread");
    let out = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn a_marker_in_markdown_reads_nothing() {
    // req:check.affected/affected-readers@94edd10f
    let dir = repo("");
    let root = dir.path();
    write(root, "docs/notes.md", &format!("{READS}ARCHITECTURE.md\n"));
    write(root, "ARCHITECTURE.md", "# C\n");
    commit_all(root, "notes");
    let out = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn an_untracked_reader_and_an_untracked_doc_count() {
    // req:check.affected/affected-readers@94edd10f
    let dir = readers_repo();
    let root = dir.path();
    write(
        root,
        "crates/e/tests/u.rs",
        &format!("// {READS}docs/new.txt\n"),
    );
    write(root, "docs/new.txt", "x\n");
    let out = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "file crates/e/tests/u.rs\n");
}

#[test]
fn why_prints_the_chain_through_a_dependency() {
    // req:check.affected/why-chain@7e236f82
    // req:clm/affected-why-command@2b2b52c9
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    let out = clm_affected_with(root, "main", &["--why", "crates/b/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "crates/b/src/lib.rs owned by b\nb depends on a\na owns crates/a/src/lib.rs (changed)\n"
    );
    let out = clm_affected_with(root, "main", &["--why", "crates/a/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "crates/a/src/lib.rs (changed)\n");
}

#[test]
fn why_says_not_selected_and_all() {
    // req:check.affected/why-chain@7e236f82
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    let out = clm_affected_with(root, "main", &["--why", "crates/d/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "crates/d/src/lib.rs not selected\n");
    write(root, "tools/build.sh", "false\n");
    commit_all(root, "tool");
    let out = clm_affected_with(root, "main", &["--why", "crates/b/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "all: tools/build.sh\n");
}

#[test]
fn why_with_readers_exits_1_naming_both_flags() {
    // req:check.affected/why-chain@7e236f82
    // req:clm/affected-why-command@2b2b52c9
    let dir = repo("");
    let out = clm_affected_with(
        dir.path(),
        "main",
        &["--readers", "--why", "crates/a/src/lib.rs"],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(err.contains("--readers") && err.contains("--why"), "{err}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn why_follows_the_settings_glob_and_the_errors_of_the_selection() {
    // req:check.affected/why-chain@7e236f82
    let dir = repo("[affected]\nall = [\"Cargo.lock\"]\n");
    let root = dir.path();
    write(root, "Cargo.lock", "lock 2\n");
    commit_all(root, "lock");
    let out = clm_affected_with(root, "main", &["--why", "crates/b/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "all: Cargo.lock\n");
    let out = clm_affected_with(root, "nope", &["--why", "crates/b/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
}

#[test]
fn why_reads_the_scan_for_readers_and_citations() {
    // req:check.affected/why-chain@7e236f82
    let dir = readers_repo();
    let root = dir.path();
    write(root, "ARCHITECTURE.md", "# B\n");
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "both");
    let out = clm_affected_with(root, "main", &["--why", "crates/e/tests/r.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "crates/e/tests/r.rs reads ARCHITECTURE.md\n");
}

/// `clm affected <args>` in `root`, with no user config.
fn clm_affected_args(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .arg("affected")
        .args(args)
        .arg("--root")
        .arg(root)
        .output()
        .unwrap()
}

/// A repository whose `main` holds the root (with `Cargo.lock`), `a` and `b` on nothing,
/// `c` on `c_needs` with a test `crates/c/tests/t.rs`, and a reader of `specs/**` in `a`;
/// `x` and `y` are branches off it that each hold one commit, written by the two closures.
fn overlap_repo(clm_toml: &str, c_needs: &str, x: &dyn Fn(&Path), y: &dyn Fn(&Path)) -> TempDir {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    git(root, &["init", "-q", "-b", "main"]);
    write(root, "clm.toml", clm_toml);
    write(
        root,
        "specs/SPEC.md",
        &spec("[\"Cargo.lock\", \"clm.toml\"]", "[]"),
    );
    write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[]"));
    write(root, "specs/b/SPEC.md", &spec("[\"crates/b/**\"]", "[]"));
    write(root, "specs/c/SPEC.md", &spec("[\"crates/c/**\"]", c_needs));
    write(root, "Cargo.lock", "lock\n");
    for id in ["a", "b", "c"] {
        write(root, &format!("crates/{id}/src/lib.rs"), "fn f() {}\n");
    }
    write(root, "crates/c/tests/t.rs", "fn t() {}\n");
    write(
        root,
        "crates/a/tests/spec.rs",
        &format!("// {READS}specs/**\n"),
    );
    commit_all(root, "start");
    git(root, &["switch", "-q", "-c", "x"]);
    x(root);
    commit_all(root, "x");
    git(root, &["switch", "-q", "-c", "y", "main"]);
    y(root);
    commit_all(root, "y");
    dir
}

fn touch(root: &Path, rel: &str) {
    write(root, rel, "fn changed() {}\n");
}

#[test]
fn independent_changes_overlap_in_nothing() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo("", "[]", &|r| touch(r, "crates/a/src/lib.rs"), &|r| {
        touch(r, "crates/b/src/lib.rs")
    });
    let out = clm_affected_args(dir.path(), &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn a_dependant_of_both_is_what_they_share() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo("", "[a, b]", &|r| touch(r, "crates/a/src/lib.rs"), &|r| {
        touch(r, "crates/b/src/lib.rs")
    });
    let out = clm_affected_args(dir.path(), &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node c\nfile crates/c/src/lib.rs\nfile crates/c/tests/t.rs\n"
    );
}

#[test]
fn a_side_that_selects_everything_leaves_the_others_output() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo(
        "[affected]\nall = [\"Cargo.lock\"]\n",
        "[a, b]",
        &|r| write(r, "Cargo.lock", "lock 2\n"),
        &|r| touch(r, "crates/b/src/lib.rs"),
    );
    let root = dir.path();
    let out = clm_affected_args(root, &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    // What `--base main` prints with `y` checked out.
    let alone = clm_affected_args(root, &["--base", "main"]);
    assert_eq!(stdout(&out), stdout(&alone));
    assert!(stdout(&out).starts_with("node b\nnode c\n"));
    // The other way round.
    let out = clm_affected_args(root, &["--overlap", "y", "x"]);
    assert_eq!(stdout(&out), stdout(&alone));
}

#[test]
fn a_reader_on_one_side_meets_its_owner_on_the_other() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo(
        "",
        "[]",
        &|r| {
            write(
                r,
                "specs/a/SPEC.md",
                &spec("[\"crates/a/**\"]", "[]").replace("N\n", "A\n"),
            )
        },
        &|r| touch(r, "crates/a/src/lib.rs"),
    );
    // The reader is read from the commits, not the checkout.
    fs::remove_file(dir.path().join("crates/a/tests/spec.rs")).unwrap();
    let out = clm_affected_args(dir.path(), &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        stdout(&out).contains("file crates/a/tests/spec.rs\n"),
        "{out:?}"
    );
}

#[test]
fn overlap_takes_no_working_tree_or_untracked_file() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo("", "[a, b]", &|r| touch(r, "crates/a/src/lib.rs"), &|r| {
        touch(r, "crates/b/src/lib.rs")
    });
    let root = dir.path();
    write(root, "tools/untracked.sh", "true\n");
    write(root, "crates/c/src/lib.rs", "fn dirty() {}\n");
    let out = clm_affected_args(root, &["--overlap", "x", "y"]);
    assert_eq!(
        stdout(&out),
        "node c\nfile crates/c/src/lib.rs\nfile crates/c/tests/t.rs\n"
    );
}

#[test]
fn overlap_refuses_flags_that_dont_combine() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo("", "[]", &|_| {}, &|_| {});
    for flag in [&["--readers"][..], &["--why", "p"], &["--base", "main"]] {
        let mut args = vec!["--overlap", "x", "y"];
        args.extend_from_slice(flag);
        let out = clm_affected_args(dir.path(), &args);
        assert_eq!(out.status.code(), Some(1), "{flag:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("--overlap") && err.contains(flag[0]), "{err}");
        assert_eq!(stdout(&out), "");
    }
}

#[test]
fn overlap_names_a_revision_that_is_no_commit_and_commits_with_no_base() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo("", "[]", &|_| {}, &|_| {});
    let root = dir.path();
    let out = clm_affected_args(root, &["--overlap", "x", "nope"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown revision nope"));
    git(root, &["switch", "-q", "--orphan", "z"]);
    commit_all(root, "unrelated");
    let out = clm_affected_args(root, &["--overlap", "x", "z"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stderr).contains("no merge base between x and z"));
}

#[test]
fn overlap_has_no_file_line_for_a_deleted_file() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo(
        "",
        "[]",
        &|r| fs::remove_file(r.join("crates/c/src/lib.rs")).unwrap(),
        &|r| touch(r, "crates/c/src/other.rs"),
    );
    let out = clm_affected_args(dir.path(), &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "node c\nfile crates/c/tests/t.rs\n");
}

#[test]
fn overlap_with_shared_components_but_no_shared_file_prints_nothing() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo(
        "",
        "[]",
        &|r| fs::remove_file(r.join("crates/c/src/lib.rs")).unwrap(),
        &|r| fs::remove_file(r.join("crates/c/tests/t.rs")).unwrap(),
    );
    let out = clm_affected_args(dir.path(), &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn overlap_reads_the_graph_whole_from_the_commits() {
    // req:check.affected/overlap-selection@2171c658
    let dir = overlap_repo("", "[a, b]", &|r| touch(r, "crates/a/src/lib.rs"), &|r| {
        touch(r, "crates/b/src/lib.rs")
    });
    let root = dir.path();
    // A component serving an architecture entry loads only with `ARCHITECTURE.md` present.
    git(root, &["switch", "-q", "main"]);
    write(
        root,
        "ARCHITECTURE.md",
        "# A\n\n## Constraints\n\n### Constraint: local-rule\nThe tests SHALL pass.\n\n#### Scenario: s\n- GIVEN a\n- WHEN b\n- THEN c\n\n#### Untested\nx\n",
    );
    write(
        root,
        "specs/a/SPEC.md",
        "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"crates/a/**\"]\ndepends_on: []\nserves: [architecture/local-rule]\n---\n",
    );
    commit_all(root, "serves");
    git(root, &["switch", "-q", "-c", "x2", "main"]);
    touch(root, "crates/a/src/lib.rs");
    commit_all(root, "x2");
    git(root, &["switch", "-q", "-c", "y2", "main"]);
    touch(root, "crates/b/src/lib.rs");
    commit_all(root, "y2");
    let out = clm_affected_args(root, &["--overlap", "x2", "y2"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(stdout(&out).starts_with("node c\n"), "{out:?}");
}
