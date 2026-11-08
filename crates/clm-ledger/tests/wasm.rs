use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `cargo <args>` for `clm-ledger` with its default features off, for wasm.
fn cargo(args: &[&str]) -> Output {
    Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args(args)
        .args([
            "-p",
            "clm-ledger",
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
    // req:ledger/ledger-builds-for-wasm@85a2d194
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
fn wasm_build_leaves_out_git() {
    // req:ledger/ledger-builds-for-wasm@85a2d194
    // What only reading and writing branches needs, which can't run in a Worker.
    let out = cargo(&["tree", "-e", "normal", "--prefix", "none"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let tree = String::from_utf8(out.stdout).unwrap();
    for name in ["clm-git ", "clm-config ", "sha2 ", "getrandom "] {
        assert!(!tree.contains(name), "{name}in:\n{tree}");
    }
}

/// `tests/<test>.rs` on the host, with the default features off.
fn test_without_default_features(test: &str) {
    let dir = workspace().join("target/no-default-test");
    let out = Command::new(env!("CARGO"))
        .current_dir(workspace())
        .args([
            "test",
            "-p",
            "clm-ledger",
            "--no-default-features",
            "--locked",
        ])
        .args(["--test", test, "--target-dir", dir.to_str().unwrap()])
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn doc_only_without_default_features() {
    // req:ledger/doc-only@4dc00eb4
    test_without_default_features("doc_only");
}

#[test]
fn cleared_without_default_features() {
    // req:ledger/request-cleared@0fe654c6
    test_without_default_features("cleared");
}

#[test]
fn fold_records_is_the_fold() {
    // req:ledger/ledger-builds-for-wasm@85a2d194
    let line = r#"{"id":"1","kind":"landed","writer":"x/1","at":"2026-01-01T00:00:00.000Z","commit":"c","branch":"b","land_kind":"spec","review":false,"fingerprint":"f","changes":["x"]}"#;
    let fold = clm_ledger::fold_records(&[("origin/clm/ledger/x/1", line)]);
    let land = &fold.lands["c"][0];
    assert_eq!(
        land.body,
        clm_ledger::Body::Landed {
            commit: "c".into(),
            branch: "b".into(),
            kind: clm_ledger::LandKind::Spec,
            review: false,
            fingerprint: "f".into(),
            code_key: None,
            changes: vec!["x".into()],
            check_seconds: None,
            steps: vec![],
        }
    );
}
