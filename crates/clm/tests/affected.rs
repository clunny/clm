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
    // req:check.affected/affected-components@6fdb3e70
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
    // req:check.affected/affected-components@6fdb3e70
    let dir = repo("");
    let out = clm_affected(dir.path(), "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn uncommitted_untracked_and_deleted() {
    // req:check.affected/affected-files@f7c86d73
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
    // req:check.affected/doc-only-selects-nothing@4ded8e5f
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
    // req:check.affected/doc-only-selects-nothing@4ded8e5f
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
    // req:check.affected/affected-files@f7c86d73
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
