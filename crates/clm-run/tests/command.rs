//! `clm run`, `clm setup` and `clm superpowers` as `clm-run`'s calls (`commands-in-crate`):
//! the call writes what the command prints to the buffers it is given and returns the status
//! it exits with, never ending the test's process.

mod support;

use std::path::Path;

use support::common::*;
use support::*;

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:orchestrator.run/commands-in-crate@14428b0e
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
    // req:orchestrator.run/commands-in-crate@14428b0e
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
    // req:orchestrator.run/commands-in-crate@14428b0e
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

#[test]
fn scenario_help() {
    // req:orchestrator.run/commands-in-crate@14428b0e
    let here = std::env::current_dir().unwrap();
    let none = |_: &str| None;
    for (call, description, usage, options) in [
        (
            clm_run::run_command::<Vec<&str>, &str> as Call,
            "Claim a component, or pick up its unfinished build, write the prompt for a session \
             that builds and lands its pending requirements to `.clm/prompts/<component>.md`, \
             and print a launcher naming that file. With `--finish`, free the component's build \
             place instead and, once its branch has landed, remove its clean worktrees under \
             `.claude/worktrees/`, keeping one whose lock a running process holds unless \
             `--returned` says the build's subagent has handed back",
            "Usage: clm run [OPTIONS] <COMPONENT>",
            &[
                "--finish",
                "--returned",
                "--expiry",
                "--review-model",
                "--root",
            ][..],
        ),
        (
            clm_run::setup_command::<Vec<&str>, &str>,
            "Run the repository's `[build] setup`, which makes a fresh worktree ready to run \
             its tests",
            "Usage: clm setup [OPTIONS]",
            &["--root"][..],
        ),
        (
            clm_run::superpowers_command::<Vec<&str>, &str>,
            "Print the path of a skill's SKILL.md in the pinned superpowers copy builds follow",
            "Usage: clm superpowers <SKILL>",
            &[][..],
        ),
    ] {
        let (mut out, mut err) = (Vec::new(), Vec::new());
        let status = call(vec!["--help"], &here, &here, &none, &mut out, &mut err);
        let out = String::from_utf8(out).unwrap();
        assert_eq!(status, 0, "{out}");
        assert!(err.is_empty(), "{}", String::from_utf8_lossy(&err));
        let mut lines = out.lines();
        assert_eq!(lines.next(), Some(description), "{out}");
        assert_eq!(lines.next(), Some(""), "{out}");
        assert_eq!(lines.next(), Some(usage), "{out}");
        for option in options {
            assert!(out.contains(option), "{option} missing from {out}");
        }
    }
}

#[test]
fn scenario_a_root_given_by_root() {
    // req:orchestrator.run/commands-in-crate@14428b0e
    let (r, f) = (repo(), fake());
    let parent = r.work.parent().unwrap();
    let gone = parent.join("no-such-root");
    let gone_arg = gone.to_str().unwrap();
    // The call of `clm run b` with a root that doesn't exist: 1 naming it, nothing claimed.
    let ran = ran_with(
        |env, out, err| {
            clm_run::run_command(["b", "--root", gone_arg], &r.work, parent, env, out, err)
        },
        &run_env(&f, &[]),
    );
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert_eq!(ran.stdout, "");
    assert!(ran.stderr.contains(gone_arg), "{}", ran.stderr);
    assert!(remote_tip(&r, "impl/b").is_none(), "no claim");
    assert!(place_names(&r).is_empty(), "no place");
    // The call of `clm setup` with that root: 1 naming it, nothing run.
    let seen = parent.join("seen");
    write(
        &r.work,
        "clm.toml",
        &format!("[build]\nsetup = \"touch {}\"\n", seen.display()),
    );
    let ran = ran_with(
        |env, out, err| {
            clm_run::setup_command(["--root", gone_arg], &r.work, parent, env, out, err)
        },
        &run_env(&f, &[]),
    );
    assert_eq!(ran.code, Some(1), "{}", ran.stderr);
    assert!(ran.stderr.contains(gone_arg), "{}", ran.stderr);
    assert!(!seen.exists(), "nothing run");
    // The call of `clm run b` in `r`'s parent with `--root r` writes `r/.clm/prompts/b.md`.
    std::fs::remove_file(r.work.join("clm.toml")).unwrap();
    let ran = ran_with(
        |env, out, err| {
            let line = ["b", "--root", "work"];
            clm_run::run_command(line, Path::new("/nonexistent"), parent, env, out, err)
        },
        &run_env(&f, &[]),
    );
    assert_eq!(ran.code, Some(0), "{}", ran.stderr);
    assert!(r.work.join(".clm/prompts/b.md").is_file());
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
fn scenario_a_stream_that_cant_be_written() {
    // req:orchestrator.run/commands-in-crate@14428b0e
    let (r, f) = (repo(), fake());
    let env = run_env(&f, &[]);
    let lookup = |k: &str| env.get(k).cloned();
    let root = r.work.to_str().unwrap();
    // `clm run b`: 2, and the claim, the place `b` and the prompt it wrote stay.
    let mut err = Vec::new();
    let line = ["b", "--root", root];
    let status = clm_run::run_command(line, &r.work, &r.work, &lookup, &mut Broken, &mut err);
    assert_eq!(status, 2, "{}", String::from_utf8_lossy(&err));
    assert!(remote_tip(&r, "impl/b").is_some(), "the claim stays");
    assert_eq!(place_names(&r), ["b"]);
    assert!(r.work.join(".clm/prompts/b.md").is_file());
    // `clm run --finish b` with the place `b`: 2.
    let line = ["--finish", "b", "--root", root];
    let status = clm_run::run_command(line, &r.work, &r.work, &lookup, &mut Broken, &mut err);
    assert_eq!(status, 2);
    // `clm setup` running `echo ready`, whose output passes through to that stream: 2.
    write(&r.work, "clm.toml", "[build]\nsetup = \"echo ready\"\n");
    let mut err = Vec::new();
    let line = ["--root", root];
    let status = clm_run::setup_command(line, &r.work, &r.work, &lookup, &mut Broken, &mut err);
    assert_eq!(status, 2, "{}", String::from_utf8_lossy(&err));
}
