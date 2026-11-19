//! The place `[verify] check` builds in (verify/verify-build-dir).

mod common;

use std::sync::Once;

use common::{Fixture, in_child, write_exec};

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
    // req:verify/verify-build-dir@457f8211
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
    let out = common::verify(
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
    // req:verify/verify-build-dir@457f8211
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

/// A clone with a passing record on a land of `files`, whose `[verify]` runs the script body
/// `check`, with the lines `extra` added to its `clm.toml`, `MUTATE` in them standing for the
/// path of the fixture's `bin/mutate`.
fn with_check(check: &str, files: &[(&str, &str)], extra: &str) -> Fixture {
    caller_env();
    let f = Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ));
    let script = f.path("bin/check");
    let mutate = f.path("bin/mutate");
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n{}",
        script.display(),
        extra.replace("MUTATE", &mutate.display().to_string())
    );
    let mut all = vec![("clm.toml", toml.as_str()), ("a.txt", "1\n")];
    all.extend_from_slice(files);
    let green = f.land(&all, "baseline");
    f.verified(&green, true);
    write_exec(&script, &format!("#!/bin/sh\n{check}\n"));
    f
}

#[test]
fn a_fresh_clones_place() {
    // req:verify/verify-build-dir@457f8211
    let f = with_check("echo ok > \"$CLM_LAND_DIR/x\"", &[], "");
    let place = std::path::PathBuf::from(main_build_dir(&f));
    assert!(!place.exists());
    f.land(&[("a.txt", "2\n")], "two");
    let out = f.due();
    assert!(out.line.starts_with("verify passed"), "{}", out.line);
    assert_eq!(out.code, 0);
    assert_eq!(std::fs::read_to_string(place.join("x")).unwrap(), "ok\n");
}

#[test]
fn the_pass_after_the_cap() {
    // req:verify/verify-build-dir@457f8211
    if let Some(stderr) = in_child("the_pass_after_the_cap") {
        assert!(
            stderr.contains("cleared .clm/verify-build (2 GB, cap 1 GB)"),
            "{stderr}"
        );
        return;
    }
    let f = with_check(
        "truncate -s 2000000000 \"$CLM_LAND_DIR/big\"",
        &[("crates/a/src/lib.rs", "1\n")],
        "mutate = \"MUTATE\"\n[land]\ncheck = \"true\"\ntarget_cap_gb = 1\n",
    );
    let base = f.git(&f.work(), &["rev-parse", "HEAD"]);
    let body = clm_ledger::Body::mutated(base.trim().to_string(), None, vec![], vec![], vec![]);
    clm_ledger::append(&f.work(), body, &f.env()).unwrap();
    write_exec(
        &f.path("bin/mutate"),
        "#!/bin/sh\necho ok > \"$CLM_LAND_DIR/lcov.info\" || exit $?\n\
         echo 'a/thing killed' > \"$CLM_MUTATE_RESULT\"\n",
    );
    let version = clm_model::load(&f.work())
        .0
        .requirement("a/thing")
        .unwrap()
        .version()
        .to_string();
    let test = format!(
        "#[test]\nfn proves_x() {{\n    // {}{version}\n    let _ = 1;\n}}\n",
        concat!("req", ":a/thing@")
    );
    f.land(
        &[
            ("crates/a/src/lib.rs", "2\n"),
            ("crates/a/tests/x.rs", &test),
        ],
        "a new test",
    );
    let out = f.due();
    assert!(out.line.starts_with("verify passed"), "{}", out.line);
    assert!(
        out.notes
            .iter()
            .any(|n| n == "mutation: 1 killed, 0 unproven, 0 not judged"),
        "{:?}",
        out.notes
    );
    assert!(
        !out.notes.iter().any(|n| n.contains("failed with status")),
        "{:?}",
        out.notes
    );
    let place = std::path::PathBuf::from(main_build_dir(&f));
    assert_eq!(
        std::fs::read_to_string(place.join("lcov.info")).unwrap(),
        "ok\n"
    );
}

#[test]
fn a_place_that_cant_be_made() {
    // req:verify/verify-build-dir@457f8211
    if let Some(stderr) = in_child("a_place_that_cant_be_made") {
        assert!(
            stderr
                .lines()
                .any(|l| l.starts_with("clm verify: cannot create .clm/verify-build: ")),
            "{stderr}"
        );
        return;
    }
    let f = with_check("exit 0", &[], "");
    let place = std::path::PathBuf::from(main_build_dir(&f));
    std::fs::create_dir_all(place.parent().unwrap()).unwrap();
    std::fs::write(&place, "a file\n").unwrap();
    f.land(&[("a.txt", "2\n")], "two");
    let out = f.due();
    assert!(out.line.starts_with("verify passed"), "{}", out.line);
    assert_eq!(out.code, 0);
    assert!(matches!(
        f.records().last().unwrap(),
        clm_ledger::Body::Verified { passed: true, .. }
    ));
}
