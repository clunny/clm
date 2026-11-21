//! `clm affected` as `clm-affected`'s call, run on git repositories.

use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

/// What a call wrote and returned, shaped as a process's output so the tests moved from the
/// binary read it as they did.
#[derive(Debug)]
struct Output {
    status: Status,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

#[derive(Debug)]
struct Status(u8);

impl Status {
    fn code(&self) -> Option<i32> {
        Some(i32::from(self.0))
    }
}

/// `[affected] all` as `crates/clm` passes it: read from the repository's `clm.toml`, or the
/// settings' error as the line `crates/clm` formats.
fn settings(root: &Path, _env: &dyn Fn(&str) -> Option<String>) -> Result<Vec<String>, String> {
    let Ok(text) = fs::read_to_string(root.join("clm.toml")) else {
        return Ok(Vec::new());
    };
    let bad = |why: &str| format!("clm.toml: check.affected/affected-base: affected.all: {why}");
    let table: toml::Table = text.parse().map_err(|e| bad(&format!("{e}")))?;
    let Some(affected) = table.get("affected") else {
        return Ok(Vec::new());
    };
    let Some(all) = affected.get("all") else {
        return Ok(Vec::new());
    };
    all.as_array()
        .ok_or_else(|| bad("not a list of globs"))?
        .iter()
        .map(|glob| {
            glob.as_str()
                .map(str::to_string)
                .ok_or_else(|| bad("not a glob"))
        })
        .collect()
}

/// `clm affected <args>` called in-process, with `found` as the root `crates/clm` found.
fn call_in(found: &Path, args: &[&str]) -> Output {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let status = clm_affected::command(
        args.iter().copied(),
        found,
        &|_| None,
        &settings,
        &mut stdout,
        &mut stderr,
    );
    Output {
        status: Status(status),
        stdout,
        stderr,
    }
}

fn clm_affected(root: &Path, base: &str) -> Output {
    let root = root.to_str().unwrap();
    call_in(Path::new(root), &["--base", base, "--root", root])
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
fn the_call_writes_what_the_command_prints_and_returns_its_status() {
    // req:check.affected/commands-in-crate@5a7c0307
    // req:check.affected/clm-root-paths@de45727f
    // req:check.affected/affected-components@ce7d5c08
    // req:check.affected/affected-base@ec64a600
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
    assert_eq!(String::from_utf8(out.stderr).unwrap(), "");
    // A refusal goes to the error stream, with the command's prefix, and returns 1.
    let out = clm_affected(root, "nope");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    assert_eq!(
        String::from_utf8(out.stderr).unwrap(),
        "clm affected: unknown base nope\n"
    );
}

#[test]
fn the_root_found_is_used_unless_root_overrides_it() {
    // req:check.affected/commands-in-crate@6731de32
    let dir = repo("");
    let root = dir.path();
    write(root, "crates/d/src/lib.rs", "fn g() {}\n");
    commit_all(root, "d");
    let out = call_in(root, &["--base", "main"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "node d\nfile crates/d/src/lib.rs\n");
    // A found root that is no repository: `--root` overrides it.
    let elsewhere = TempDir::new().unwrap();
    let out = call_in(
        elsewhere.path(),
        &["--base", "main", "--root", root.to_str().unwrap()],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "node d\nfile crates/d/src/lib.rs\n");
}

#[test]
fn the_settings_are_loaded_with_the_environment_given() {
    // req:check.affected/commands-in-crate@6731de32
    let dir = repo("");
    let root = dir.path();
    write(root, "Cargo.lock", "lock 2\n");
    commit_all(root, "lock");
    let from_env = |_: &Path, env: &dyn Fn(&str) -> Option<String>| {
        Ok(env("CLM_AFFECTED_ALL").into_iter().collect::<Vec<_>>())
    };
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_affected::command(
        ["--base", "main"],
        root,
        &|name| (name == "CLM_AFFECTED_ALL").then(|| "Cargo.lock".to_string()),
        &from_env,
        &mut out,
        &mut err,
    );
    assert_eq!(status, 0, "{}", String::from_utf8_lossy(&err));
    assert_eq!(String::from_utf8_lossy(&out), "all Cargo.lock\n");
}

#[test]
fn arguments_it_cant_parse_return_2_with_the_usage_error() {
    // req:check.affected/commands-in-crate@6731de32
    let dir = repo("");
    let out = call_in(dir.path(), &["--nope"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(stdout(&out), "");
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("--nope"), "{err}");
    assert!(err.contains("Usage: clm affected"), "{err}");
}

#[test]
fn nothing_changed_prints_nothing() {
    // req:check.affected/affected-components@ce7d5c08
    let dir = repo("");
    let out = clm_affected(dir.path(), "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn uncommitted_untracked_and_deleted() {
    // req:check.affected/affected-files@4fca296e
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
    // req:check.affected/affected-all@a55dd83e
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
    // req:check.affected/affected-all@a55dd83e
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
    // req:check.affected/affected-all@a55dd83e
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
    // req:check.affected/affected-base@ec64a600
    let dir = repo("[affected]\nall = 3\n");
    let root = dir.path();
    let given = settings(root, &|_| None).unwrap_err();
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    // The caller's line, as it gave it, and nothing else.
    assert_eq!(String::from_utf8_lossy(&out.stderr), format!("{given}\n"));
}

#[test]
fn a_callers_settings_failure_is_written_as_given_whatever_the_question() {
    // req:check.affected/affected-base@ec64a600
    let dir = repo("");
    let root = dir.path();
    let path = root.to_str().unwrap();
    let failing = |_: &Path, _: &dyn Fn(&str) -> Option<String>| -> Result<Vec<String>, String> {
        Err("the caller's own line".to_string())
    };
    for args in [
        vec!["--base", "main", "--root", path],
        vec![
            "--base",
            "main",
            "--root",
            path,
            "--why",
            "crates/a/src/lib.rs",
        ],
        vec!["--overlap", "main", "work", "--root", path],
    ] {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = clm_affected::command(
            args.iter().copied(),
            root,
            &|_| None,
            &failing,
            &mut out,
            &mut err,
        );
        assert_eq!(status, 1, "{args:?}");
        assert_eq!(String::from_utf8_lossy(&out), "", "{args:?}");
        assert_eq!(
            String::from_utf8_lossy(&err),
            "the caller's own line\n",
            "{args:?}"
        );
    }
}

#[test]
fn doc_only_changes_print_nothing() {
    // req:check.affected/doc-only-selects-nothing@bb19731a
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
    // req:check.affected/doc-only-selects-nothing@bb19731a
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
    // req:check.affected/affected-base@ec64a600
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
    // req:check.affected/affected-base@ec64a600
    let dir = repo("");
    let root = dir.path();
    // `origin/main` is behind `main`, which already holds `b`'s change.
    git(root, &["update-ref", "refs/remotes/origin/main", "main"]);
    write(root, "crates/b/src/lib.rs", "fn g() {}\n");
    commit_all(root, "b");
    git(root, &["branch", "-f", "main"]);
    write(root, "crates/d/src/lib.rs", "fn g() {}\n");
    commit_all(root, "d");
    let out = call_in(root, &["--root", root.to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node b\nnode c\nnode d\nfile crates/b/src/lib.rs\nfile crates/c/src/lib.rs\nfile crates/d/src/lib.rs\n"
    );
}

#[test]
fn selects_through_the_markers_of_the_files_the_scan_reads() {
    // req:check.affected/affected-files@4fca296e
    let dir = repo("");
    let root = dir.path();
    // Cites `a`, so selected though `d` owns it.
    write(
        root,
        "crates/d/tests/cites.rs",
        concat!("// req", ":a/x@12345678\n#[test]\nfn t() {}\n"),
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
fn a_marker_outside_a_test_still_selects() {
    // req:check.affected/affected-files@4fca296e
    let dir = repo("");
    let root = dir.path();
    // The only marker is a file-header comment, on no line of a test.
    write(
        root,
        "crates/d/tests/t.ts",
        concat!(
            "// req",
            ":a/x\n\nimport { it } from 'vitest';\nit('t', () => {});\n"
        ),
    );
    commit_all(root, "files");
    git(root, &["branch", "-f", "main"]);
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        stdout(&out).contains("file crates/d/tests/t.ts\n"),
        "{}",
        stdout(&out)
    );
}

#[test]
fn a_git_failure_exits_2() {
    // req:check.affected/affected-base@ec64a600
    // No git repository, but `specs/` loads as a graph: git itself fails.
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    write(root, "specs/SPEC.md", &spec("[]", "[]"));
    // Git looks no higher than the directory, wherever the system's temporary directory is.
    // SAFETY: the tests run one per process (`ci/nextest-runner`), so no other thread reads
    // the environment meanwhile.
    unsafe { std::env::set_var("GIT_CEILING_DIRECTORIES", root.parent().unwrap()) };
    for args in [&["--base", "main"][..], &["--overlap", "x", "y"]] {
        let out = clm_affected_args(root, args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        assert_eq!(stdout(&out), "", "{args:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.starts_with("clm affected: "), "{err}");
        // The command that failed, and git's own error.
        assert!(err.contains("git rev-parse"), "{err}");
        assert!(err.contains("not a git repository"), "{err}");
    }
}

#[test]
fn a_base_with_no_merge_base_exits_1_naming_it() {
    // req:check.affected/affected-base@ec64a600
    let dir = repo("");
    let root = dir.path();
    git(root, &["switch", "-q", "--orphan", "lonely"]);
    commit_all(root, "lonely");
    git(root, &["switch", "-q", "work"]);
    let out = clm_affected(root, "lonely");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "clm affected: no merge base between lonely and HEAD\n"
    );
}

#[test]
fn an_invalid_graph_exits_1_with_the_error() {
    // req:check.affected/affected-base@ec64a600
    let dir = repo("");
    let root = dir.path();
    write(root, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[c]"));
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "cycle");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.starts_with("clm affected: "), "{err}");
    assert!(err.contains("cycle"), "{err}");
}

/// The marker's text, built so this file isn't itself a reader.
const READS: &str = concat!("clm", ":reads ");

fn clm_affected_with(root: &Path, base: &str, extra: &[&str]) -> Output {
    let root = root.to_str().unwrap();
    let mut args = vec!["--base", base, "--root", root];
    args.extend_from_slice(extra);
    call_in(Path::new(root), &args)
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
    // req:check.affected/affected-readers@e1298640
    // req:check.affected/doc-only-selects-nothing@bb19731a
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
    // req:check.affected/affected-readers@e1298640
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
    // req:check.affected/affected-readers@e1298640
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
fn a_deleted_reader_is_not_printed() {
    // req:check.affected/affected-readers@e1298640
    let dir = readers_repo();
    let root = dir.path();
    write(root, "ARCHITECTURE.md", "# B\n");
    commit_all(root, "doc");
    std::fs::remove_file(root.join("crates/e/tests/r.rs")).unwrap();
    let out = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn readers_alone_ignore_the_graph_and_the_settings() {
    // req:check.affected/affected-base@ec64a600
    // A dependency cycle and an `[affected] all` that is no list stop neither: the question
    // is only which tests read what changed.
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
fn readers_alone_read_no_setting() {
    // req:check.affected/affected-base@ec64a600
    let dir = readers_repo();
    let root = dir.path();
    write(root, "clm.toml", "[affected]\nall = 3\n");
    write(root, "ARCHITECTURE.md", "# B\n");
    commit_all(root, "doc and bad settings");
    let out = clm_affected_with(root, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stderr), "", "{out:?}");
    assert_eq!(stdout(&out), "file crates/e/tests/r.rs\n");
}

#[test]
fn readers_alone_with_owned_code_changed_and_a_reader_in_an_affected_component() {
    // req:check.affected/affected-readers@e1298640
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
    // req:check.affected/affected-readers@e1298640
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
    // req:check.affected/affected-readers@e1298640
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
    // req:check.affected/affected-readers@e1298640
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
    // req:check.affected/why-chain@c55b66f4
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
    // req:check.affected/why-chain@c55b66f4
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
    // req:check.affected/why-chain@c55b66f4
    let dir = repo("");
    let out = clm_affected_with(
        dir.path(),
        "main",
        &["--readers", "--why", "crates/a/src/lib.rs"],
    );
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = String::from_utf8(out.stderr.clone()).unwrap();
    assert!(err.contains("--readers") && err.contains("--why"), "{err}");
    // req:check.affected/affected-base@ec64a600
    assert!(err.starts_with("clm affected: "), "{err}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn why_follows_the_settings_glob_and_the_errors_of_the_selection() {
    // req:check.affected/why-chain@c55b66f4
    let dir = repo("[affected]\nall = [\"Cargo.lock\"]\n");
    let root = dir.path();
    write(root, "Cargo.lock", "lock 2\n");
    commit_all(root, "lock");
    let out = clm_affected_with(root, "main", &["--why", "crates/b/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "all: Cargo.lock\n");
    let out = clm_affected_with(root, "nope", &["--why", "crates/b/src/lib.rs"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    // req:check.affected/affected-base@ec64a600
    assert_eq!(stdout(&out), "");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "clm affected: unknown base nope\n"
    );
}

#[test]
fn why_reads_the_scan_for_readers_and_citations() {
    // req:check.affected/why-chain@c55b66f4
    let dir = readers_repo();
    let root = dir.path();
    write(root, "ARCHITECTURE.md", "# B\n");
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "both");
    let out = clm_affected_with(root, "main", &["--why", "crates/e/tests/r.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "crates/e/tests/r.rs reads ARCHITECTURE.md\n");
}

/// `clm affected <args>` in `root`.
fn clm_affected_args(root: &Path, args: &[&str]) -> Output {
    let root = root.to_str().unwrap();
    let mut args = args.to_vec();
    args.extend_from_slice(&["--root", root]);
    call_in(Path::new(root), &args)
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
fn a_rename_on_one_side_counts_as_its_old_path_and_its_new_one() {
    // req:check.affected/overlap-selection@f23b11bd
    // `x` moves `b`'s file into `a`, unchanged, which git would read as a rename.
    let dir = overlap_repo(
        "",
        "[]",
        &|r| {
            fs::rename(
                r.join("crates/b/src/lib.rs"),
                r.join("crates/a/src/lib2.rs"),
            )
            .unwrap()
        },
        &|r| {
            touch(r, "crates/a/src/lib.rs");
            touch(r, "crates/b/src/other.rs");
        },
    );
    let out = clm_affected_args(dir.path(), &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    // The new path makes `x` affect `a`, and the old one `b`.
    assert_eq!(
        stdout(&out),
        "node a\nnode b\nfile crates/a/src/lib.rs\nfile crates/a/tests/spec.rs\n"
    );
}

#[test]
fn independent_changes_overlap_in_nothing() {
    // req:check.affected/overlap-selection@f23b11bd
    let dir = overlap_repo("", "[]", &|r| touch(r, "crates/a/src/lib.rs"), &|r| {
        touch(r, "crates/b/src/lib.rs")
    });
    let out = clm_affected_args(dir.path(), &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
}

#[test]
fn a_dependant_of_both_is_what_they_share() {
    // req:check.affected/overlap-selection@f23b11bd
    // req:check.affected/clm-root-paths@de45727f
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
    // req:check.affected/overlap-selection@f23b11bd
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
    // req:check.affected/overlap-selection@f23b11bd
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
    // req:check.affected/overlap-selection@f23b11bd
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
    // req:check.affected/overlap-selection@f23b11bd
    let dir = overlap_repo("", "[]", &|_| {}, &|_| {});
    for flag in [&["--readers"][..], &["--why", "p"], &["--base", "main"]] {
        let mut args = vec!["--overlap", "x", "y"];
        args.extend_from_slice(flag);
        let out = clm_affected_args(dir.path(), &args);
        assert_eq!(out.status.code(), Some(1), "{flag:?}: {out:?}");
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("--overlap") && err.contains(flag[0]), "{err}");
        assert!(err.starts_with("clm affected: "), "{err}");
        assert_eq!(stdout(&out), "");
    }
}

#[test]
fn overlap_names_a_revision_that_is_no_commit_and_commits_with_no_base() {
    // req:check.affected/overlap-selection@f23b11bd
    let dir = overlap_repo("", "[]", &|_| {}, &|_| {});
    let root = dir.path();
    let out = clm_affected_args(root, &["--overlap", "x", "nope"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "clm affected: unknown revision nope\n"
    );
    git(root, &["switch", "-q", "--orphan", "z"]);
    commit_all(root, "unrelated");
    let out = clm_affected_args(root, &["--overlap", "x", "z"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(stdout(&out), "");
    assert_eq!(
        String::from_utf8_lossy(&out.stderr),
        "clm affected: no merge base between x and z\n"
    );
}

#[test]
fn overlap_has_no_file_line_for_a_deleted_file() {
    // req:check.affected/overlap-selection@f23b11bd
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
    // req:check.affected/overlap-selection@f23b11bd
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
    // req:check.affected/overlap-selection@f23b11bd
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

#[test]
fn an_exempt_test_is_selected_and_explained_whenever_a_component_is_affected() {
    // req:check.affected/affected-files@4fca296e
    // req:check.affected/why-chain@c55b66f4
    let dir = repo("");
    let root = dir.path();
    // `d` depends on nothing the change reaches, but exempts a test from `undeclared-imports`.
    write(
        root,
        "specs/d/SPEC.md",
        "---\ntitle: N\nstatus: active\nowners: []\npaths: [\"crates/d/**\"]\ndepends_on: []\nexemptions:\n  - rule: undeclared-imports\n    files: [\"crates/d/tests/t.rs\"]\n    until: backlog/x\n---\n",
    );
    write(root, "crates/d/tests/t.rs", "fn t() {}\n");
    write(root, "crates/d/tests/u.rs", "fn u() {}\n");
    commit_all(root, "exempt");
    git(root, &["branch", "-f", "main", "HEAD"]);
    // Nothing changed: no component is affected, so nothing is selected.
    assert_eq!(stdout(&clm_affected(root, "main")), "");
    write(root, "crates/a/src/lib.rs", "fn g() {}\n");
    commit_all(root, "a");
    let out = clm_affected(root, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let printed = stdout(&out);
    assert!(printed.contains("file crates/d/tests/t.rs\n"), "{printed}");
    assert!(!printed.contains("crates/d/tests/u.rs"), "{printed}");
    let out = clm_affected_with(root, "main", &["--why", "crates/d/tests/t.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "crates/d/tests/t.rs exempt from undeclared-imports\n"
    );
}

/// A repository on `main` whose CLM root is `sub/`: a root component and `a` owning
/// `crates/a/**`, with `sub/crates/a/src/lib.rs`, `sub/crates/a/src/util.rs` (no marker) and
/// `sub/crates/a/tests/r.rs` reading `ARCHITECTURE.md`; outside `sub/`, `other/build.sh` and
/// an `ARCHITECTURE.md` at the top of the git repository. Switched to a branch `work`.
fn subdirectory_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    git(top, &["init", "-q", "-b", "main"]);
    let sub = top.join("sub");
    write(&sub, "clm.toml", "");
    write(&sub, "specs/SPEC.md", &spec("[\"clm.toml\"]", "[]"));
    write(&sub, "specs/a/SPEC.md", &spec("[\"crates/a/**\"]", "[]"));
    write(&sub, "crates/a/src/lib.rs", "fn f() {}\n");
    write(&sub, "crates/a/src/util.rs", "fn u() {}\n");
    write(
        &sub,
        "crates/a/tests/r.rs",
        &format!("// {READS}ARCHITECTURE.md\n"),
    );
    write(&sub, "ARCHITECTURE.md", "# Sub\n");
    write(&sub, "crates/a/README.md", "# A\n");
    write(top, "other/build.sh", "true\n");
    write(top, "ARCHITECTURE.md", "# Top\n");
    // Outside the CLM root, a file citing `a` and reading `ARCHITECTURE.md`: no scan reads it.
    write(
        top,
        "other/t.rs",
        &format!(
            "// {}\n// {READS}ARCHITECTURE.md\n",
            concat!("req", ":a/x@12345678")
        ),
    );
    commit_all(top, "start");
    git(top, &["switch", "-q", "-c", "work"]);
    dir
}

#[test]
fn a_branch_in_a_subdirectory_root_is_named_from_the_root() {
    // req:check.affected/clm-root-paths@de45727f
    let dir = subdirectory_repo();
    let top = dir.path();
    let sub = top.join("sub");
    write(&sub, "crates/a/src/lib.rs", "fn g() {}\n");
    write(top, "other/build.sh", "false\n");
    commit_all(top, "change");
    // An untracked file outside the CLM root is no changed file either.
    write(top, "other/new.sh", "true\n");
    let out = clm_affected(&sub, "main");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node a\nfile crates/a/src/lib.rs\nfile crates/a/src/util.rs\nfile crates/a/tests/r.rs\n"
    );
    // `--why` names the paths from the CLM root too.
    let out = clm_affected_with(&sub, "main", &["--why", "crates/a/src/util.rs"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "crates/a/src/util.rs owned by a\na owns crates/a/src/lib.rs (changed)\n"
    );
}

#[test]
fn two_commits_in_a_subdirectory_root_are_read_from_its_place_in_their_trees() {
    // req:check.affected/clm-root-paths@de45727f
    let dir = subdirectory_repo();
    let top = dir.path();
    let sub = top.join("sub");
    git(top, &["switch", "-q", "-c", "x", "main"]);
    write(&sub, "crates/a/src/lib.rs", "fn g() {}\n");
    write(top, "other/build.sh", "false\n");
    commit_all(top, "x");
    git(top, &["switch", "-q", "-c", "y", "main"]);
    write(&sub, "crates/a/src/util.rs", "fn v() {}\n");
    commit_all(top, "y");
    let out = clm_affected_args(&sub, &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node a\nfile crates/a/src/lib.rs\nfile crates/a/src/util.rs\nfile crates/a/tests/r.rs\n"
    );
    // A changed Markdown file that is code, which no scan reads, is found in each commit's
    // tree by its name from the CLM root.
    for side in ["x", "y"] {
        git(top, &["switch", "-q", side]);
        write(&sub, "crates/a/README.md", &format!("# A, {side}\n"));
        commit_all(top, side);
    }
    let out = clm_affected_args(&sub, &["--overlap", "x", "y"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        stdout(&out),
        "node a\nfile crates/a/README.md\nfile crates/a/src/lib.rs\nfile crates/a/src/util.rs\nfile crates/a/tests/r.rs\n"
    );
}

#[test]
fn a_reader_in_a_subdirectory_root_reads_only_paths_under_it() {
    // req:check.affected/clm-root-paths@de45727f
    let dir = subdirectory_repo();
    let top = dir.path();
    let sub = top.join("sub");
    write(top, "ARCHITECTURE.md", "# Top, changed\n");
    commit_all(top, "top");
    let out = clm_affected_with(&sub, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "");
    // The root's own `ARCHITECTURE.md` is the one the marker names.
    write(&sub, "ARCHITECTURE.md", "# Sub, changed\n");
    let out = clm_affected_with(&sub, "main", &["--readers"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(stdout(&out), "file crates/a/tests/r.rs\n");
}
