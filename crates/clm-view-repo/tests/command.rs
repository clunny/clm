//! `clm status`, `clm view` and `clm pending` through `clm-view-repo`'s calls
//! (`commands-in-crate`), with the settings `crates/clm` would load from `config` given as
//! arguments.

mod common;

use std::path::Path;

use clm_check::{Budgets, Violation};
use common::specd;

/// What a call wrote to its two streams, and what it returned.
struct Output {
    status: u8,
    stdout: String,
    stderr: String,
}

/// The budgets `crates/clm` passes when the settings load: the defaults.
fn defaults(_: &Path) -> Result<Budgets, Violation> {
    Ok(Budgets {
        spec_tokens: 12000,
        code_tokens: 40000,
    })
}

/// `clm status <args> --root <root>`.
fn status(root: &Path, args: &[&str]) -> Output {
    let mut argv: Vec<std::ffi::OsString> = args.iter().map(Into::into).collect();
    argv.push("--root".into());
    argv.push(root.into());
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let status = clm_view_repo::status_command(
        &argv,
        Path::new("."),
        &|_| None,
        &mut out,
        &mut err,
        &defaults,
    );
    Output {
        status,
        stdout: String::from_utf8(out).unwrap(),
        stderr: String::from_utf8(err).unwrap(),
    }
}

/// A fixture repository with one commit: a root and component `a` with requirement `x`.
fn fixture() -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    for (path, text) in [
        ("specs/SPEC.md", specd("", &[])),
        ("specs/a/SPEC.md", specd("", &["x"])),
    ] {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let git = |args: &[&str]| {
        let base = ["-c", "user.name=t", "-c", "user.email=t@t", "-c"];
        let all: Vec<&str> = base
            .into_iter()
            .chain(["commit.gpgsign=false"])
            .chain(args.iter().copied())
            .collect();
        clm_git::git(dir.path(), &all).unwrap();
    };
    git(&["init", "-q", "-b", "main"]);
    git(&["add", "-A"]);
    git(&["commit", "-qm", "base"]);
    dir
}

/// The checkout line's model for `root`: `HEAD`'s first 7 hex digits, no `origin/main`.
fn checkout(root: &Path) -> clm_view::Checkout {
    let head = clm_git::git(root, &["rev-parse", "HEAD"]).unwrap();
    clm_view::Checkout {
        commit: head[..7].to_string(),
        behind: None,
    }
}

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:view.repo/commands-in-crate@ec1f97fe
    let dir = fixture();
    let out = status(dir.path(), &[]);
    assert_eq!(out.status, 0, "{}", out.stderr);
    assert!(out.stdout.starts_with("CLM status\n"), "{}", out.stdout);
    assert_eq!(out.stderr, "");
    // A graph that doesn't load: its violations on stdout, one line each, and 1.
    std::fs::write(dir.path().join("specs/a/SPEC.md"), "not a spec").unwrap();
    let bad = status(dir.path(), &[]);
    assert_eq!(bad.status, 1);
    let (_, violations) = clm_model::load(dir.path());
    let lines: String = violations.iter().map(|v| format!("{v}\n")).collect();
    assert_eq!(bad.stdout, lines);
}

#[test]
fn scenario_arguments_it_cant_parse() {
    // req:view.repo/commands-in-crate@ec1f97fe
    let dir = fixture();
    let out = status(dir.path(), &["--bogus"]);
    assert_eq!(out.status, 2);
    assert_eq!(out.stdout, "");
    assert!(out.stderr.contains("--bogus"), "{}", out.stderr);
    // The same for `clm view` and `clm pending`, and the test's process goes on.
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let code = clm_view_repo::view_command(
        ["nonesuch"],
        dir.path(),
        &|_| None,
        &mut o,
        &mut e,
        &defaults,
        &|_| defaults(Path::new(".")).map_err(|v| v.message),
    );
    assert_eq!(code, 2);
    assert!(o.is_empty() && !e.is_empty());
    let (mut o, mut e) = (Vec::new(), Vec::new());
    let code = clm_view_repo::pending_command(["--bogus"], dir.path(), &|_| None, &mut o, &mut e);
    assert_eq!(code, 2);
    assert!(o.is_empty() && !e.is_empty());
}

#[test]
fn scenario_text_from_the_crate_below() {
    // req:view.repo/commands-in-crate@ec1f97fe
    let dir = fixture();
    let d = dir.path();
    let out = status(d, &[]);
    let (graph, _) = clm_model::load(d);
    let markers = clm_coverage::scan(d).req;
    let sizes = clm_check::size(d, &graph, &markers, defaults(d).unwrap()).unwrap();
    let oversized = sizes
        .into_iter()
        .filter(clm_check::Size::oversized)
        .map(|s| s.node)
        .collect();
    let expected = clm_view::status_text(
        &graph,
        &markers,
        &[],
        &[],
        clm_git::now(),
        Some(&oversized),
        Some(&checkout(d)),
    );
    assert_eq!(out.stdout, expected);
    assert_eq!(out.status, 0);
}

#[test]
fn settings_that_dont_load_are_named_on_stderr() {
    // req:view.repo/commands-in-crate@ec1f97fe
    let dir = fixture();
    let argv: Vec<std::ffi::OsString> = vec!["--root".into(), dir.path().into()];
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let broken = |root: &Path| -> Result<Budgets, Violation> {
        Err(Violation {
            path: root.join("clm.toml"),
            rule: "check/node-size",
            message: "broken".into(),
        })
    };
    let code = clm_view_repo::status_command(
        &argv,
        Path::new("."),
        &|_| None,
        &mut out,
        &mut err,
        &broken,
    );
    assert_eq!(code, 0);
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "size not checked: broken\n"
    );
    assert!(String::from_utf8(out).unwrap().starts_with("CLM status\n"));
}

#[test]
fn the_root_it_is_given_without_root() {
    // req:view.repo/commands-in-crate@ec1f97fe
    let dir = fixture();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let no_args: [&str; 0] = [];
    let code = clm_view_repo::status_command(
        no_args,
        dir.path(),
        &|_| None,
        &mut out,
        &mut err,
        &defaults,
    );
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&err));
    assert_eq!(
        String::from_utf8(out).unwrap(),
        status(dir.path(), &[]).stdout
    );
}

/// The budgets a commit's `clm.toml` gives, as `crates/clm` passes them: the defaults.
fn budgets_from(_: Option<&str>) -> Result<Budgets, String> {
    defaults(Path::new(".")).map_err(|v| v.message)
}

/// `name`'s call given `args` and two buffers: its status and what it wrote to each.
fn call(name: &str, args: &[&str], out: &mut dyn std::io::Write) -> (u8, String) {
    let mut err = Vec::new();
    let here = Path::new(".");
    let code = match name {
        "status" => clm_view_repo::status_command(args, here, &|_| None, out, &mut err, &defaults),
        "view" => clm_view_repo::view_command(
            args,
            here,
            &|_| None,
            out,
            &mut err,
            &defaults,
            &budgets_from,
        ),
        "pending" => clm_view_repo::pending_command(args, here, &|_| None, out, &mut err),
        other => panic!("no call for `clm {other}`"),
    };
    (code, String::from_utf8(err).unwrap())
}

#[test]
fn scenario_help() {
    // req:view.repo/commands-in-crate@ec1f97fe
    for (name, words) in [
        ("status", &["--root"][..]),
        (
            "view",
            &[
                "<NAME>", "spec", "flight", "blockers", "queue", "--format", "text", "ansi", "svg",
                "json", "--width", "100", "--at", "--root",
            ][..],
        ),
        ("pending", &["[COMPONENT]", "--at", "--root"][..]),
    ] {
        let mut out = Vec::new();
        let (code, err) = call(name, &["--help"], &mut out);
        let out = String::from_utf8(out).unwrap();
        assert_eq!((code, err.as_str()), (0, ""), "clm {name} --help: {out}");
        assert!(
            out.contains(&format!("Usage: clm {name}")),
            "clm {name}: {out}"
        );
        for word in words {
            assert!(out.contains(word), "clm {name} --help lacks {word}: {out}");
        }
    }
    // `text` is the default form, and 100 the default width.
    let mut out = Vec::new();
    call("view", &["--help"], &mut out);
    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("[default: text]"), "{out}");
    assert!(out.contains("[default: 100]"), "{out}");
}

/// A stream whose every write fails.
struct Broken;

impl std::io::Write for Broken {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("broken"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::Error::other("broken"))
    }
}

#[test]
fn scenario_an_output_it_cant_write() {
    // req:view.repo/commands-in-crate@ec1f97fe
    let dir = fixture();
    let root = dir.path().to_str().unwrap();
    let (code, _) = call("status", &["--root", root], &mut Broken);
    assert_eq!(code, 1);
    let (code, _) = call("view", &["--help"], &mut Broken);
    assert_eq!(code, 1);
    // The caller's process goes on: this line runs.
    let (code, _) = call("pending", &["--help"], &mut Broken);
    assert_eq!(code, 1);
}
