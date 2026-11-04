//! `replica/host-neutral`: no crate that reaches a network, a filesystem or a host, and a
//! build for wasm.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const BANNED: [&str; 4] = ["worker", "clm-web-platform", "clm-host", "clm-git"];

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cargo(args: &[&str]) -> Output {
    Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args(args)
        .args(["-p", "clm-replica", "--locked"])
        // Host-only flags (coverage, target-cpu) would break the wasm build.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap()
}

#[test]
fn manifest_names_no_host_crate() {
    // req:replica/host-neutral@5f781488
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let table: toml::Table = std::fs::read_to_string(path).unwrap().parse().unwrap();
    for section in ["dependencies", "dev-dependencies", "build-dependencies"] {
        let Some(deps) = table.get(section).and_then(|d| d.as_table()) else {
            continue;
        };
        for banned in BANNED {
            assert!(!deps.contains_key(banned), "{section} names {banned}");
        }
    }
}

#[test]
fn dependency_tree_names_no_host_crate() {
    // req:replica/host-neutral@5f781488
    let out = cargo(&["tree", "-e", "normal", "--prefix", "none"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8(out.stdout).unwrap();
    for banned in BANNED {
        assert!(
            !tree.contains(&format!("{banned} ")),
            "{banned} in:\n{tree}"
        );
    }
}

#[test]
fn builds_for_wasm() {
    // req:replica/host-neutral@5f781488
    // A separate target dir, so the build doesn't wait on the outer `cargo test`'s lock.
    let dir = workspace().join("target/wasm-test");
    let out = cargo(&[
        "build",
        "--target",
        "wasm32-unknown-unknown",
        "--target-dir",
        dir.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "wasm build failed (rust-toolchain.toml lists the target; rustup installs it):\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
