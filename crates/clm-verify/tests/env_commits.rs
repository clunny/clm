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
    // req:verify/verify-env@fea9542d
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
    // req:verify/verify-env@fea9542d
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
