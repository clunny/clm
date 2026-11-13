//! The human's queue (`humans-queue`).

mod common;

use std::fs;

use clm_loop::{
    BacklogRow, Batch, Escalation, Kind, RunState, Snapshot, StopMode, plan, step, stop,
    stored_plan,
};
use common::{Fixture, line, run};

fn row(slug: &str, kind: &str, blocks: &[&str], since: u64) -> BacklogRow {
    BacklogRow {
        slug: slug.to_string(),
        kind: kind.to_string(),
        blocks: blocks.iter().map(|b| b.to_string()).collect(),
        verify_failures: Vec::new(),
        since,
    }
}

#[test]
fn an_escalated_build_in_the_queue() {
    // req:orchestrator.loop/humans-queue@b7a7e8fa
    let fx = Fixture::new();
    fs::create_dir_all(fx.work.join("backlog")).unwrap();
    fs::write(
        fx.work.join("backlog/a-stops-on-work.md"),
        "---\ntitle: a stops on work\nkind: question\ncomponents: [a]\nblocks: [a/x]\n---\n\nwhy\n",
    )
    .unwrap();
    // An idea blocks nothing, so the human isn't waited on for it.
    fs::write(
        fx.work.join("backlog/an-idea.md"),
        "---\ntitle: an idea\nkind: idea\ncomponents: [a]\n---\n\nwhy\n",
    )
    .unwrap();
    for args in [
        vec!["add", "-A"],
        vec!["commit", "-q", "-m", "the loop files a question"],
        vec!["push", "-q", "origin", "main"],
    ] {
        clm_git::git(&fx.work, &args).unwrap();
    }
    step(&fx.options()).unwrap();
    let queue = stored_plan(&fx.work).unwrap().waiting_on_you;
    assert_eq!(queue.len(), 1, "{queue:?}");
    assert_eq!(queue[0].name, "a-stops-on-work");
    assert_eq!(queue[0].action, "answer backlog/a-stops-on-work");
    assert!(queue[0].what.contains("a/x"));
}

fn snapshot(file: clm_loop::LoopFile) -> Snapshot {
    Snapshot {
        now: 100_000,
        file,
        ..Snapshot::default()
    }
}

#[test]
fn every_kind_of_entry_oldest_first() {
    // req:orchestrator.loop/humans-queue@b7a7e8fa
    let mut file = clm_loop::LoopFile::default();
    file.batches.push(Batch {
        id: "b".to_string(),
        approved: false,
        drafts: Vec::new(),
        since: 90_000,
    });
    file.batches.push(Batch {
        id: "done".to_string(),
        approved: true,
        drafts: Vec::new(),
        since: 1,
    });
    file.escalations.push(Escalation {
        slug: "c-stops".to_string(),
        body: String::new(),
        since: 50_000,
    });
    file.conditions.disk = true;
    let mut s = snapshot(file);
    s.backlog = vec![
        row("q", "question", &["a/x"], 10_000),
        row("h", "human-task", &["b/y"], 20_000),
    ];
    s.unsafe_work = vec!["unpushed spec/x".to_string()];
    s.upgrades = vec!["20261107-m".to_string()];

    let queue = plan(&s).waiting_on_you;
    let actions: Vec<(&str, &str)> = queue
        .iter()
        .map(|e| (e.name.as_str(), e.action.as_str()))
        .collect();
    assert_eq!(
        actions,
        [
            ("q", "answer backlog/q"),
            ("h", "answer backlog/h"),
            ("c-stops", "fix the land of loop/c-stops"),
            ("b", "say \"approve b\""),
            ("unpushed spec/x", "push it"),
            ("20261107-m", "run /clm:upgrade"),
            ("disk", "free disk, then say so"),
        ],
        "oldest first, what is only the loop's left out"
    );
    assert_eq!(queue[0].since_minutes, (100_000 - 10_000) / 60);
}

#[test]
fn a_red_verify_until_an_item_takes_it_in() {
    // req:orchestrator.loop/humans-queue@b7a7e8fa
    let mut file = clm_loop::LoopFile::default();
    file.red_verifies
        .push("verify failed 1a2b3c4 culprit 9f8e7d6 components a".to_string());
    let mut s = snapshot(file);
    let queue = plan(&s).waiting_on_you;
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].action, "take in with /clm:triage");
    assert!(queue[0].what.contains("1a2b3c4"));

    let mut item = row("verify-red", "bug", &[], 5);
    item.verify_failures = vec!["1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b".to_string()];
    s.backlog.push(item);
    assert!(plan(&s).waiting_on_you.is_empty());
}

#[test]
fn unsafe_work_from_wrap_up() {
    // req:orchestrator.loop/humans-queue@b7a7e8fa
    let fx = Fixture::new();
    fx.set(
        "wrap-up",
        "unpushed spec/x\ndirty work/a\nelsewhere dirty /other\nopen spec/z\norigin/main: ledger/unrecorded-land: abc\n",
    );
    fx.set("upgrade", "");
    step(&fx.options()).unwrap();
    let names: Vec<String> = stored_plan(&fx.work)
        .unwrap()
        .waiting_on_you
        .into_iter()
        .map(|e| e.name)
        .collect();
    assert_eq!(
        names,
        [
            "unpushed spec/x",
            "dirty work/a",
            "origin/main: ledger/unrecorded-land: abc"
        ]
    );
}

#[test]
fn the_summary_withholds_the_all_clear_while_work_is_unsafe() {
    // req:orchestrator.loop/end-summary@512bea55
    let fx = Fixture::new();
    fx.add_run(run(
        "build:a",
        Kind::Build,
        "sonnet",
        RunState::Running,
        Some("t-1"),
    ));
    fx.set("wrap-up", "unpushed spec/x\n");
    stop(&fx.options(), StopMode::Now).unwrap();
    step(&fx.options()).unwrap();
    let actions =
        clm_loop::returned(&fx.options(), "build:a", "none", Some("stopped"), None).unwrap();
    let text = fs::read_to_string(line(&actions, "say ").strip_prefix("say ").unwrap()).unwrap();
    assert!(text.contains("build:a"), "{text}");
    assert!(text.contains("stopped"), "{text}");
    assert!(text.contains("unpushed spec/x"), "{text}");
    assert!(!text.contains("the session can end"), "{text}");
}
