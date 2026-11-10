//! The place `[verify] check` builds in (verify/verify-build-dir).

mod common;

use std::sync::Once;

use common::{Fixture, write_exec};

/// The caller's environment: a `CLM_LAND_DIR` and a `CARGO_TARGET_DIR` of its own. Every
/// test here sets the same values, once, before it starts a check.
fn caller_env() {
    static SET: Once = Once::new();
    SET.call_once(|| {
        // SAFETY: runs once, before any check of this binary starts, and sets the same
        // values whichever test gets here first.
        unsafe {
            std::env::set_var("CLM_LAND_DIR", "/elsewhere");
            std::env::set_var("CARGO_TARGET_DIR", "/mine");
            std::env::set_var("CARGO_INCREMENTAL", "1");
        }
    });
}

fn every_land() -> Fixture {
    caller_env();
    Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ))
}

fn main_build_dir(f: &Fixture) -> String {
    std::fs::canonicalize(f.work())
        .unwrap()
        .join(".clm/verify-build")
        .display()
        .to_string()
}

#[test]
fn a_runs_place() {
    // req:verify/verify-build-dir@d44a94b5
    let f = every_land();
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    f.land(&[("a.txt", "2\n")], "two");
    let out_file = f.path("env.out");
    write_exec(
        &f.path("bin/check"),
        &format!(
            "#!/bin/sh\necho \"$CLM_LAND_DIR\" > '{0}'\necho \"$CARGO_TARGET_DIR\" >> '{0}'\n\
             echo \"$CARGO_INCREMENTAL\" >> '{0}'\n",
            out_file.display()
        ),
    );
    // From a linked worktree of the clone.
    let linked = f.path("linked");
    f.git(
        &f.work(),
        &[
            "worktree",
            "add",
            "-q",
            "-b",
            "side",
            linked.to_str().unwrap(),
        ],
    );
    let out = clm_verify::verify(
        &linked,
        &clm_verify::Request::Due {
            work_done: false,
            dry_run: false,
        },
        &f.env(),
    )
    .unwrap();
    assert_eq!(out.code, 0, "{}", out.line);
    assert_eq!(
        f.lines("env.out"),
        [main_build_dir(&f), "/mine".to_string(), "1".to_string()]
    );
}

#[test]
fn a_bisect_probes_place() {
    // req:verify/verify-build-dir@d44a94b5
    let f = every_land();
    let green = f.land(&[("a.txt", "0\n")], "zero");
    f.verified(&green, true);
    f.land(&[("a.txt", "1\n")], "one");
    f.land(&[("crates/a/x.rs", "x\n")], "two");
    write_exec(
        &f.path("bin/check"),
        &format!(
            "#!/bin/sh\necho \"$CLM_LAND_DIR\" >> '{}'\n[ ! -e crates/a/x.rs ]\n",
            f.path("dirs.out").display()
        ),
    );
    let out = f.due();
    assert_eq!(out.code, 1, "{}", out.line);
    let lines = f.lines("dirs.out");
    // The verification itself and its one probe.
    assert_eq!(lines.len(), 2, "{lines:?}");
    assert!(lines.iter().all(|l| *l == main_build_dir(&f)), "{lines:?}");
}
