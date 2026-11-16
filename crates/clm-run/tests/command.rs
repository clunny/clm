//! `clm run`, `clm setup` and `clm superpowers` as `clm-run`'s calls (`commands-in-crate`):
//! the call writes what the command prints to the buffers it is given and returns the status
//! it exits with, never ending the test's process.

mod support;

use std::path::Path;

use support::common::*;
use support::*;

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:orchestrator.run/commands-in-crate@57a3e9af
    let (r, f) = (repo(), fake());
    // The launcher on standard output, nothing on standard error, and status 0.
    let ran = ran_with(
        |env, out, err| clm_run::run_command(["b"], &r.work, &r.work, env, out, err),
        &run_env(&f, &[]),
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(ran.stdout.contains("git switch impl/b"), "{}", ran.stdout);
    assert!(ran.stdout.ends_with('\n'), "{}", ran.stdout);
    assert_eq!(ran.stderr, "");
    // A refusal on standard error, prefixed as the command prints it, with status 1.
    let ran = ran_with(
        |env, out, err| clm_run::run_command(["a/b"], &r.work, &r.work, env, out, err),
        &run_env(&f, &[]),
    );
    assert_eq!(ran.code, Some(1));
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr
            .starts_with("clm run: `a/b` isn't a component ID"),
        "{}",
        ran.stderr
    );
    // `--root` overrides the root the call is given, and `--finish` frees the place.
    let root = r.work.to_str().unwrap();
    let ran = ran_with(
        |env, out, err| {
            let line = ["--finish", "b", "--root", root];
            clm_run::run_command(line, Path::new("/nonexistent"), &r.work, env, out, err)
        },
        &run_env(&f, &[]),
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert_eq!(ran.stdout, "freed b\n");
    // A relative `--root` is read from the directory the call is given, not the process's.
    let parent = r.work.parent().unwrap();
    let ran = ran_with(
        |env, out, err| {
            let line = ["b", "--root", "work"];
            clm_run::run_command(line, Path::new("/nonexistent"), parent, env, out, err)
        },
        &run_env(&f, &[]),
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    let file = r.work.canonicalize().unwrap().join(".clm/prompts/b.md");
    assert!(
        ran.stdout.contains(&file.display().to_string()),
        "{}",
        ran.stdout
    );
}

#[test]
fn scenario_no_free_place_is_status_3() {
    // req:orchestrator.run/commands-in-crate@57a3e9af
    let (r, f) = (repo(), fake());
    push_claim(&r.work, "a", 0);
    git(&r.work, &["fetch", "-q", "origin"]);
    clm_orchestrator::take_place(
        &r.work,
        "a",
        clm_orchestrator::PlaceKind::Build,
        &["impl/a".to_string()],
        1,
        4,
    )
    .unwrap();
    let ran = clm_run_with(
        &r.work,
        &r.work,
        &f,
        &["b"],
        &[("CLM_MAX_BUILDS", Path::new("1"))],
    );
    assert_eq!(ran.code, Some(3), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(
        ran.stderr.starts_with("clm run: no free build place"),
        "{}",
        ran.stderr
    );
}

#[test]
fn scenario_arguments_it_cant_parse() {
    // req:orchestrator.run/commands-in-crate@57a3e9af
    let here = std::env::current_dir().unwrap();
    let none = |_: &str| None;
    for (name, call) in [
        ("clm run", clm_run::run_command::<Vec<&str>, &str> as Call),
        ("clm setup", clm_run::setup_command::<Vec<&str>, &str>),
        (
            "clm superpowers",
            clm_run::superpowers_command::<Vec<&str>, &str>,
        ),
    ] {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = call(
            vec!["--no-such-option"],
            &here,
            &here,
            &none,
            &mut out,
            &mut err,
        );
        let err = String::from_utf8(err).unwrap();
        assert_eq!(status, 2, "{name}: {err}");
        assert!(err.contains("--no-such-option"), "{name}: {err}");
        assert!(err.contains(name), "{name}: {err}");
        assert!(out.is_empty(), "{name}");
    }
    // The test's process goes on.
}

/// The shape every call of `clm-run` has.
type Call = fn(
    Vec<&'static str>,
    &Path,
    &Path,
    &dyn Fn(&str) -> Option<String>,
    &mut dyn std::io::Write,
    &mut dyn std::io::Write,
) -> u8;
