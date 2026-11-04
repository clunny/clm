//! `effort/effort-builds-for-wasm`.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `cargo <args>` for `clm-effort` with its default features off, for wasm.
fn cargo(args: &[&str]) -> Output {
    Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args(args)
        .args([
            "-p",
            "clm-effort",
            "--no-default-features",
            "--target",
            "wasm32-unknown-unknown",
            "--locked",
        ])
        // Host-only flags (coverage, target-cpu) would break the wasm build.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap()
}

#[test]
fn effort_builds_for_wasm() {
    // req:effort/effort-builds-for-wasm@48a17dd6
    // A separate target dir, so the build doesn't wait on the outer `cargo test`'s lock.
    let dir = workspace().join("target/wasm-test");
    let out = cargo(&["build", "--target-dir", dir.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "wasm build failed (rust-toolchain.toml lists the target; rustup installs it):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn effort_wasm_build_leaves_out_git() {
    // req:effort/effort-builds-for-wasm@48a17dd6
    let out = cargo(&["tree", "-e", "normal", "--prefix", "none"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8(out.stdout).unwrap();
    for name in ["clm-git ", "clm-check "] {
        assert!(!tree.contains(name), "{name}in:\n{tree}");
    }
}
