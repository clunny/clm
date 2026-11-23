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

/// `clm <args>` run as a process in `dir` with no user `config.toml` to load: `HOME` and
/// `XDG_CONFIG_HOME` name an empty directory, and `TMPDIR` another, whose paths (as given and
/// resolved) are returned beside its output.
fn clm_without_user_settings(dir: &Path, args: &[&str]) -> (Output, [String; 2]) {
    let home = TempDir::new().unwrap();
    let tmp = TempDir::new().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir)
        .env_clear()
        .env("PATH", std::env::var_os("PATH").unwrap())
        .env("HOME", home.path())
        .env("XDG_CONFIG_HOME", home.path())
        .env("TMPDIR", tmp.path())
        .output()
        .unwrap();
    let resolved = tmp.path().canonicalize().unwrap();
    (
        out,
        [tmp.path(), resolved.as_path()].map(|p| p.to_string_lossy().into_owned()),
    )
}

#[test]
fn scenario_settings_that_cant_be_loaded() {
    // req:clm/rebase-settings@01f11df1
    let (_dir, work) = node_a("[component_size]\nspec_tokens = 0\n");
    git(&work, &["checkout", "-q", "-b", "spec/x"]);
    let (out, tmp) = clm_without_user_settings(&work, &["rebase"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let first = stdout.lines().next().unwrap_or_default();
    let (source, rest) = first
        .split_once(": check.health/health-report: ")
        .unwrap_or_else(|| panic!("{out:?}"));
    // The source is the path of `clm.toml` in the repository, relative to its root.
    assert!(Path::new(source).is_relative(), "{out:?}");
    assert_eq!(
        work.join(source).canonicalize().unwrap(),
        work.join("clm.toml").canonicalize().unwrap(),
        "{out:?}"
    );
    assert!(rest.contains("component_size"), "{out:?}");
    assert!(tmp.iter().all(|t| !first.contains(t.as_str())), "{out:?}");
}

#[test]
fn scenario_the_repositorys_reconsider_threshold() {
    // req:clm/rebase-settings@01f11df1
    // Two flags reach the repository's threshold of 2, and not the default of 3.
    assert!(files_reconsider_item("[health]\nreconsider = 2\n"));
    assert!(!files_reconsider_item(""));
}

/// Whether `clm rebase` files a reconsider item for `a/x`, flagged twice at its version, on
/// `impl/a` with a committed `clm.toml` holding `clm_toml`.
fn files_reconsider_item(clm_toml: &str) -> bool {
    let (_dir, work) = node_a(clm_toml);
    write(
        &work,
        "specs/a/SPEC.md",
        "---\ntitle: A\nstatus: active\nowners: []\npaths: [\"src/a/**\"]\n---\n\n\
         ## Contract\nc\n\n## Requirements\n\n### Requirement: x\nIt SHALL x.\n\n\
         #### Scenario: s\n- ok\n",
    );
    let graph = clm_model::load(&work).0;
    let version = graph.requirement("a/x").unwrap().version().to_string();
    for n in 1..=2 {
        write(
            &work,
            &format!("backlog/workaround-a-x-{n}.md"),
            &format!(
                "---\ntitle: \"Hard {n}\"\nkind: workaround\ncomponents: [a]\n\
                 requirement: a/x@{version}\n---\n\nHard.\n"
            ),
        );
    }
    commit(&work, "two flags");
    let (out, _) = clm_without_user_settings(&work, &["rebase"]);
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .any(|l| l == "filed backlog/reconsider-a-x.md")
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
    // req:clm/old-command-names-retired@24a5a6f9
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
