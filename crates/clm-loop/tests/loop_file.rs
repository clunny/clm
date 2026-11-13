//! The loop file (`loop-file`).

mod common;

use std::fs;
use std::process::Command;

use clm_loop::{Kind, RunState, start, step};
use common::{Fixture, line, run};

#[test]
fn state_survives_the_session() {
    // req:orchestrator.loop/loop-file@821217fb
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.set_lease(20);

    // A new session adopts the loop once the lease lapsed.
    let actions = start(&fx.options(), "opus", false).unwrap();
    let launch = line(&actions, "launch build:a build sonnet ");
    assert!(launch.ends_with("build-a.md"), "{launch}");

    // The file still lists the run, as relaunched, and holds the session's model.
    let file = fx.file();
    assert_eq!(file.runs.len(), 1);
    assert_eq!(file.runs[0].name, "build:a");
    assert_eq!(file.session_model, "opus");
    assert!(file.lease.is_some());
    // Everything the loop keeps is in the file and round-trips whole.
    let text = fs::read_to_string(fx.loop_dir().join("loop")).unwrap();
    let parsed: clm_loop::LoopFile = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed, file);
}

/// Runs `damaged_file_child` in a process of its own, so its stderr is the loop's.
#[test]
fn damaged_file() {
    // req:orchestrator.loop/loop-file@821217fb
    let fx = Fixture::new();
    let loop_path = fx.loop_dir().join("loop");
    fs::create_dir_all(fx.loop_dir()).unwrap();
    fs::write(&loop_path, b"\x00not a loop file").unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "damaged_file_child", "--nocapture"])
        .env("LOOP_TEST_ROOT", &fx.work)
        .env("LOOP_TEST_CLM", fx.fake.join("clm"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("started afresh"), "{stderr}");
    assert_eq!(
        fs::read(fx.loop_dir().join("loop.damaged")).unwrap(),
        b"\x00not a loop file"
    );
}

#[test]
fn damaged_file_child() {
    let (Some(root), Some(clm)) = (
        std::env::var_os("LOOP_TEST_ROOT"),
        std::env::var_os("LOOP_TEST_CLM"),
    ) else {
        return;
    };
    let options = clm_loop::Options {
        clm: clm.into(),
        root: root.into(),
        max_builds: 3,
        agent_harness: clm_agent_harness::AgentHarness::ClaudeCode,
    };
    start(&options, "opus", false).unwrap();
}

#[test]
fn no_lock_across_a_subprocess() {
    // req:orchestrator.loop/loop-file@821217fb
    let fx = Fixture::new();
    fx.set("next", "a\n");
    fx.set("lock", fx.loop_dir().join("loop.lock").to_str().unwrap());
    fs::create_dir_all(fx.loop_dir()).unwrap();
    let actions = step(&fx.options()).unwrap();
    line(&actions, "launch build:a ");
    // The fake `clm run` tried to take the lock without blocking while the step prepared it.
    assert_eq!(
        fs::read_to_string(fx.fake.join("lock-result"))
            .unwrap()
            .trim(),
        "free"
    );
}
