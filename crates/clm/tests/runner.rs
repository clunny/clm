//! `clm runner`'s dispatch: the options reach `clm-runner` (orchestrator.runner).

use std::process::Command;

fn clm(args: &[&str]) -> std::process::Output {
    let dir = tempfile::tempdir().unwrap();
    Command::new(env!("CARGO_BIN_EXE_clm"))
        .args(args)
        .current_dir(dir.path())
        .output()
        .unwrap()
}

#[test]
fn missing_permission_mode_is_the_runners_refusal() {
    // req:orchestrator.runner/runner-prerequisites@fd6e7608
    let out = clm(&["runner", "--once"]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("--permission-mode"),
        "{out:?}"
    );
}

#[test]
fn interval_must_be_a_positive_number() {
    for bad in ["0", "-1", "x", "NaN"] {
        let interval = format!("--interval={bad}");
        let out = clm(&["runner", "--permission-mode", "m", &interval]);
        assert_eq!(out.status.code(), Some(2), "{bad}: {out:?}");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("--interval"),
            "{bad}: {out:?}"
        );
    }
}
