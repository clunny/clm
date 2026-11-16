//! `clm triage`, `clm draft` and `clm intake` as `clm-triage`'s calls
//! (`orchestrator.triage/commands-in-crate`): what each writes to its two streams and the
//! status it returns, never ending the test's process.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::path::Path;

use clm_host::{Alert, Error as HostError, Host, Listed};
use clm_triage::{DraftSettings, draft_command, intake_command, triage_command};
use common::*;

/// What a call wrote to standard output and to standard error, and the status it returned.
struct Called {
    code: u8,
    out: String,
    err: String,
}

fn called(run: impl FnOnce(&mut Vec<u8>, &mut Vec<u8>) -> u8) -> Called {
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let code = run(&mut out, &mut err);
    Called {
        code,
        out: String::from_utf8(out).unwrap(),
        err: String::from_utf8(err).unwrap(),
    }
}

fn triage(args: &[&str], root: &Path) -> Called {
    called(|out, err| triage_command(args, root, root, &|_| None, out, err))
}

fn draft(args: &[&str], root: &Path, settings: Result<DraftSettings, String>) -> Called {
    called(|out, err| draft_command(args, root, root, &|_| None, settings, out, err))
}

fn settings(max_builds: u64) -> Result<DraftSettings, String> {
    Ok(DraftSettings {
        setup: None,
        max_builds,
    })
}

/// A host with nothing open and no alerts.
struct Quiet;

impl Host for Quiet {
    fn page_open(&self, _: &Path, _: &str, _: &str, _: &Path) -> Result<Option<String>, HostError> {
        Ok(None)
    }
    fn page_body(&self, _: &Path, _: &Path) -> Result<Option<String>, HostError> {
        Ok(None)
    }
    fn issue(&self, _: &Path, _: &str, _: &Path, _: &str) -> Result<String, HostError> {
        Err(HostError::Nowhere(String::new()))
    }
    fn list(&self, _: &Path) -> Result<Vec<Listed>, HostError> {
        Ok(Vec::new())
    }
    fn alerts(&self, _: &Path) -> Result<Vec<Alert>, HostError> {
        Ok(Vec::new())
    }
}

/// The repo with bug `a` (`components: [check]`) in `origin/main`'s backlog.
fn with_bug() -> Repo {
    let r = repo();
    write(&r.work, "specs/check/SPEC.md", &node("", &["x"]));
    write(
        &r.work,
        "backlog/a.md",
        "---\ntitle: A\nkind: bug\ncomponents: [check]\n---\nBody a.\n",
    );
    push_main(&r.work, "items");
    r
}

#[test]
fn scenario_a_command_called_in_a_test() {
    // req:orchestrator.triage/commands-in-crate@6d42d28b
    let r = with_bug();
    let c = triage(&["--expiry", "4"], &r.work);
    assert_eq!(
        (c.code, c.out.as_str(), c.err.as_str()),
        (0, "group bug:check [check] P5 0 a\n", "")
    );
    // `--root` overrides the root the call is given.
    let c = triage(
        &["--root", r.work.to_str().unwrap()],
        Path::new("/nonexistent-dir"),
    );
    assert_eq!(c.out, "group bug:check [check] P5 0 a\n", "{}", c.err);
    // A git failure is `clm triage: <why>` on the error stream, and 2.
    let c = triage(&[], Path::new("/nonexistent-dir"));
    assert_eq!(c.code, 2, "{}", c.err);
    assert!(c.out.is_empty(), "{}", c.out);
    assert!(c.err.starts_with("clm triage: "), "{}", c.err);
}

#[test]
fn a_draft_writes_its_launcher_out_and_its_id_last_on_err() {
    // req:orchestrator.triage/commands-in-crate@6d42d28b
    let r = with_bug();
    let c = draft(&["a", "nope"], &r.work, settings(3));
    assert_eq!(c.code, 0, "{}", c.err);
    assert!(c.out.starts_with("Read `"), "{}", c.out);
    let lines: Vec<&str> = c.err.lines().collect();
    assert!(lines[0].starts_with("left nope: "), "{}", c.err);
    assert!(
        lines.last().unwrap().starts_with("draft ") && lines.last().unwrap().ends_with("-a"),
        "{}",
        c.err
    );
}

#[test]
fn a_draft_with_no_free_place_returns_3() {
    // req:orchestrator.triage/commands-in-crate@6d42d28b
    let r = with_bug();
    let c = draft(&["a"], &r.work, settings(0));
    assert_eq!(c.code, 3, "{}", c.err);
    assert!(c.out.is_empty(), "{}", c.out);
    assert!(
        c.err.starts_with("clm draft: no free build place: "),
        "{}",
        c.err
    );
}

#[test]
fn settings_that_dont_load_refuse_a_draft_but_not_a_finish() {
    // req:orchestrator.triage/commands-in-crate@6d42d28b
    let r = with_bug();
    let c = draft(&["a"], &r.work, Err("bad toml".into()));
    assert_eq!(
        (c.code, c.out.as_str(), c.err.as_str()),
        (1, "", "clm draft: settings: bad toml\n")
    );
    let c = draft(&["--resume", "20261027-a"], &r.work, Err("bad toml".into()));
    assert_eq!(
        (c.code, c.err.as_str()),
        (1, "clm draft: settings: bad toml\n")
    );
    let c = draft(&["--finish", "20261027-a"], &r.work, Err("bad toml".into()));
    assert_eq!(c.code, 0, "{}", c.err);
}

#[test]
fn intake_writes_its_lines_or_why_the_settings_failed() {
    // req:orchestrator.triage/commands-in-crate@6d42d28b
    let r = with_bug();
    let intake = |args: &[&str], root: &Path, cwd: &Path| {
        called(|out, err| intake_command(args, root, cwd, &|_| None, Ok(Box::new(Quiet)), out, err))
    };
    let c = intake(&[], &r.work, &r.work);
    assert_eq!((c.code, c.out.as_str()), (0, ""), "{}", c.err);
    // Where git can't run, the fetch fails with 2; `--root` overrides that root, and a
    // relative one is read against the directory the call runs in.
    let nowhere = Path::new("/nonexistent-dir");
    let c = intake(&[], nowhere, nowhere);
    assert_eq!(c.code, 2, "{}", c.err);
    assert!(c.err.starts_with("clm intake: "), "{}", c.err);
    let c = intake(&["--root", "work"], nowhere, r.work.parent().unwrap());
    assert_eq!((c.code, c.out.as_str()), (0, ""), "{}", c.err);
    let c = called(|out, err| {
        intake_command(
            [] as [&str; 0],
            &r.work,
            &r.work,
            &|_| None,
            Err("bad host".into()),
            out,
            err,
        )
    });
    assert_eq!(
        (c.code, c.out.as_str(), c.err.as_str()),
        (2, "", "clm intake: bad host\n")
    );
}

#[test]
fn scenario_arguments_it_cant_parse() {
    // req:orchestrator.triage/commands-in-crate@6d42d28b
    let r = with_bug();
    let c = triage(&["--no-such-option"], &r.work);
    // The test's process goes on: the call returns rather than exits.
    assert_eq!(c.code, 2);
    assert!(c.out.is_empty(), "{}", c.out);
    assert!(c.err.contains("--no-such-option"), "{}", c.err);
    assert!(c.err.contains("Usage: clm triage"), "{}", c.err);
    let c = draft(&["--fold", "--finish", "x"], &r.work, settings(3));
    assert_eq!(c.code, 2);
    assert!(c.err.contains("Usage: clm draft"), "{}", c.err);
}
