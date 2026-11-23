//! The check runner is offered only in the form that takes its caller's name
//! (land/runner-names-its-caller): a crate calling the unnamed `run_check` no longer compiles
//! against `clm-land`, while the same call through `run_check_as` with a name does.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The newest `lib<name>-*.rlib` beside this test's binary. Building this test rebuilds the
/// crate's rlib from the source as it stands, so the newest holds that source; an older one,
/// another profile's or feature set's, may not.
fn rlib(deps: &Path, name: &str) -> PathBuf {
    let prefix = format!("lib{name}-");
    std::fs::read_dir(deps)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let file = p.file_name().unwrap().to_string_lossy();
            file.starts_with(&prefix) && file.ends_with(".rlib")
        })
        .max_by_key(|p| p.metadata().unwrap().modified().unwrap())
        .unwrap_or_else(|| panic!("no {prefix}*.rlib in {}", deps.display()))
}

/// Whether `source`, a library crate, compiles against this build's `clm_land`, with the
/// compiler's message when it doesn't. It runs where this crate's toolchain is pinned.
fn compiles(source: &str) -> Result<(), String> {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("caller.rs");
    std::fs::write(&file, source).unwrap();
    let out = Command::new("rustc")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "--edition",
            "2024",
            "--crate-type",
            "lib",
            "--emit=metadata",
            "-A",
            "warnings",
        ])
        .arg("-o")
        .arg(dir.path().join("libcaller.rmeta"))
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("--extern")
        .arg(format!("clm_land={}", rlib(deps, "clm_land").display()))
        .arg(&file)
        .output()
        .unwrap();
    match out.status.success() {
        true => Ok(()),
        false => Err(String::from_utf8_lossy(&out.stderr).into_owned()),
    }
}

#[test]
fn the_unnamed_form_is_gone() {
    // req:land/runner-names-its-caller@360a611c
    // The named form compiles, so a failure below is the call's and not the harness's.
    let named = "pub fn f() {
        let env = clm_land::CheckEnv::default();
        let _ = clm_land::run_check_as(\"clm x\", \"true\", std::path::Path::new(\".\"), None, &env, None);
    }";
    compiles(named).expect("the named form compiles");
    let unnamed = "pub fn f() {
        let env = clm_land::CheckEnv::default();
        let _ = clm_land::run_check(\"true\", std::path::Path::new(\".\"), None, &env, None);
    }";
    let why = compiles(unnamed).expect_err("`run_check` is gone");
    // Refused for the name alone: no function `run_check` at the crate's top.
    assert!(why.contains("error[E0425]"), "{why}");
}
