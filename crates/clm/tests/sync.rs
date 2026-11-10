//! `clm rebase` and `clm autosync` as commands: what they print and how they exit.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

use tempfile::TempDir;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(out.status.success(), "git {args:?}: {out:?}");
}

fn write(dir: &Path, rel: &str, text: &str) {
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

fn commit(dir: &Path, msg: &str) {
    git(dir, &["add", "-A"]);
    git(dir, &["commit", "-qm", msg]);
}

fn head(dir: &Path) -> String {
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap()
}

fn clm(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .output()
        .unwrap()
}

/// A work clone of a bare `origin`, on branch `pr`, where `pr` and `origin/main` both
/// changed `src/lib.rs`.
fn conflicting() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let (origin, work) = (dir.path().join("origin"), dir.path().join("work"));
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        dir.path(),
        &["clone", "-q", origin.to_str().unwrap(), "work"],
    );
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    write(
        &work,
        "specs/SPEC.md",
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    );
    write(&work, "src/lib.rs", "one\n");
    commit(&work, "base");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    write(&work, "src/lib.rs", "theirs\n");
    commit(&work, "main moves");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", "pr", "HEAD~1"]);
    write(&work, "src/lib.rs", "mine\n");
    commit(&work, "work");
    (dir, work)
}

#[test]
fn sync_prints_what_is_left_and_exits_1_then_0() {
    // req:sync/rebase@3ba762e9 req:sync/outcome-line@45f1fe6d
    let (_dir, work) = conflicting();
    write(&work, "src/lib.rs", "edited\n");
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "uncommitted src/lib.rs\nrebase incomplete\n"
    );
    git(&work, &["checkout", "-q", "--", "src/lib.rs"]);

    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "conflict src/lib.rs\nrebase incomplete\n"
    );

    write(&work, "src/lib.rs", "both\n");
    git(&work, &["add", "src/lib.rs"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "rebase ok\n",
        "{out:?}"
    );
}

#[test]
fn sync_refusing_exits_1_and_git_failing_exits_2() {
    // req:sync/rebase@3ba762e9 req:sync/outcome-line@45f1fe6d
    let (dir, work) = conflicting();
    git(&work, &["checkout", "-q", "main"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("main"),
        "{out:?}"
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "rebase incomplete\n",
        "{out:?}"
    );

    // An `origin` that isn't there: the fetch fails.
    git(&work, &["checkout", "-q", "pr"]);
    let gone = dir.path().join("gone");
    git(
        &work,
        &["remote", "set-url", "origin", gone.to_str().unwrap()],
    );
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        "rebase failed\n",
        "{out:?}"
    );

    let elsewhere = dir.path().join("not-a-repo");
    fs::create_dir_all(&elsewhere).unwrap();
    assert_eq!(clm(&elsewhere, &["rebase"]).status.code(), Some(2));
}

#[test]
fn a_refused_sync_keeps_its_output() {
    // req:sync/shared-plumbing@0453c1dd
    let (_dir, work) = conflicting();
    git(&work, &["checkout", "-q", "main"]);
    let out = clm(&work, &["rebase"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("clm rebase: "), "{out:?}");
    assert!(stderr.len() > "clm rebase: \n".len(), "no reason: {out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(stdout.lines().last(), Some("rebase incomplete"), "{out:?}");
}

#[test]
fn autosync_exits_0_off_a_spec_branch() {
    // req:sync/autosync@9720eb37
    let (_dir, work) = conflicting();
    let out = clm(&work, &["autosync"]);
    assert_eq!(
        (out.status.code(), out.stdout.len()),
        (Some(0), 0),
        "{out:?}"
    );
}

/// A work clone of a bare `origin`, on branch `impl/a`, whose node `a` owns `src/a/x.rs` and
/// whose `clm.toml` holds `clm_toml`, all on `origin/main`.
fn node_a(clm_toml: &str) -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().unwrap();
    let (origin, work) = (dir.path().join("origin"), dir.path().join("work"));
    fs::create_dir_all(&origin).unwrap();
    git(&origin, &["init", "-q", "--bare", "-b", "main"]);
    git(
        dir.path(),
        &["clone", "-q", origin.to_str().unwrap(), "work"],
    );
    git(&work, &["config", "user.name", "t"]);
    git(&work, &["config", "user.email", "t@t"]);
    write(&work, "clm.toml", clm_toml);
    write(
        &work,
        "specs/SPEC.md",
        "---\ntitle: Root\nstatus: active\nowners: []\n---\n\n## Contract\nr\n",
    );
    write(
        &work,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n\n\
         ## Contract\nc\n",
    );
    write(&work, "src/a/x.rs", "a\n");
    commit(&work, "base");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "-b", "impl/a"]);
    (dir, work)
}

/// `clm rebase` in `dir`, with no user settings and `TMPDIR` an empty directory of its own,
/// which it asserts is empty again afterwards; its stdout.
fn rebase_without_user_settings(dir: &Path) -> (Option<i32>, String) {
    let tmp = TempDir::new().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .env(
            "XDG_CONFIG_HOME",
            Path::new(env!("CARGO_TARGET_TMPDIR")).join("no-user-config"),
        )
        .env("TMPDIR", tmp.path())
        .arg("rebase")
        .current_dir(dir)
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let left: Vec<_> = fs::read_dir(tmp.path()).unwrap().collect();
    assert!(left.is_empty(), "TMPDIR holds {left:?}: {stdout}");
    (
        out.status.code(),
        stdout.replace(&*tmp.path().to_string_lossy(), "<TMPDIR>"),
    )
}

#[test]
fn sync_leaves_no_temporary_worktree() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("");
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    write(&work, "src/a/t.rs", concat!("// req", ":a/missing\n"));
    commit(&work, "a violation");

    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!(code, Some(1), "{stdout}");
    assert!(stdout.contains("check/dangling-markers"), "{stdout}");
    let list = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(&work)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&list.stdout).lines().count(), 1);
}

#[test]
fn sync_ignores_an_uncommitted_settings_edit() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("");
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    git(&work, &["checkout", "-q", "main"]);
    write(&work, "notes.md", "main\n");
    commit(&work, "main moves");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "spec/x"]);
    write(&work, "clm.toml", "no_such_key = 1\n");

    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!((code, stdout.as_str()), (Some(0), "rebase ok\n"));
    assert_eq!(
        fs::read_to_string(work.join("clm.toml")).unwrap(),
        "no_such_key = 1\n"
    );
}

#[test]
fn sync_with_settings_that_cant_be_loaded() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("[component_size]\nspec_tokens = 0\n");
    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!(code, Some(1), "{stdout}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    assert!(lines[0].contains("clm.toml: "), "{stdout}");
    assert!(lines[0].contains("component_size"), "{stdout}");
    assert!(lines[0].contains("check.health/health-report"), "{stdout}");
    assert_eq!(lines[1], "rebase incomplete");
}

#[test]
fn sync_fails_on_settings_main_brought_in_that_cant_be_loaded() {
    // req:sync/judged-by-check@bfefb55e
    let (_dir, work) = node_a("");
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    write(&work, "notes.md", "mine\n");
    commit(&work, "work");
    // `main` gains a key this `clm` doesn't know after the branch was cut.
    git(&work, &["checkout", "-q", "main"]);
    write(&work, "clm.toml", "no_such_key = 1\n");
    commit(&work, "new key");
    git(&work, &["push", "-q", "origin", "HEAD:main"]);
    git(&work, &["checkout", "-q", "spec/x"]);

    let (code, stdout) = rebase_without_user_settings(&work);
    assert_eq!(code, Some(1), "{stdout}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert!(
        lines[0].contains("clm.toml: ") && lines[0].contains("no_such_key"),
        "{stdout}"
    );
    // Named by its path in the repository, no part of it under the temporary directory.
    assert!(!lines[0].contains("<TMPDIR>"), "{stdout}");
    assert_eq!(lines.last(), Some(&"rebase incomplete"), "{stdout}");
    assert_eq!(
        fs::read_to_string(work.join("clm.toml")).unwrap(),
        "no_such_key = 1\n",
        "the branch is rebased"
    );
}

#[test]
fn sync_files_a_reconsider_item_at_the_repository_threshold() {
    // req:sync/file-reconsider-item@1f67b16b
    let (_dir, work) = node_a("[health]\nreconsider = 2\n");
    write(
        &work,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n\n\
         ## Contract\nc\n\n## Requirements\n\n### Requirement: r\nIt SHALL r.\n\n\
         #### Scenario: s\n- ok\n",
    );
    let graph = clm_model::load(&work).0;
    let version = graph.requirement("a/r").unwrap().version().to_string();
    for n in 1..=2 {
        write(
            &work,
            &format!("backlog/friction-a-r-{n}.md"),
            &format!(
                "---\ntitle: \"Hard {n}\"\nkind: friction\ncomponents: [a]\n\
                 requirement: a/r@{version}\n---\n\nHard.\n"
            ),
        );
    }
    commit(&work, "two flags");

    // Two flags reach the repository's threshold of 2, under the default of 3.
    assert_eq!(
        rebase_without_user_settings(&work),
        (
            Some(0),
            "filed backlog/reconsider-a-r.md\nrebase ok\n".into()
        ),
    );
}

#[test]
fn scenario_a_skipped_proposals_warning_names_the_command_as_run() {
    // req:clm/rebase-warnings-prefixed@8240fcd8
    {
        let name = "rebase";
        let (dir, work) = node_a("");
        let ledger = dir.path().join("ledger");
        fs::create_dir_all(&ledger).unwrap();
        git(dir.path(), &["init", "-q", ledger.to_str().unwrap()]);
        git(&ledger, &["config", "user.name", "l"]);
        git(&ledger, &["config", "user.email", "l@l"]);
        let record = serde_json::json!({
            "id": "p9",
            "kind": "proposed",
            "writer": "alice/web",
            "at": "2026-10-28T10:00:00.000Z",
            "slug": "Not Kebab",
            "title": "Bad",
            "body": "x",
            "nodes": ["a"],
        });
        write(&ledger, "records.jsonl", &format!("{record}\n"));
        commit(&ledger, "records");
        let origin = dir.path().join("origin");
        git(
            &ledger,
            &[
                "push",
                "-q",
                origin.to_str().unwrap(),
                "HEAD:refs/heads/clm/ledger/alice/web",
            ],
        );

        let out = clm(&work, &[name]);
        let stderr = String::from_utf8_lossy(&out.stderr);
        let line = stderr
            .lines()
            .find(|l| l.contains("skipped proposal"))
            .unwrap_or_else(|| panic!("no warning under {name}: {out:?}"));
        assert!(
            line.starts_with(&format!("clm {name}: skipped proposal ")) && line.contains("p9"),
            "{line}"
        );
    }
}

#[test]
fn scenario_rebase_by_its_new_name() {
    // req:clm/renamed-commands@164ad075
    {
        let name = "rebase";
        let (dir, work) = conflicting();
        write(&work, "src/lib.rs", "edited\n");
        let out = clm(&work, &[name]);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            format!("uncommitted src/lib.rs\n{name} incomplete\n")
        );
        git(&work, &["checkout", "-q", "--", "src/lib.rs"]);
        let out = clm(&work, &[name]);
        assert_eq!(out.status.code(), Some(1), "{out:?}");
        write(&work, "src/lib.rs", "both\n");
        git(&work, &["add", "src/lib.rs"]);
        let out = clm(&work, &[name]);
        assert_eq!(out.status.code(), Some(0), "{out:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            format!("{name} ok\n"),
            "{out:?}"
        );
        // A failure names the command as it was run.
        let elsewhere = dir.path().join("not-a-repo");
        fs::create_dir_all(&elsewhere).unwrap();
        let out = clm(&elsewhere, &[name]);
        let prefix = format!("clm {name}: ");
        assert!(
            String::from_utf8_lossy(&out.stderr).starts_with(&prefix),
            "{out:?}"
        );
    }
}

#[test]
fn scenario_an_old_name_is_refused() {
    // req:clm/old-command-names-retired@94b2cc25
    let (_dir, work) = conflicting();
    let before = head(&work);
    for old in ["sync", "checkpoint", "page"] {
        let out = clm(&work, &[old]);
        assert_eq!(out.status.code(), Some(2), "{old}: {out:?}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("unrecognized subcommand"),
            "{old}: {stderr}"
        );
        assert!(stderr.contains(old), "{old}: {stderr}");
        assert!(out.stdout.is_empty(), "{old}: {out:?}");
    }
    assert_eq!(head(&work), before);
}
