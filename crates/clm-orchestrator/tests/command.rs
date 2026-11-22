//! `clm next`, `clm claim` and `clm proposals` as `clm-orchestrator`'s calls
//! (`commands-in-crate`): what each writes to its streams and the status it returns.

mod common;

use std::fs;
use std::path::Path;

use common::*;
use tempfile::TempDir;

/// What one call wrote to each stream, and the status it returned.
type Output = (Option<i32>, String, String);

/// The settings `crates/clm` would pass when `max_builds` loads as 3, the default.
const DEFAULT_CAP: Result<u64, String> = Ok(3);

fn finish(status: u8, out: Vec<u8>, err: Vec<u8>) -> Output {
    (
        Some(i32::from(status)),
        String::from_utf8(out).unwrap(),
        String::from_utf8(err).unwrap(),
    )
}

/// `clm next` with `args`, in `root`, with `max_builds` as `crates/clm` loaded it.
fn next(root: &Path, args: &[&str], max_builds: Result<u64, String>) -> Output {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_orchestrator::next_command(
        args.iter().copied(),
        root,
        &|_| None,
        max_builds,
        &mut out,
        &mut err,
    );
    finish(status, out, err)
}

/// `clm claim` with `args`, in `root`.
fn claim(root: &Path, args: &[&str]) -> Output {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status =
        clm_orchestrator::claim_command(args.iter().copied(), root, &|_| None, &mut out, &mut err);
    finish(status, out, err)
}

/// `clm proposals` with `args`, in `root`.
fn proposals(root: &Path, args: &[&str]) -> Output {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_orchestrator::proposals_command(
        args.iter().copied(),
        root,
        &|_| None,
        &mut out,
        &mut err,
    );
    finish(status, out, err)
}

/// `clm <command> <args> --root <root>`, called with a root that holds no repository, so
/// only `--root` finds one, as the binary's runs found it.
fn clm(root: &Path, args: &[&str]) -> (Option<i32>, String) {
    let elsewhere = Path::new("/nonexistent-clm-root");
    let root = root.to_str().unwrap();
    let (command, rest) = args.split_first().unwrap();
    let line: Vec<&str> = rest.iter().copied().chain(["--root", root]).collect();
    let (code, out, _) = match *command {
        "next" => next(elsewhere, &line, DEFAULT_CAP),
        "claim" => claim(elsewhere, &line),
        "proposals" => proposals(elsewhere, &line),
        other => panic!("not an orchestrator command: {other}"),
    };
    (code, out)
}

const NODE: &str = "---\ntitle: T\nstatus: specd\nowners: []\n---\n\n## Contract\nc\n\n## Design\nd\n\n## Requirements\n\n### Requirement: r\nR.\n\n#### Scenario: s\n- ok\n";

fn change_repo(delta: &str) -> TempDir {
    let dir = TempDir::new().unwrap();
    let put = |rel: &str, content: &str| {
        let path = dir.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    };
    put("specs/SPEC.md", NODE);
    put("changes/c/change.md", "---\nstatus: proposed\n---\n");
    put("changes/c/deltas/clm.md", delta);
    dir
}

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:orchestrator/commands-in-crate@29a80e82
    let r = repo();
    push_claim(&r.work, "b", 1);
    let (code, out, err) = next(&r.work, &[], DEFAULT_CAP);
    assert_eq!((code, out.as_str(), err.as_str()), (Some(0), "a\nc\n", ""));
    // A full cap: nothing on stdout, the one line on stderr, still 0.
    let (code, out, err) = next(&r.work, &[], Ok(0));
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (
            Some(0),
            "",
            "no free build place: 0 of max_builds = 0 in use, 2 ready\n"
        )
    );
    // The command's `--root` overrides the root the call is given.
    let root = r.work.to_str().unwrap();
    let (code, out, _) = next(Path::new("/nonexistent"), &["--root", root], DEFAULT_CAP);
    assert_eq!((code, out.as_str()), (Some(0), "a\nc\n"));
    // A refusal goes to stderr with the command's prefix, and exits 1.
    let (code, out, err) = claim(&r.work, &["b"]);
    assert_eq!((code, out.as_str()), (Some(1), ""));
    assert!(err.starts_with("clm claim: "), "{err}");
    assert!(err.contains("`b` is claimed"), "{err}");
    // Settings that didn't load refuse with the command's prefix.
    let (code, out, err) = next(&r.work, &[], Err("max_builds: bad".into()));
    assert_eq!(
        (code, out.as_str(), err.as_str()),
        (Some(1), "", "clm next: settings: max_builds: bad\n")
    );
}

#[test]
fn specs_that_dont_load_are_violations_on_stdout() {
    // req:orchestrator/commands-in-crate@29a80e82
    let r = repo();
    write(&r.work, "specs/a/SPEC.md", "garbage");
    push_main(&r.work, "break a");
    let (code, out, err) = claim(&r.work, &["c"]);
    assert_eq!(code, Some(1), "{err}");
    assert!(out.starts_with("specs/a/SPEC.md: "), "{out}");
    assert_eq!(err, "");
}

#[test]
fn scenario_arguments_it_cant_parse() {
    // req:orchestrator/commands-in-crate@29a80e82
    let dir = TempDir::new().unwrap();
    let (code, out, err) = next(dir.path(), &["--no-such-option"], DEFAULT_CAP);
    assert_eq!((code, out.as_str()), (Some(2), ""));
    assert!(err.contains("--no-such-option"), "{err}");
    assert!(err.contains("Usage: clm next"), "{err}");
    let (code, _, err) = claim(dir.path(), &[]);
    assert_eq!(code, Some(2));
    assert!(err.contains("Usage: clm claim"), "{err}");
    let (code, _, err) = proposals(dir.path(), &["--nope"]);
    assert_eq!(code, Some(2));
    assert!(err.contains("Usage: clm proposals"), "{err}");
}

#[test]
fn orchestrator_commands_exit_2_when_git_fails() {
    // req:orchestrator/next@cc111fbd req:orchestrator/claim@026fab7a
    // req:orchestrator/backlog-claim@50959cb7 req:orchestrator/proposals@ba5ce6a8
    // req:orchestrator/claim-fix@5a3c119e
    let dir = change_repo("---\ncomponent: clm\n---\n");
    // Not a git repository, so there's no `origin` to fetch.
    assert_eq!(clm(dir.path(), &["next"]).0, Some(2));
    assert_eq!(clm(dir.path(), &["claim", "clm"]).0, Some(2));
    let fix = clm(dir.path(), &["claim", "clm", "--fix", "backlog/q"]);
    assert_eq!(fix.0, Some(2));
    assert_eq!(clm(dir.path(), &["claim", "backlog/q"]).0, Some(2));
    assert_eq!(clm(dir.path(), &["proposals"]).0, Some(2));
}

#[test]
fn git_failures_and_help_on_their_streams() {
    // req:orchestrator/commands-in-crate@29a80e82
    let dir = TempDir::new().unwrap();
    for (command, (code, out, err)) in [
        ("next", next(dir.path(), &[], DEFAULT_CAP)),
        ("claim", claim(dir.path(), &["a"])),
        ("proposals", proposals(dir.path(), &[])),
    ] {
        assert_eq!((code, out.as_str()), (Some(2), ""), "{command}: {err}");
        assert!(err.starts_with(&format!("clm {command}: ")), "{err}");
    }
    let (code, out, err) = claim(dir.path(), &["--help"]);
    assert_eq!((code, err.as_str()), (Some(0), ""));
    assert!(out.contains("Usage: clm claim"), "{out}");
}

/// `clm <command> <args>` run in `dir`: the call given `dir` as the root `crates/clm` finds
/// from it.
fn clm_in(dir: &Path, args: &[&str]) -> Output {
    let (command, rest) = args.split_first().unwrap();
    match *command {
        "next" => next(dir, rest, DEFAULT_CAP),
        "claim" => claim(dir, rest),
        "proposals" => proposals(dir, rest),
        other => panic!("not an orchestrator command: {other}"),
    }
}

#[test]
fn scenario_claim_prints_the_branch_then_is_refused() {
    // req:orchestrator/backlog-claim@50959cb7
    let r = repo();
    write(&r.work, "backlog/q.md", &item("c"));
    push_main(&r.work, "q");
    let (code, out, err) = clm_in(&r.work, &["claim", "backlog/q"]);
    assert_eq!((code, out.as_str()), (Some(0), "propose/q\n"), "{err}");
    let (code, out, err) = clm_in(&r.work, &["claim", "backlog/q"]);
    assert_eq!((code, out.as_str()), (Some(1), ""));
    assert!(err.contains("claimed by propose/q"), "{err}");
    let (code, _, err) = clm_in(&r.work, &["claim", "backlog/nope"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("no such item"), "{err}");
}

#[test]
fn claim_fix_prints_the_branch_then_is_refused() {
    // req:orchestrator/claim-fix@5a3c119e
    let r = repo();
    cite(&r.work, "b.rs", &["b/r"]);
    write(
        &r.work,
        "backlog/x.md",
        "---\ntitle: X\nkind: bug\ncomponents: [b]\n---\n",
    );
    push_main(&r.work, "b converged, bug x");
    let (code, _, err) = clm_in(&r.work, &["claim", "b"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("converged"), "{err}");
    let (code, out, err) = clm_in(&r.work, &["claim", "b", "--fix", "backlog/x"]);
    assert_eq!((code, out.as_str()), (Some(0), "impl/b\n"), "{err}");
    // The checkout is on the claimed branch by the time it is printed.
    assert_eq!(
        git(&r.work, &["rev-parse", "--abbrev-ref", "HEAD"]),
        "impl/b\n"
    );
    let (code, out, err) = clm_in(&r.work, &["claim", "b", "--fix", "backlog/x"]);
    assert_eq!((code, out.as_str()), (Some(1), ""));
    assert!(err.contains("`b` is claimed"), "{err}");
    // A fix claims a node, never a backlog item.
    let (code, _, err) = clm_in(&r.work, &["claim", "backlog/x", "--fix", "backlog/x"]);
    assert_eq!(code, Some(1));
    assert!(err.contains("isn't a component"), "{err}");
}

/// `clm next` in `dir` with `max_builds` as `crates/clm` loads it from `CLM_MAX_BUILDS`,
/// away from the user's settings: the number, or the settings' error for one that isn't.
fn next_with(dir: &Path, max_builds: &str) -> Output {
    let loaded = max_builds.parse::<u64>().map_err(|_| {
        format!("CLM_MAX_BUILDS: max_builds must be a whole number, not `{max_builds}`")
    });
    next(dir, &[], loaded)
}

#[test]
fn next_is_capped_by_the_max_builds_setting() {
    // req:orchestrator/next@cc111fbd
    let r = repo();
    let (code, out, err) = next_with(&r.work, "3");
    assert_eq!((code, out.as_str()), (Some(0), "a\nb\nc\n"), "{err}");
    let placed = clm_orchestrator::take_place(
        &r.work,
        "z",
        clm_orchestrator::PlaceKind::Build,
        &["impl/z".to_string()],
        3,
        4,
    );
    assert_eq!(placed.unwrap(), clm_orchestrator::Placed::New);
    let (code, out, err) = next_with(&r.work, "2");
    assert_eq!((code, out.as_str()), (Some(0), "a\n"), "{err}");
    assert_eq!(err, "", "a list that isn't empty says nothing");
    let (code, out, err) = next_with(&r.work, "1");
    assert_eq!((code, out.as_str()), (Some(0), ""));
    assert_eq!(
        err,
        "no free build place: 1 of max_builds = 1 in use, 3 ready\n"
    );
    let (code, _, err) = next_with(&r.work, "none");
    assert_eq!(code, Some(1), "settings that don't load: {err}");
    assert!(err.contains("max_builds"), "{err}");
}

#[test]
fn proposals_lines_filter_and_warning_through_the_call() {
    // req:orchestrator/proposals@ba5ce6a8
    let r = repo();
    write(&r.work, "backlog/q.md", &item("c"));
    write(&r.work, "backlog/s.md", &item("b"));
    push_main(&r.work, "items");
    push_files(
        &r.work,
        "spec/20260101-c",
        &[
            (
                "changes/20260101-c/change.md",
                &change_md("proposed", "resolves: [q]\n"),
            ),
            ("changes/20260101-c/deltas/a.md", &delta("a", &["u"])),
        ],
    );
    push_files(
        &r.work,
        "spec/20260101-e",
        &[("changes/20260101-e/change.md", "# e\n")],
    );
    push_propose(&r.work, "s", 1);
    let (code, out, err) = clm_in(&r.work, &["proposals"]);
    assert_eq!(code, Some(0), "{err}");
    assert_eq!(
        out,
        "20260101-c proposed [a,c] q\n20260101-e proposed []\nbacklog/s claimed [b]\n"
    );
    assert!(
        err.starts_with("clm proposals: warning: ") && err.contains("origin/spec/20260101-e"),
        "{err}"
    );
    let (code, out, _) = clm_in(
        &r.work,
        &["proposals", "--component", "b", "--component", "c"],
    );
    assert_eq!(
        (code, out.as_str()),
        (
            Some(0),
            "20260101-c proposed [a,c] q\nbacklog/s claimed [b]\n"
        )
    );
    let (code, out, _) = clm_in(&r.work, &["proposals", "--component", "x"]);
    assert_eq!((code, out.as_str()), (Some(0), ""));
}

#[test]
fn a_node_that_writes_a_protected_path_is_ready() {
    // req:orchestrator/next@cc111fbd
    let r = repo();
    write(
        &r.work,
        "specs/w/SPEC.md",
        &node_with(
            "",
            "paths: [\"web/**\"]\n",
            &[("x", "It SHALL write `web/.npmrc`.")],
        ),
    );
    push_main(&r.work, "w");
    let (code, out, err) = next(&r.work, &[], Ok(9));
    assert_eq!((code, out.as_str()), (Some(0), "a\nb\nc\nw\n"), "{err}");
}
