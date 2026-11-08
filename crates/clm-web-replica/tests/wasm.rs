//! The crate builds for wasm on its own (`object-class`), not only inside `clm-web`'s build,
//! whose features (such as `clm-web-live`'s `durable`) could hide one this crate doesn't declare.

use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn builds_for_wasm_on_its_own() {
    // req:web.worker.replica/object-class@2ce37c07
    // A separate target dir, so the build doesn't wait on the outer `cargo test`'s lock.
    let dir = workspace().join("target/wasm-test");
    let out = Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args([
            "build",
            "-p",
            "clm-web-replica",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .args(["--locked", "--target-dir"])
        .arg(&dir)
        // Host-only flags (coverage, target-cpu) would break the wasm build.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "wasm build failed (rust-toolchain.toml lists the target; rustup installs it):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
