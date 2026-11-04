use std::path::Path;
use std::process::Command;

#[test]
fn wasm_build() {
    // req:check/check-builds-for-wasm@3b5bc7cd
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // A separate target dir, so the build doesn't wait on the outer `cargo test`'s lock.
    let out = Command::new(env!("CARGO"))
        .current_dir(&root)
        .args([
            "build",
            "-p",
            "clm-check",
            "--no-default-features",
            "--target",
            "wasm32-unknown-unknown",
            "--locked",
        ])
        .arg("--target-dir")
        .arg(root.join("target/wasm-test"))
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
