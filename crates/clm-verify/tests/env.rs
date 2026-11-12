//! The variables `[verify] check` runs with (verify/verify-env).

mod common;

use common::{Fixture, write_exec};

fn every_land() -> Fixture {
    Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ))
}

/// A `clm.toml` with the fixture's `[verify]` table and `env` after it.
fn toml_with(f: &Fixture, env: &str) -> String {
    format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n{env}",
        f.path("bin/check").display()
    )
}

#[test]
fn mains_table_and_the_clones() {
    // req:verify/verify-env@fea9542d
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(
        &[(
            "clm.toml",
            &toml_with(&f, "[env]\nVERIFY_A = \"main\"\nVERIFY_B = \"main\"\n"),
        )],
        "env",
    );
    // The checkout's own table is never read; the clone's local file replaces `VERIFY_B`.
    f.write("work/clm.toml", "[env]\nVERIFY_A = \"checkout\"\n");
    f.write("work/clm.local.toml", "[env]\nVERIFY_B = \"local\"\n");
    write_exec(
        &f.path("bin/check"),
        &format!(
            "#!/bin/sh\necho \"$VERIFY_A\" > '{0}'\necho \"$VERIFY_B\" >> '{0}'\n",
            f.path("vars.out").display()
        ),
    );
    // SAFETY: only this test of the binary sets `VERIFY_A`, before the check starts.
    unsafe { std::env::set_var("VERIFY_A", "caller") };
    let out = f.due();
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(f.lines("vars.out"), ["main", "local"]);
}

#[test]
fn a_probe_takes_its_lands_variables() {
    // req:verify/verify-env@fea9542d
    let f = every_land();
    let green = f.land(&[("a.txt", "0\n")], "zero");
    f.verified(&green, true);
    let first = f.land(
        &[("clm.toml", &toml_with(&f, "[env]\nPROBE_A = \"1\"\n"))],
        "one",
    );
    let second = f.land(
        &[("clm.toml", &toml_with(&f, "[env]\nPROBE_A = \"2\"\n"))],
        "two",
    );
    write_exec(
        &f.path("bin/check"),
        &format!(
            "#!/bin/sh\necho \"$PROBE_A\" >> '{}'\n[ \"$PROBE_A\" != 2 ]\n",
            f.path("a.out").display()
        ),
    );
    let out = f.due();
    assert_eq!(out.code, 1, "{}", out.line);
    // The verification of the head, then the probe of the first land.
    assert_eq!(f.lines("a.out"), ["2", "1"]);
    let records = f.records();
    let clm_ledger::Body::Verified { culprit, .. } = records.last().unwrap() else {
        panic!("no verified record");
    };
    assert_eq!(culprit.as_deref(), Some(second.as_str()));
    assert_ne!(culprit.as_deref(), Some(first.as_str()));
}

#[test]
fn a_changed_variable_runs_every_test() {
    // req:verify/verify-env@fea9542d
    let f = every_land();
    let green = f.land(&[("a.txt", "0\n")], "zero");
    f.verified(&green, true);
    f.land(
        &[("clm.toml", &toml_with(&f, "[env]\nX = \"1\"\n"))],
        "env only",
    );
    let out = f.due();
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(f.lines("check-selection.log"), ["unset"]);
}

#[test]
fn an_unchanged_table_keeps_the_selection() {
    // req:verify/verify-env@fea9542d
    let f = every_land();
    let green = f.land(&[("a.txt", "0\n")], "zero");
    f.verified(&green, true);
    f.land(&[("crates/a/x.rs", "x\n")], "code");
    let out = f.due();
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(f.lines("check-selection.log"), ["set"]);
}
