//! The crate builds for wasm with its `durable` feature, where `clm-web` links it, and for
//! the host without it.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `cargo <args> -p clm-web-live --target wasm32-unknown-unknown`.
fn wasm_cargo(args: &[&str]) -> Output {
    Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args(args)
        .args([
            "-p",
            "clm-web-live",
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
fn builds_for_wasm_with_durable() {
    // req:web.live/builds-for-wasm@f9ab086a
    // A separate target dir, so the build doesn't wait on the outer `cargo test`'s lock.
    let dir = workspace().join("target/wasm-test");
    let out = wasm_cargo(&[
        "build",
        "--features",
        "durable",
        "--target-dir",
        dir.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "wasm build failed (rust-toolchain.toml lists the target; rustup installs it):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn builds_for_the_host() {
    // req:web.live/builds-for-wasm@f9ab086a
    // A separate target dir, as for the wasm build.
    let dir = workspace().join("target/host-test");
    let out = Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args(["build", "-p", "clm-web-live", "--locked", "--target-dir"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "host build failed:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
