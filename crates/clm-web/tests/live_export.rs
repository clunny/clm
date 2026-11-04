//! The Worker's module exports the `Live` Durable Object class `web/wrangler.toml` binds.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use toml::Value;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn exports_the_live_class() {
    // req:web.worker/live-class-export@d6ddd6fd
    let manifest: Value = toml::from_str(include_str!("../Cargo.toml")).unwrap();
    let live = manifest
        .get("target")
        .and_then(|t| t.get("cfg(target_arch = \"wasm32\")"))
        .and_then(|t| t.get("dependencies"))
        .and_then(|d| d.get("clm-web-live"))
        .expect("the wasm32 target dependencies name clm-web-live");
    let features: Vec<&str> = live
        .get("features")
        .and_then(Value::as_array)
        .map(|f| f.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    assert!(features.contains(&"durable"), "{live:?}");
    // A line of its own, so a commented-out re-export doesn't count.
    assert!(
        include_str!("../src/lib.rs")
            .lines()
            .any(|l| l.trim() == "pub use clm_web_live::Live;"),
        "lib.rs holds no `pub use clm_web_live::Live;`"
    );
}

#[test]
fn builds_for_wasm() {
    // req:web.worker/live-class-export@d6ddd6fd
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
