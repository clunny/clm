//! `clm approve` as `clm-approval`'s call (`land.approval/commands-in-crate`): what it prints
//! and how it exits.

mod common;

use std::path::Path;

use common::Fixture;

/// What the call wrote and returned.
#[derive(Debug)]
struct Out {
    code: u8,
    stdout: String,
    stderr: String,
}

/// `clm approve <args>` in `root`, as run from `cwd`, with `f`'s environment.
fn approve(f: &Fixture, root: &Path, cwd: &Path, args: &[&str]) -> Out {
    let (mut stdout, mut stderr) = (Vec::new(), Vec::new());
    let code = clm_approval::approve_command(
        args.iter().copied(),
        root,
        cwd,
        &f.env(&[]),
        &mut stdout,
        &mut stderr,
    );
    Out {
        code,
        stdout: String::from_utf8(stdout).unwrap(),
        stderr: String::from_utf8(stderr).unwrap(),
    }
}

#[test]
fn approve_prints_the_fingerprint() {
    // req:land.approval/commands-in-crate@1f31b577
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let fp = f.fingerprint("spec/x");
    let out = approve(&f, &f.work(), &f.work(), &["spec/x"]);
    assert_eq!(out.code, 0, "{out:?}");
    assert_eq!(out.stdout, format!("approved {fp} spec/x\n"));
    assert_eq!(out.stderr, "");
    assert_eq!(f.records().len(), 1);
}

#[test]
fn root_option_overrides_the_root_given() {
    // req:land.approval/commands-in-crate@1f31b577
    let f = Fixture::new("");
    f.branch("spec/x", &[("changes/x/change.md", "x\n")]);
    let elsewhere = f.path("missing");
    let out = approve(&f, &elsewhere, &f.path(""), &["spec/x", "--root", "work"]);
    assert_eq!(out.code, 0, "{out:?}");
    assert!(out.stdout.starts_with("approved "), "{out:?}");
}

#[test]
fn a_refusal_exits_1_on_the_error_stream() {
    // req:land.approval/commands-in-crate@1f31b577
    let f = Fixture::new("");
    let out = approve(&f, &f.work(), &f.work(), &[]);
    assert_eq!(out.code, 1, "{out:?}");
    assert_eq!(out.stdout, "");
    assert!(out.stderr.starts_with("clm approve: "), "{out:?}");
    assert!(out.stderr.contains("`main`"), "{out:?}");
    assert!(f.records().is_empty());
}

#[test]
fn a_failure_exits_2() {
    // req:land.approval/commands-in-crate@1f31b577
    let f = Fixture::new("");
    let out = approve(&f, &f.path("missing"), &f.path(""), &["spec/x"]);
    assert_eq!(out.code, 2, "{out:?}");
    assert!(out.stderr.starts_with("clm approve: "), "{out:?}");
}

#[test]
fn arguments_it_cant_parse() {
    // req:land.approval/commands-in-crate@1f31b577
    let f = Fixture::new("");
    let out = approve(&f, &f.work(), &f.work(), &["--frobnicate"]);
    assert_eq!(out.code, 2, "{out:?}");
    assert_eq!(out.stdout, "");
    assert!(out.stderr.contains("--frobnicate"), "{out:?}");
    assert!(f.records().is_empty());
}
