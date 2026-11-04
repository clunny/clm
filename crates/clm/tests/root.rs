//! `--root` defaults to the CLM repository holding the current directory (`root-from-cwd`).
//! Fixtures live under the system temp directory: under `target/`, with no `.git` between
//! them and this repository, the search would find this repository's own `specs/SPEC.md`.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::fs;
use std::path::Path;
use std::process::Command;

use common::*;
use tempfile::TempDir;

/// `clm <args>` run in `cwd`: its exit code, stdout and stderr.
fn clm(cwd: &Path, args: &[&str]) -> (Option<i32>, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    (
        out.status.code(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

/// A git repository holding a CLM repository at `rel` (`""` for its top level) whose
/// node `a` has one requirement no test covers and a test citing one that doesn't exist,
/// so `clm check` has something to say.
fn clm_repo(top: &Path, rel: &str) {
    git(top, &["init", "-q", "-b", "main"]);
    let root = top.join(rel);
    write(&root, "specs/SPEC.md", &node("", &[]));
    write(&root, "specs/a/SPEC.md", &node("", &["p"]));
    write(
        &root,
        "tests/t.rs",
        &format!("// {}\n", concat!("req", ":a/missing")),
    );
    fs::create_dir_all(root.join("changes")).unwrap();
}

#[test]
fn from_a_subdirectory() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    clm_repo(top, "");
    let sub = top.join("changes");

    for cmd in ["check", "status"] {
        let at_top = clm(top, &[cmd]);
        assert!(!at_top.1.is_empty(), "{cmd}: {at_top:?}");
        assert_eq!(clm(&sub, &[cmd]), at_top, "{cmd}");
    }
}

/// A git repository whose CLM repository is `tool/`, with code outside it, all tracked, so
/// a survey of the CLM root and one of the git top level differ. Returns `tool/`.
fn nested_survey_repo(top: &Path) -> std::path::PathBuf {
    clm_repo(top, "tool");
    write(top, "src/main.rs", "fn main() {}\n");
    git(top, &["add", "-A"]);
    top.join("tool")
}

#[test]
fn survey_from_a_subdirectory() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let tool = nested_survey_repo(dir.path());

    let at_tool = clm(&tool, &["survey"]);
    assert_eq!(at_tool.0, Some(0), "{at_tool:?}");
    assert!(at_tool.1.contains("\ntests  rust 1 file"), "{at_tool:?}");
    assert!(!at_tool.1.contains("src"), "{at_tool:?}");
    assert_eq!(clm(&tool.join("changes"), &["survey"]), at_tool);
}

#[test]
fn survey_nodes_from_a_subdirectory() {
    // req:clm/root-from-cwd@839330d9
    // The node map covers `tool/`'s code only, so a survey of the git top would report
    // `src/main.rs`; the file is read relative to the directory it's given in.
    let dir = TempDir::new().unwrap();
    let tool = nested_survey_repo(dir.path());
    write(
        &tool,
        "nodes.yaml",
        "components:\n  - { id: t, paths: [\"tests/**\"] }\n",
    );

    let out = clm(
        &tool.join("changes"),
        &["survey", "--components", "../nodes.yaml"],
    );
    assert_eq!(out, (Some(0), String::new(), String::new()));
}

#[test]
fn survey_explicit_root_is_used_as_given() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    let tool = nested_survey_repo(top);

    let at_tool = clm(&tool, &["survey"]);
    assert_eq!(at_tool.0, Some(0), "{at_tool:?}");
    let given = clm(&tool.join("changes"), &["survey", "--root", "../.."]);
    assert_eq!(given.0, Some(0), "{given:?}");
    assert!(given.1.contains("\nsrc  rust 1 file"), "{given:?}");
    assert_eq!(given.1, clm(top, &["survey", "--root", "."]).1);
}

#[test]
fn init_takes_no_root() {
    // `clm init` works on the git top level, where there's no CLM repository yet.
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    git(top, &["init", "-q", "-b", "main"]);

    let (code, _, err) = clm(top, &["init", "--root", "."]);
    assert_eq!(code, Some(2), "{err}");
    assert!(err.contains("--root"), "{err}");
    assert!(!top.join("clm.toml").exists());
}

#[test]
fn a_bad_clm_toml_is_named_as_from_the_top() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    clm_repo(top, "");
    write(top, "clm.toml", "defect = \"off\"\n");

    // At the top the root is `.`, so the line is as it was before `root-from-cwd`.
    let at_top = clm(top, &["check"]);
    assert!(
        at_top
            .1
            .lines()
            .any(|l| l.starts_with("./clm.toml: check/detectors:")),
        "{at_top:?}"
    );
    assert_eq!(clm(&top.join("changes"), &["check"]), at_top);
}

#[test]
fn the_nearest_clm_repository_wins() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    clm_repo(top, "");
    // `tool/` is a CLM repository of its own inside the outer one, with no violations.
    write(&top.join("tool"), "specs/SPEC.md", &node("", &[]));
    let tool = top.join("tool");
    fs::create_dir_all(tool.join("changes")).unwrap();

    let (code, out, _) = clm(&tool.join("changes"), &["check"]);
    assert_eq!((code, out.as_str()), (Some(0), ""));
}

#[test]
fn the_search_stops_at_git() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    // A CLM repository above a git repository that isn't one: not this repository's.
    write(top, "specs/SPEC.md", &node("", &[]));
    let inner = top.join("inner");
    fs::create_dir_all(inner.join("src")).unwrap();
    git(&inner, &["init", "-q", "-b", "main"]);

    let src = inner.join("src");
    let found = clm(&src, &["check"]);
    assert_eq!(found.0, Some(1), "{found:?}");
    assert_eq!(found, clm(&src, &["check", "--root", "."]));
}

#[test]
fn explicit_root_is_used_as_given() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    clm_repo(top, "");
    let sub = top.join("changes");

    let at_top = clm(top, &["check"]);
    let given = clm(&sub, &["check", "--root", ".."]);
    assert_eq!((&given.0, &given.1), (&at_top.0, &at_top.1));
    // `--root .` in `changes/` is `changes/`, not the repository found from it.
    let here = clm(&sub, &["check", "--root", "."]);
    assert_ne!(here.1, at_top.1, "{here:?}");
}

#[test]
fn from_a_worktrees_subdirectory() {
    // req:clm/root-from-cwd@839330d9
    let r = repo();
    // A linked worktree on the shelf inside the main checkout, as `clm run` makes it.
    fs::write(r.work.join(".git/info/exclude"), ".clm/\n").unwrap();
    let wt = r.work.join(".clm/worktrees/a");
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/a",
            wt.to_str().unwrap(),
        ],
    );
    // The worktree's spec moves on: `a` gains a requirement the main checkout lacks.
    write(&wt, "specs/a/SPEC.md", &node("b", &["p", "q", "z"]));

    let at_top = clm(&wt, &["pending", "a"]);
    assert_eq!(at_top.0, Some(0), "{at_top:?}");
    assert!(at_top.1.contains("a/z"), "{at_top:?}");
    assert_eq!(clm(&wt.join("specs"), &["pending", "a"]), at_top);
    let main = clm(&r.work, &["pending", "a"]);
    assert_eq!(main.0, Some(0), "{main:?}");
    assert!(
        main.1.contains("a/p") && !main.1.contains("a/z"),
        "{main:?}"
    );
}

#[test]
fn the_search_stops_at_a_worktrees_git_file() {
    // req:clm/root-from-cwd@839330d9
    let r = repo();
    // A worktree inside the main checkout whose own tree holds no CLM repository: the
    // search must stop at its `.git` file, not climb on to the main checkout's spec.
    fs::write(r.work.join(".git/info/exclude"), ".clm/\n").unwrap();
    let wt = r.work.join(".clm/worktrees/a");
    git(
        &r.work,
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "impl/a",
            wt.to_str().unwrap(),
        ],
    );
    fs::remove_dir_all(wt.join("specs")).unwrap();
    let src = wt.join("src");
    fs::create_dir_all(&src).unwrap();

    let here = clm(&src, &["check", "--root", "."]);
    assert_ne!(here, clm(&r.work, &["check"]), "{here:?}");
    assert_eq!(clm(&src, &["check"]), here);
}

#[test]
fn inside_a_larger_repository() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    clm_repo(top, "tool");
    // A broken spec outside `tool/` that a check of the git repository's top would report.
    write(top, "specs/b/SPEC.md", "no frontmatter\n");

    let (code, out, _) = clm(&top.join("tool/changes"), &["check"]);
    assert_eq!(code, Some(1), "{out}");
    assert!(out.contains("a/missing"), "{out}");
    assert!(!out.contains("specs/b"), "{out}");
    assert_eq!(clm(&top.join("tool"), &["check"]).1, out);
}

#[test]
fn no_clm_repository() {
    // req:clm/root-from-cwd@839330d9
    let dir = TempDir::new().unwrap();
    let top = dir.path();
    git(top, &["init", "-q", "-b", "main"]);
    let src = top.join("src");
    // A node spec without a root makes `src/` report something the git top doesn't, so
    // falling back to any directory but the current one shows.
    write(&src, "specs/a/SPEC.md", "no frontmatter\n");

    let here = clm(&src, &["check", "--root", "."]);
    assert_ne!(here, clm(top, &["check", "--root", "."]));
    assert_eq!(clm(&src, &["check"]), here);
}
