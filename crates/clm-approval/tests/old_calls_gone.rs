//! The calls `clm-land` moved off are gone (land.approval/old-calls-gone): a crate calling
//! either no longer compiles against `clm-approval`, while one calling their replacements does.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The newest `lib<name>-*.rlib` beside this test's binary: the one this build linked.
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

/// Whether `source`, a library crate, compiles against this build's `clm_approval`, with
/// the compiler's message when it doesn't. It runs where this crate's toolchain is pinned.
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
        .arg(format!(
            "clm_approval={}",
            rlib(deps, "clm_approval").display()
        ))
        .arg(&file)
        .output()
        .unwrap();
    match out.status.success() {
        true => Ok(()),
        false => Err(String::from_utf8_lossy(&out.stderr).into_owned()),
    }
}

#[test]
fn the_old_calls_are_gone() {
    // req:land.approval/old-calls-gone@7bcc3552
    // The replacements compile, so a failure below is the call's and not the harness's.
    let replacements = "pub fn f(repo: &clm_approval::Repo<'_>) {
        let _ = repo.held(todo!(), \"spec/x\", \"f\", None);
        let _ = repo.branch_head(\"spec/x\", true);
    }";
    compiles(replacements).expect("the replacements compile");
    let free_held = "pub fn f() { let _ = clm_approval::held(todo!(), \"spec/x\", \"f\"); }";
    let why = compiles(free_held).expect_err("the free `held` is gone");
    // Refused for the name alone: no function `held` at the crate's top.
    assert!(why.contains("error[E0425]"), "{why}");
    let head = "pub fn f(repo: &clm_approval::Repo<'_>) { let _ = repo.head(\"spec/x\", true); }";
    let why = compiles(head).expect_err("`Repo::head` is gone");
    // No method `head` on `Repo`.
    assert!(why.contains("error[E0599]"), "{why}");
}
