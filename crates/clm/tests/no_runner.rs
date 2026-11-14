//! The workspace holds no `clm-runner` crate and `clm` no `runner` subcommand (clm/no-runner-crate).

// clm:reads Cargo.toml
// clm:reads crates/clm/Cargo.toml
// clm:reads crates/clm-runner/**

use std::fs;
use std::path::Path;
use std::process::Command;

#[test]
fn scenario_crate_and_command_gone() {
    // req:clm/no-runner-crate@4f3de494
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for manifest in ["Cargo.toml", "crates/clm/Cargo.toml"] {
        let text = fs::read_to_string(root.join(manifest)).unwrap();
        assert!(!text.contains("clm-runner"), "{manifest} names clm-runner");
    }
    assert!(!root.join("crates/clm-runner").exists());

    let dir = tempfile::tempdir().unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_clm"))
        .arg("runner")
        .current_dir(dir.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("unrecognized subcommand"),
        "{out:?}"
    );
}
