//! `verify-env` for bisect probes and `--commit` (verify/verify-env).

mod common;

use common::{Fixture, write_exec};

fn every_land() -> Fixture {
    Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ))
}

fn toml_with(f: &Fixture, env: &str) -> String {
    format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n{env}",
        f.path("bin/check").display()
    )
}

#[test]
fn every_probe_runs_every_test_when_the_table_changed() {
    // req:verify/verify-env@877031f2
    let f = every_land();
    let green = f.land(&[("a.txt", "0\n")], "zero");
    f.verified(&green, true);
    f.land(&[("crates/a/x.rs", "x\n")], "code");
    f.land(
        &[("clm.toml", &toml_with(&f, "[env]\nPROBE_B = \"1\"\n"))],
        "env",
    );
    write_exec(
        &f.path("bin/check"),
        &format!(
            "#!/bin/sh\nif [ -n \"${{CLM_SELECTION-}}\" ]; then echo set; else echo unset; fi >> '{}'\n\
             [ -z \"$PROBE_B\" ]\n",
            f.path("sel.out").display()
        ),
    );
    let out = f.due();
    assert_eq!(out.code, 1, "{}", out.line);
    let lines = f.lines("sel.out");
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines.iter().all(|l| l == "unset"), "{lines:?}");
}

#[test]
fn a_commit_takes_its_own_variables() {
    // req:verify/verify-env@877031f2
    let f = every_land();
    let green = f.land(&[("a.txt", "0\n")], "zero");
    f.verified(&green, true);
    let mid = f.land(
        &[("clm.toml", &toml_with(&f, "[env]\nCOMMIT_A = \"mid\"\n"))],
        "mid",
    );
    f.land(
        &[("clm.toml", &toml_with(&f, "[env]\nCOMMIT_A = \"head\"\n"))],
        "head",
    );
    write_exec(
        &f.path("bin/check"),
        &format!(
            "#!/bin/sh\necho \"$COMMIT_A\" >> '{}'\n",
            f.path("c.out").display()
        ),
    );
    let out = f.run(&clm_verify::Request::Commit(mid));
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(f.lines("c.out"), ["mid"]);
}

/// A `clm.toml` whose `[env]` `config` refuses, since CLM sets `CLM_` variables itself.
const REFUSED: &str = "[env]\nCLM_X = \"1\"\n";

/// A land `g`, with a passing record when `record_g`, then a land `bad` whose `[env]` can't be
/// used, then a land `good` that restores `g`'s `clm.toml` and changes a file of `a`; the check
/// fails on any tree holding `good`'s change.
fn bad_then_good(record_g: bool) -> (Fixture, String, String) {
    let f = every_land();
    let g = f.land(&[("a.txt", "0\n")], "g");
    if record_g {
        f.verified(&g, true);
    }
    let original = std::fs::read_to_string(f.work().join("clm.toml")).unwrap();
    let bad = f.land(&[("clm.toml", &toml_with(&f, REFUSED))], "bad");
    let good = f.land(
        &[("clm.toml", &original), ("crates/a/good.rs", "x\n")],
        "good",
    );
    f.write("fail-path", "crates/a/good.rs");
    (f, bad, good)
}

/// The last `verified` record's commit, result, culprit and components.
fn last_record(f: &Fixture) -> (String, bool, Option<String>, Vec<String>) {
    let records = f.records();
    let Some(clm_ledger::Body::Verified {
        commit,
        passed,
        culprit,
        components,
        ..
    }) = records.last()
    else {
        panic!("{records:?}");
    };
    (commit.clone(), *passed, culprit.clone(), components.clone())
}

#[test]
fn a_commit_whose_table_cant_be_used_is_refused() {
    // req:verify/verify-env@877031f2
    let (f, bad, _) = bad_then_good(true);
    let before = f.records().len();
    let err = common::verify(
        &f.work(),
        &clm_verify::Request::Commit(bad.clone()),
        &f.env(),
    )
    .unwrap_err();
    assert!(matches!(err, clm_verify::Error::Refused(_)), "{err}");
    let message = err.to_string();
    assert!(
        message.contains(&format!("{}:clm.toml", &bad[..7])),
        "{message}"
    );
    assert!(message.contains("env"), "{message}");
    assert!(f.lines("check-pwd.log").is_empty());
    assert_eq!(f.records().len(), before);
}

#[test]
fn a_probe_on_a_land_whose_table_cant_be_used_is_untestable() {
    // req:verify/verify-env@877031f2
    let (f, bad, good) = bad_then_good(true);
    let out = f.due();
    assert_eq!(
        out.line,
        format!("verify failed {} culprit unknown components a", &good[..7])
    );
    assert_eq!(out.code, 1);
    let note = format!(
        "culprit unknown: [verify] check could not run on 1 lands, {0} to {0}, before the \
         first red one, {1}",
        &bad[..7],
        &good[..7]
    );
    assert!(out.notes.contains(&note), "{:?}", out.notes);
    // The check ran on the head alone, never on `bad`.
    assert_eq!(f.lines("check-head.log"), std::slice::from_ref(&good));
    let (commit, passed, culprit, _) = last_record(&f);
    assert_eq!((commit, passed, culprit), (good, false, None));
}

#[test]
fn a_green_commit_whose_table_cant_be_used_differs() {
    // req:verify/verify-env@877031f2
    let (f, bad, good) = bad_then_good(false);
    f.verified(&bad, true);
    let out = f.run(&clm_verify::Request::Commit(good.clone()));
    assert_eq!(out.code, 1, "{}", out.line);
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
    let (commit, passed, culprit, _) = last_record(&f);
    assert_eq!((commit, passed), (good.clone(), false));
    assert_eq!(culprit, Some(good));
}

#[test]
fn a_bisect_of_a_red_commit_whose_table_cant_be_used() {
    // req:verify/verify-env@877031f2
    let f = every_land();
    let g = f.land(&[("a.txt", "0\n")], "g");
    f.verified(&g, true);
    let l1 = f.land(&[("crates/a/l1.rs", "x\n")], "l1");
    let original = std::fs::read_to_string(f.work().join("clm.toml")).unwrap();
    let bad = f.land(&[("clm.toml", &toml_with(&f, REFUSED))], "bad");
    // A later land restores the table: `origin/main`'s own `clm.toml` is loaded first, and
    // refused there it would end every `clm verify` (`settings-from-main`).
    f.land(&[("clm.toml", &original)], "after");
    f.red(&bad, &[&l1, &bad], clm_ledger::VerifyMode::Local);
    f.write("fail-path", "crates/a/l1.rs");
    let out = f.run(&clm_verify::Request::Bisect(bad.clone()));
    assert_eq!(out.code, 1, "{}", out.line);
    // The probe of `l1` ran with no selection, since the red commit's table can't be used.
    assert_eq!(f.lines("check-head.log"), std::slice::from_ref(&l1));
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
    let (commit, passed, culprit, components) = last_record(&f);
    assert_eq!((commit, passed), (bad, false));
    assert_eq!(culprit, Some(l1));
    assert_eq!(components, ["a"]);
}

#[test]
fn a_git_failure_reading_the_table_ends_the_run() {
    // req:verify/verify-env@877031f2
    let f = every_land();
    let green = f.land(&[("a.txt", "0\n")], "g");
    f.verified(&green, true);
    let mid = f.land(&[("a.txt", "1\n")], "mid");
    f.land(&[("a.txt", "2\n")], "head");
    // A `git` first on `PATH` that fails reading `mid`'s `clm.toml` as a broken repository
    // would, and passes everything else to the real git.
    let real = String::from_utf8(
        std::process::Command::new("sh")
            .args(["-c", "command -v git"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let bin = f.path("git-bin");
    std::fs::create_dir_all(&bin).unwrap();
    write_exec(
        &bin.join("git"),
        &format!(
            "#!/bin/sh\nfor a in \"$@\"; do\n  if [ \"$a\" = '{mid}:clm.toml' ]; then\n    \
             echo 'fatal: bad object' >&2; exit 128\n  fi\ndone\nexec '{}' \"$@\"\n",
            real.trim()
        ),
    );
    let old = std::env::var("PATH").unwrap();
    // SAFETY: the tests run one per process, and this one alone sets `PATH`.
    unsafe { std::env::set_var("PATH", format!("{}:{old}", bin.display())) };
    let before = f.records().len();
    let out = common::verify(&f.work(), &clm_verify::Request::Commit(mid), &f.env());
    unsafe { std::env::set_var("PATH", old) };
    let err = out.unwrap_err();
    assert!(matches!(err, clm_verify::Error::Git(_)), "{err}");
    assert!(err.to_string().contains("bad object"), "{err}");
    assert!(f.lines("check-pwd.log").is_empty());
    assert_eq!(f.records().len(), before);
}
