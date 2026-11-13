//! `clm build step`'s housekeeping: `clm wrap-up` run in the directory `clm build` runs in.

mod common;

use std::fs;

use clm_loop::{step, stored_plan};
use common::{Fixture, line};

fn queue_names(fx: &Fixture) -> Vec<String> {
    stored_plan(&fx.work)
        .unwrap()
        .waiting_on_you
        .into_iter()
        .map(|e| e.name)
        .collect()
}

#[test]
fn unsafe_work_in_sight() {
    // req:orchestrator.loop/step-housekeeping@772499ae
    let fx = Fixture::new();
    fx.set("wrap-up", "unpushed spec/x\nwrap-up incomplete\n");
    step(&fx.options()).unwrap();
    assert!(queue_names(&fx).contains(&"unpushed spec/x".to_string()));
}

#[test]
fn other_checkouts_work_is_left_out() {
    // req:orchestrator.loop/step-housekeeping@772499ae
    let fx = Fixture::new();
    fx.set(
        "wrap-up",
        "dirty work/a\nelsewhere dirty /other\nopen spec/z\nwrap-up incomplete\n",
    );
    step(&fx.options()).unwrap();
    assert_eq!(queue_names(&fx), ["dirty work/a"]);
}

#[test]
fn left_lines_are_said_once() {
    // req:orchestrator.loop/step-housekeeping@772499ae
    let fx = Fixture::new();
    fx.set("wrap-up", "left /x/wt: directory can't be removed\n");
    let first = step(&fx.options()).unwrap();
    let file = line(&first, "say ").strip_prefix("say ").unwrap();
    let text = fs::read_to_string(file).unwrap();
    assert!(
        text.contains("left /x/wt: directory can't be removed"),
        "{text}"
    );
    let second = step(&fx.options()).unwrap();
    assert!(!second.iter().any(|l| l.starts_with("say ")), "{second:?}");
}

#[test]
fn wrap_up_runs_where_clm_build_runs() {
    // req:orchestrator.loop/step-housekeeping@772499ae
    let fx = Fixture::new();
    let sub = fx.work.join("sub");
    fs::create_dir_all(&sub).unwrap();
    let mut options = fx.options();
    options.cwd = sub.clone();
    step(&options).unwrap();
    let ran_in = fs::read_to_string(fx.fake.join("wrap-up-cwd")).unwrap();
    assert_eq!(ran_in.trim(), sub.to_str().unwrap());
}

#[test]
fn complete_changes_reach_the_archive() {
    // req:orchestrator.loop/step-housekeeping@772499ae
    let fx = Fixture::new();
    fx.set("wrap-up", "complete c\n");
    let actions = step(&fx.options()).unwrap();
    line(&actions, "launch archive:c archive ");
}
