use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `cargo <args>` for `clm-orchestrator` with its default features off, for wasm.
fn cargo(args: &[&str]) -> Output {
    let root = workspace();
    Command::new(env!("CARGO"))
        .current_dir(&root)
        .args(args)
        .args([
            "-p",
            "clm-orchestrator",
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
fn wasm_build() {
    // req:orchestrator/orchestrator-builds-for-wasm@a279f633
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
fn wasm_build_leaves_out_git_and_files() {
    // req:orchestrator/orchestrator-builds-for-wasm@a279f633
    // These compile for wasm but only serve the git commands, which fail there at run
    // time. `clm-check` without its `checkout` feature stays: the queue reads requirement
    // states through it.
    let out = cargo(&["tree", "-e", "normal", "--prefix", "none"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8(out.stdout).unwrap();
    for name in ["tempfile ", "clm-apply ", "serde_json "] {
        assert!(!tree.contains(name), "{name}in:\n{tree}");
    }
}
