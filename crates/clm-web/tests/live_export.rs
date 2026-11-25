//! The Worker's module exports the `Live` Durable Object class `web/wrangler.toml` binds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn builds_for_wasm() {
    // req:web.worker/live-class-export@dab9c53e
    // A separate target dir, so the build doesn't wait on the outer `cargo test`'s lock.
    let root = workspace();
    let dir = root.join("target/wasm-test");
    let out = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args([
            "build",
            "-p",
            "clm-web",
            "--target",
            "wasm32-unknown-unknown",
            "--locked",
        ])
        .arg("--target-dir")
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
    // The module carries the class's wasm-bindgen exports, which `worker-build` turns into
    // the `Live` export `wrangler` binds.
    let wasm = fs::read(dir.join("wasm32-unknown-unknown/debug/clm_web.wasm")).unwrap();
    let export = b"__wbg_live_free";
    assert!(
        wasm.windows(export.len()).any(|w| w == export),
        "clm_web.wasm exports no `Live` class"
    );
}
