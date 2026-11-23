//! The `clm` the loop runs is the one now at its path (`runs-the-clm-now-at-its-path`). Only
//! Linux marks a replaced binary's path with ` (deleted)`.
#![cfg(target_os = "linux")]

mod common;

use std::fs;
use std::path::PathBuf;

use clm_loop::{Options, step};
use common::Fixture;

#[test]
fn the_clm_it_started_from_was_replaced() {
    // req:orchestrator.loop/runs-the-clm-now-at-its-path@97592436
    let fx = Fixture::new();
    fx.set("next", "a\n");
    let deleted = PathBuf::from(format!("{} (deleted)", fx.fake.join("clm").display()));
    assert!(!deleted.exists());
    let options = Options {
        clm: deleted,
        ..fx.options()
    };
    let actions = step(&options).unwrap();
    assert!(
        actions.iter().any(|a| a.starts_with("launch build:a ")),
        "{actions:?}"
    );
    // The fake `clm` at the path without the suffix received the step's subcommands.
    assert!(
        fx.calls().iter().any(|c| c.starts_with("run a ")),
        "{:?}",
        fx.calls()
    );
}

#[test]
fn a_failed_start_names_the_resolved_path() {
    // req:orchestrator.loop/runs-the-clm-now-at-its-path@97592436
    let fx = Fixture::new();
    // A file that exists but can't be run, behind a replaced process's path.
    let blocked = fx.fake.join("blocked");
    fs::write(&blocked, "not a program\n").unwrap();
    let options = Options {
        clm: PathBuf::from(format!("{} (deleted)", blocked.display())),
        ..fx.options()
    };
    let err = format!("{:?}", step(&options).unwrap_err());
    assert!(err.contains(&format!("{}: ", blocked.display())), "{err}");
    assert!(!err.contains("(deleted)"), "{err}");
}
