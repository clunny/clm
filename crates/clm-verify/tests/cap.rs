//! The cap on the build place (verify/verify-build-cap).

mod common;

use clm_verify::Request;
use common::{Fixture, write_exec};

/// Set in the child each scenario re-runs itself in, so its stderr can be read.
const CHILD: &str = "CLM_TEST_CAP_CHILD";

/// A fixture with a passing record, one land since, and `[land] target_cap_gb = 1` when `cap`
/// is set; its check writes `bytes` bytes (sparse) into `CLM_LAND_DIR` and exits `code`.
fn fixture(cap: bool, bytes: u64, code: u8) -> Fixture {
    let f = Fixture::new(Some(
        "mode = \"local\"\ntriggers = []\nevery = { lands = 1 }",
    ));
    let green = f.land(&[("a.txt", "1\n")], "one");
    f.verified(&green, true);
    let check = f.path("bin/check");
    let land = if cap {
        format!(
            "[land]\ncheck = \"{}\"\ntarget_cap_gb = 1\n",
            check.display()
        )
    } else {
        "# the cap is left at its default\n".to_string()
    };
    let toml = format!(
        "[verify]\ncheck = \"{}\"\nmode = \"local\"\ntriggers = []\nevery = {{ lands = 1 }}\n{land}",
        check.display()
    );
    f.land(&[("clm.toml", &toml)], "settings");
    write_exec(
        &check,
        &format!(
            "#!/bin/sh\nmkdir -p \"$CLM_LAND_DIR\"\ntruncate -s {bytes} \"$CLM_LAND_DIR/big\"\nexit {code}\n"
        ),
    );
    f
}

/// Re-runs the test `name` in a child and returns the child's stderr; `None` in the child.
fn in_child(name: &str) -> Option<String> {
    if std::env::var_os(CHILD).is_some() {
        return None;
    }
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success() && stdout.contains("1 passed"),
        "{stdout}"
    );
    Some(String::from_utf8_lossy(&out.stderr).into_owned())
}

fn main_build(f: &Fixture) -> std::path::PathBuf {
    std::fs::canonicalize(f.work())
        .unwrap()
        .join(".clm/verify-build")
}

fn due(f: &Fixture) -> clm_verify::Outcome {
    f.run(&Request::Due {
        work_done: false,
        dry_run: false,
    })
}

const CLEARED: &str = "cleared .clm/verify-build (2 GB, cap 1 GB)";

#[test]
fn over_the_cap() {
    // req:verify/verify-build-cap@010f2d33
    if let Some(stderr) = in_child("over_the_cap") {
        assert!(stderr.contains(CLEARED), "{stderr}");
        return;
    }
    let f = fixture(true, 2_000_000_000, 0);
    let out = due(&f);
    assert!(out.line.starts_with("verify passed"), "{}", out.line);
    assert!(!main_build(&f).exists());
}

#[test]
fn under_the_cap() {
    // req:verify/verify-build-cap@010f2d33
    if let Some(stderr) = in_child("under_the_cap") {
        assert!(!stderr.contains("cleared"), "{stderr}");
        return;
    }
    let f = fixture(false, 10, 0);
    let out = due(&f);
    assert!(out.line.starts_with("verify passed"), "{}", out.line);
    assert!(main_build(&f).join("big").exists());
}

#[test]
fn a_red_run_is_capped_too() {
    // req:verify/verify-build-cap@010f2d33
    if let Some(stderr) = in_child("a_red_run_is_capped_too") {
        assert!(stderr.contains(CLEARED), "{stderr}");
        return;
    }
    let f = fixture(true, 2_000_000_000, 1);
    let out = due(&f);
    assert_eq!(out.code, 1, "{}", out.line);
    assert!(!main_build(&f).exists());
    let records = f.records();
    assert!(matches!(
        records.last().unwrap(),
        clm_ledger::Body::Verified { passed: false, .. }
    ));
}
