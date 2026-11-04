//! The Worker handles no scheduled event.

use std::path::Path;
use std::process::Command;

#[test]
fn worker_holds_no_scheduled_handler() {
    // req:web.worker/no-scheduled-event@d7b14f2e
    let src = include_str!("../src/worker.rs");
    for needle in [
        concat!("event(", "scheduled)"),
        concat!("Scheduled", "Event"),
        concat!("Schedule", "Context"),
    ] {
        assert!(!src.contains(needle), "worker.rs holds `{needle}`");
    }
}

#[test]
fn the_worker_builds_for_wasm() {
    // req:web.worker/no-scheduled-event@d7b14f2e
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = Command::new(env!("CARGO"))
        .args([
            "build",
            "-p",
            "clm-web",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .current_dir(root)
        // Host-only flags (coverage, target-cpu) would break the wasm build.
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
