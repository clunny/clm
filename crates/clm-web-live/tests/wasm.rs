//! The crate builds for wasm with its `durable` feature, where `clm-web` links it, and
//! natively without it, where these tests run.

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
    // req:web.live/builds-for-wasm@7c0fd37d
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
fn needs_no_getrandom() {
    // req:web.live/builds-for-wasm@7c0fd37d
    // The randomness is a parameter, so the crypto builds for wasm without `getrandom`,
    // with `durable` (the build `clm-web` links) and without it (where `worker` stays out).
    let tree = |features: &[&str]| {
        let out = wasm_cargo(&[&["tree", "-e", "normal", "--prefix", "none"], features].concat());
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    let durable = tree(&["--features", "durable"]);
    assert!(durable.contains("worker "), "{durable}");
    assert!(!durable.contains("getrandom"), "{durable}");
    let plain = tree(&[]);
    assert!(plain.contains("p256 "), "{plain}");
    assert!(!plain.contains("getrandom"), "{plain}");
    assert!(!plain.contains("worker "), "without durable: {plain}");
}
