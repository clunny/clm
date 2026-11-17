//! A land whose running binary's file is gone still copies a binary (`land/land-binary-copy`).

#![cfg(target_os = "linux")]

mod common;

use common::Fixture;

/// Set in the child the scenario runs in, a process started from a scratch copy of this test
/// binary.
const CHILD: &str = "CLM_TEST_DELETED_EXE_CHILD";

/// A scratch directory beside this test binary, inside the checkout's build directory, so a
/// copy of the binary runs there even where the system's temporary directory is `noexec`.
fn beside_self() -> tempfile::TempDir {
    let exe = std::env::current_exe().unwrap();
    tempfile::Builder::new()
        .prefix("deleted-exe-")
        .tempdir_in(exe.parent().unwrap())
        .unwrap()
}

#[test]
fn the_running_binarys_file_deleted() {
    // req:land/land-binary-copy@a11966b0
    if std::env::var_os(CHILD).is_none() {
        let scratch = beside_self();
        let copy = scratch.path().join("deleted-exe");
        std::fs::copy(std::env::current_exe().unwrap(), &copy).unwrap();
        let out = std::process::Command::new(&copy)
            .args(["--exact", "the_running_binarys_file_deleted", "--nocapture"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && said.contains("1 passed"),
            "{said}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(!copy.exists(), "the child deletes its own file");
        return;
    }
    // In the child: the file this process started from goes before the land starts.
    let own = std::env::current_exe().unwrap();
    std::fs::remove_file(&own).unwrap();
    let gone = std::env::current_exe().unwrap();
    assert!(gone.to_string_lossy().ends_with(" (deleted)"), "{gone:?}");

    let f = Fixture::new("");
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    // `CLM_GATE` and `CLM_CHECK` name passing fakes; with `CLM_AFFECTED` unset the land
    // still makes its copy.
    let mut vars = f.vars(&[]);
    vars.remove("CLM_AFFECTED");
    let env = move |key: &str| vars.get(key).cloned();
    clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    let origin = f.path("origin.git");
    let main = f.remote("main");
    assert_eq!(f.parents(&origin, &main), [old, head]);
    assert!(!own.exists(), "the scratch file came back");
}

/// Set in the child the scenario below runs in.
const REPLACED: &str = "CLM_TEST_REPLACED_EXE_CHILD";

#[test]
fn the_running_binarys_file_replaced() {
    // req:land/land-binary-copy@a11966b0
    if std::env::var_os(REPLACED).is_none() {
        let scratch = beside_self();
        let copy = scratch.path().join("replaced-exe");
        std::fs::copy(std::env::current_exe().unwrap(), &copy).unwrap();
        let out = std::process::Command::new(&copy)
            .args([
                "--exact",
                "the_running_binarys_file_replaced",
                "--nocapture",
            ])
            .env(REPLACED, "1")
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stdout);
        assert!(
            out.status.success() && said.contains("1 passed"),
            "{said}\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        return;
    }
    // In the child: a new file takes the place of the one this process started from, as the
    // dev install replaces `clm`.
    let f = Fixture::new("");
    let own = std::env::current_exe().unwrap();
    std::fs::remove_file(&own).unwrap();
    let log = f.path("replacement.log");
    std::fs::write(
        &own,
        format!("#!/bin/sh\necho \"$1\" >> '{}'\n", log.display()),
    )
    .unwrap();
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&own, std::fs::Permissions::from_mode(0o755)).unwrap();
    let head = f.branch("impl/a", &[("b.txt", "b\n")]);
    let old = f.remote("main");
    let mut vars = f.vars(&[]);
    vars.remove("CLM_AFFECTED");
    let env = move |key: &str| vars.get(key).cloned();
    clm_land::land(&f.work(), Some("impl/a"), &env).unwrap();
    let main = f.remote("main");
    assert_eq!(f.parents(&f.path("origin.git"), &main), [old, head]);
    // The land copied the replacement, so its `clm affected` ran the new file.
    let ran = std::fs::read_to_string(&log).unwrap_or_default();
    assert!(ran.lines().any(|l| l == "affected"), "{ran:?}");
}
