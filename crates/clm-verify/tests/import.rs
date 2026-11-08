//! The result of a host pipeline's run, recorded without running a check (verify/verify-import).

mod common;

use std::sync::Mutex;

use clm_ledger::{Body, VerifyMode};
use clm_verify::{Outcome, Request};
use common::{Fixture, write_exec};

/// The tests put a fake `gh` first on the process's `PATH`, one test at a time.
static PATH_LOCK: Mutex<()> = Mutex::new(());

const IMPORT: &str = "mode = \"import\"\nworkflow = \"slow\"\ntriggers = [\"change-done\"]";

fn run(head: &str, status: &str, conclusion: &str, id: u32) -> String {
    format!(
        "{{\"databaseId\":{id},\"headSha\":\"{head}\",\"status\":\"{status}\",\
         \"conclusion\":\"{conclusion}\",\"url\":\"https://host/run/{id}\"}}"
    )
}

/// Runs `body` with a `gh` that prints `runs` (a JSON array's items) first on `PATH`, or
/// fails with `fails`.
fn with_gh<T>(f: &Fixture, runs: &[String], body: impl FnOnce() -> T) -> T {
    let _one = PATH_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let bin = f.path("gh-bin");
    std::fs::create_dir_all(&bin).unwrap();
    let list = format!("[{}]", runs.join(","));
    let script = format!(
        "#!/bin/sh\necho \"$@\" >> '{calls}'\n{reply}\n",
        calls = f.path("gh-calls.log").display(),
        reply = match runs.first().map(String::as_str) {
            Some("fail") => "echo 'gh: no luck' >&2; exit 1".to_string(),
            _ => format!("cat <<'EOT'\n{list}\nEOT"),
        }
    );
    write_exec(&bin.join("gh"), &script);
    let old = std::env::var("PATH").unwrap();
    // SAFETY: every test of this file that touches the environment holds PATH_LOCK.
    unsafe { std::env::set_var("PATH", format!("{}:{old}", bin.display())) };
    let out = body();
    unsafe { std::env::set_var("PATH", old) };
    out
}

fn import(f: &Fixture, request: &Request) -> Outcome {
    f.run(request)
}

fn last_body(f: &Fixture) -> Body {
    f.records().pop().expect("a record")
}

#[test]
fn a_passing_run_imported() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("crates/a/x.rs", "1\n")], "l");
    let out = with_gh(&f, &[run(&l, "completed", "success", 7)], || {
        import(&f, &Request::Import)
    });
    assert_eq!(out.line, format!("verify passed {} 1 lands", &l[..7]));
    assert_eq!(out.code, 0);
    let Body::Verified {
        commit,
        passed,
        run,
        culprit,
        mode,
        lands,
        ..
    } = last_body(&f)
    else {
        panic!()
    };
    assert_eq!(commit, l);
    assert!(passed);
    assert_eq!(run.as_deref(), Some("https://host/run/7"));
    assert_eq!(culprit, None);
    assert_eq!(mode, VerifyMode::Import);
    assert_eq!(lands, [l]);
    // No check ran.
    assert!(f.lines("check-pwd.log").is_empty());
    let calls = f.lines("gh-calls.log");
    assert_eq!(calls.len(), 1);
    assert!(calls[0].contains("run list --workflow slow"), "{calls:?}");
}

#[test]
fn a_failing_run_over_one_land_names_it() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("crates/b/x.rs", "1\n")], "l");
    let out = with_gh(&f, &[run(&l, "completed", "failure", 8)], || {
        import(&f, &Request::Import)
    });
    assert_eq!(
        out.line,
        format!("verify failed {0} culprit {0} components b", &l[..7])
    );
    assert_eq!(out.code, 1);
    let Body::Verified {
        culprit, passed, ..
    } = last_body(&f)
    else {
        panic!()
    };
    assert_eq!(culprit.as_deref(), Some(l.as_str()));
    assert!(!passed);
}

#[test]
fn a_failing_run_over_several_lands_leaves_the_search() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l1 = f.land(&[("crates/a/x.rs", "1\n")], "l1");
    let l2 = f.land(&[("crates/b/y.rs", "2\n")], "l2");
    let out = with_gh(&f, &[run(&l2, "completed", "failure", 9)], || {
        import(&f, &Request::Import)
    });
    assert_eq!(
        out.line,
        format!("verify failed {} culprit unknown components a b", &l2[..7])
    );
    let Body::Verified {
        culprit,
        components,
        lands,
        ..
    } = last_body(&f)
    else {
        panic!()
    };
    assert_eq!(culprit, None);
    assert_eq!(components, ["a", "b"]);
    assert_eq!(lands, [l1, l2]);
    assert!(f.lines("check-pwd.log").is_empty());
}

#[test]
fn the_newest_result_covers_the_lands_up_to_it() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let l1 = f.land(&[("crates/a/x.rs", "1\n")], "l1");
    let l2 = f.land(&[("crates/a/y.rs", "2\n")], "l2");
    let l3 = f.land(&[("crates/a/z.rs", "3\n")], "l3");
    // Newest first: the run on `l3` is still going, so `l2`'s is the newest result.
    let runs = [
        run(&l3, "in_progress", "", 12),
        run(&l2, "completed", "success", 11),
        run(&l1, "completed", "failure", 10),
    ];
    let out = with_gh(&f, &runs, || import(&f, &Request::Import));
    assert_eq!(out.line, format!("verify passed {} 3 lands", &l2[..7]));
    let Body::Verified { commit, run, .. } = last_body(&f) else {
        panic!()
    };
    assert_eq!(commit, l2);
    assert_eq!(run.as_deref(), Some("https://host/run/11"));
}

#[test]
fn still_running() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("crates/a/x.rs", "1\n")], "l");
    let before = f.records().len();
    let out = with_gh(&f, &[run(&l, "in_progress", "", 7)], || {
        import(&f, &Request::Import)
    });
    assert_eq!(out.line, format!("verify pending {}", &l[..7]));
    assert_eq!(out.code, 4);
    assert_eq!(f.records().len(), before);
}

#[test]
fn a_cancelled_run_is_no_result() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("crates/a/x.rs", "1\n")], "l");
    let before = f.records().len();
    let out = with_gh(&f, &[run(&l, "completed", "cancelled", 7)], || {
        import(&f, &Request::Import)
    });
    assert_eq!(out.line, format!("verify pending {}", &l[..7]));
    assert_eq!(f.records().len(), before);
}

#[test]
fn a_run_on_an_already_verified_commit_is_no_result() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("crates/a/x.rs", "1\n")], "l");
    let out = with_gh(&f, &[run(&g, "completed", "success", 5)], || {
        import(&f, &Request::Import)
    });
    assert_eq!(out.line, format!("verify pending {}", &l[..7]));
}

#[test]
fn a_run_not_completed_is_no_result_whatever_it_says() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let l = f.land(&[("crates/a/x.rs", "1\n")], "l");
    let out = with_gh(&f, &[run(&l, "in_progress", "success", 7)], || {
        import(&f, &Request::Import)
    });
    assert_eq!(out.line, format!("verify pending {}", &l[..7]));
    assert!(f.records().is_empty());
}

#[test]
fn no_result_is_pending_even_with_nothing_landed() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let head = f.back(0);
    f.verified(&head, true);
    let out = with_gh(&f, &[], || import(&f, &Request::Import));
    assert_eq!(out.line, format!("verify pending {}", &head[..7]));
    assert_eq!(out.code, 4);
}

#[test]
fn settings_that_cannot_be_used_exit_1() {
    // req:verify/verify-due@702ae35c
    let f = Fixture::new(Some("mode = \"pipeline\"\ntriggers = [\"work-done\"]"));
    let err = clm_verify::verify(&f.work(), &Request::Import, &f.env()).unwrap_err();
    assert!(matches!(err, clm_verify::Error::Refused(_)), "{err}");
}

#[test]
fn nothing_to_import_elsewhere() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ));
    f.land(&[("a.txt", "1\n")], "one");
    let out = with_gh(&f, &[], || import(&f, &Request::Import));
    assert_eq!(out.line, "verify nothing to import");
    assert_eq!(out.code, 0);
    assert!(f.lines("gh-calls.log").is_empty());
}

#[test]
fn due_imports_only_when_a_trigger_holds() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    let g = f.land(&[("a.txt", "1\n")], "green");
    f.verified(&g, true);
    let l = f.land(&[("crates/a/x.rs", "1\n")], "l");
    let runs = [run(&l, "completed", "success", 7)];
    let out = with_gh(&f, &runs, || f.due());
    assert!(out.line.starts_with("verify not-due"), "{}", out.line);
    assert!(f.lines("gh-calls.log").is_empty());
    let archive = "---\nstatus: applied\n---\nintent\n";
    let done = f.land(&[("changes/archive/x/change.md", archive)], "archive x");
    let runs = [run(&done, "completed", "success", 8)];
    let out = with_gh(&f, &runs, || f.due());
    assert_eq!(out.line, format!("verify passed {} 2 lands", &done[..7]));
    assert!(f.lines("check-pwd.log").is_empty());
}

#[test]
fn a_host_that_fails_exits_1_with_its_message() {
    // req:verify/verify-import@dd2aac1d
    let f = Fixture::new(Some(IMPORT));
    f.land(&[("a.txt", "1\n")], "one");
    let err = with_gh(&f, &["fail".to_string()], || {
        clm_verify::verify(&f.work(), &Request::Import, &f.env()).unwrap_err()
    });
    let clm_verify::Error::Refused(why) = err else {
        panic!("{err}")
    };
    assert!(why.contains("gh"), "{why}");
}
