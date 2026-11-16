//! `clm wrap-up` by its name, run as the `clm` binary. The tests of `orchestrator.checkpoint`'s
//! requirements call `clm-checkpoint` in its own crate.

#[path = "../../clm-orchestrator/tests/common/mod.rs"]
mod common;

use std::process::Command;

use common::*;

#[test]
fn wrap_up_failure_names_the_new_command() {
    // req:clm/renamed-commands@164ad075
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(["wrap-up", "--root", "/nonexistent-dir"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.starts_with("clm wrap-up: "), "{stderr}");
}

#[test]
fn scenario_wrap_up_by_its_new_name() {
    // req:clm/renamed-commands@164ad075
    let r = repo();
    let out = output(
        Command::new(env!("CARGO_BIN_EXE_clm"))
            .args(["wrap-up", "--root"])
            .arg(&r.work),
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{text}");
    assert_eq!(text.lines().last(), Some("wrap-up ok"), "{text}");
}
