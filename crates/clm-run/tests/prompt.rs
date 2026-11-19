//! The prompt `clm run` builds, read as text.

use clm_run::{Blocked, Item, Job, SKILLS, prompt};

fn job() -> Job {
    Job {
        node: "b".into(),
        spec: "specs/b/SPEC.md".into(),
        skills: SKILLS
            .iter()
            .map(|s| (s.to_string(), format!("/pin/skills/{s}/SKILL.md")))
            .collect(),
        pending: vec![
            Item {
                id: "b/x".into(),
                version: Some("22222222".into()),
                state: "behind".into(),
                diffs: vec![
                    ("11111111".into(), Some("@@ -1 +1 @@\n-old\n+new\n".into())),
                    ("none".into(), None),
                ],
                markers: vec!["crates/c/t.rs:8".into()],
                foreign: vec![],
                delete: vec![],
                keep: vec![],
                elsewhere: None,
            },
            Item {
                id: "b/y".into(),
                version: Some("33333333".into()),
                state: "unbuilt".into(),
                diffs: vec![],
                markers: vec![],
                foreign: vec![],
                delete: vec![],
                keep: vec![],
                elsewhere: None,
            },
            Item {
                id: "b/gone".into(),
                version: None,
                state: "obsolete".into(),
                diffs: vec![],
                markers: vec!["t.rs:9".into()],
                foreign: vec![],
                delete: vec![],
                keep: vec![],
                elsewhere: None,
            },
        ],
        blocked: vec![
            Blocked {
                id: "b/z".into(),
                by: vec!["backlog/q".into()],
                cited_in: vec![],
            },
            Blocked {
                id: "b/w".into(),
                by: vec!["c/z".into()],
                cited_in: vec![],
            },
        ],
        bundle: Some("# Spine\nbundle text".into()),
        resumed: false,
        check: None,
        review_model: None,
        capability_tests: false,
        read_from_main: false,
        slot: 2,
        setup: None,
        wiring: false,
    }
}

#[test]
fn names_the_branch_skills_plan_and_bundle() {
    // req:orchestrator.run/run@5c35d326
    let p = prompt(&job());
    let first = p.lines().next().unwrap();
    assert!(first.contains("git switch impl/b"), "{p}");
    for (skill, path) in &job().skills {
        assert!(
            p.contains(&format!("`{skill}`, at `{path}`")),
            "{skill}: {p}"
        );
    }
    assert!(p.contains(".clm/plans/b.md"), "{p}");
    assert!(p.ends_with("# Spine\nbundle text"), "{p}");
}

#[test]
fn scenario_prompt_pr_body() {
    // req:orchestrator.run/prompt-pr-body@4349e259
    let p = prompt(&job());
    assert!(p.contains("clm pr open impl/b"), "{p}");
}

#[test]
fn scenario_prompt_serves() {
    // req:orchestrator.run/prompt-serves@1c7d5c95
    let p = prompt(&job());
    assert!(
        !p.contains("public boundary"),
        "only the capability-tests component is told: {p}"
    );
}

#[test]
fn capability_tests_job_is_told_its_capabilities() {
    // req:orchestrator.run/prompt-serves@1c7d5c95
    let p = prompt(&Job {
        capability_tests: true,
        ..job()
    });
    assert!(p.contains("public boundary"), "{p}");
}

#[test]
fn scenario_prompt_workaround() {
    // req:orchestrator.run/prompt-workaround@21eb0fa0
    let p = prompt(&job());
    assert!(p.contains("clm workaround b/"), "{p}");
}

#[test]
fn scenario_check_command_named() {
    // req:orchestrator.run/prompt-check-command@8a568bf6
    let mut j = job();
    j.check = Some("scripts/check.sh".into());
    let p = prompt(&j);
    // Both places the prompt says to pass the checks name the command, and the
    // configured command is named only as what it runs.
    for needle in [
        "re-run the checks: run `clm land --check-only`, which runs `scripts/check.sh`",
        "pass the repo's checks again with `clm land --check-only`, which runs `scripts/check.sh`",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    assert!(!p.contains("run `scripts/check.sh`"), "{p}");
    assert!(!p.contains("with `scripts/check.sh`"), "{p}");
}

#[test]
fn scenario_no_check_command() {
    // req:orchestrator.run/prompt-check-command@8a568bf6
    let p = prompt(&job());
    assert!(!p.contains("--check-only"), "{p}");
}

#[test]
fn names_no_target_directory() {
    // req:orchestrator.run/prompt-check-command@8a568bf6
    let mut j = job();
    j.check = Some("scripts/check.sh".into());
    for p in [prompt(&j), prompt(&job())] {
        assert!(!p.contains("CARGO_TARGET_DIR"), "{p}");
        assert!(!p.contains("target/build"), "{p}");
    }
}

#[test]
fn scenario_behind_and_obsolete() {
    // req:orchestrator.run/pending-prompt@d236f866
    let p = prompt(&job());
    let pending = p.find("## Pending requirements").unwrap();
    let list = &p[pending..];
    let x = list.find("`b/x` behind `22222222`").expect("b/x behind");
    let y = list.find("`b/y` unbuilt `33333333`").expect("b/y unbuilt");
    let gone = list.find("`b/gone` obsolete").expect("b/gone obsolete");
    assert!(x < y && y < gone, "most urgent first, as given:\n{list}");
    assert!(
        list.contains("```diff\n@@ -1 +1 @@\n-old\n+new\n```"),
        "{list}"
    );
    assert!(list.contains("version `none` wasn't found"), "{list}");
    assert!(list.contains("markers are at t.rs:9"), "{list}");
    assert!(
        list.contains("Re-cite each marker") && list.contains("another component's `paths`"),
        "{list}"
    );
    assert!(list.contains("crates/c/t.rs:8"), "{list}");
}

#[test]
fn scenario_an_obsolete_requirement_s_tests() {
    // req:orchestrator.run/pending-prompt@d236f866
    let mut j = job();
    let gone = &mut j.pending[2];
    gone.delete = vec!["t.rs::old_rule".into()];
    gone.keep = vec!["t.rs::kept_rule".into()];
    let p = prompt(&j);
    let list = &p[p.find("- `b/gone` obsolete").expect("b/gone obsolete")..];
    let list = &list[..list.find("\nBlocked").unwrap_or(list.len())];
    for needle in [
        "Remove the code they mark that no other requirement needs",
        "Delete the tests an approved list names (`land/deleted-tests-approved`): `t.rs::old_rule`.",
        "`clm land` refuses this build to lose every other test that cites it: `t.rs::kept_rule`;",
        "re-cite each to the requirement that now states what it proves",
        "where none does, leave it and the code it runs and add a `question` item whose \
         `blocks` names `b/gone` (`model/parse-backlog`)",
    ] {
        assert!(list.contains(needle), "{needle} missing from {list}");
    }
    // A list with nothing in it isn't printed.
    let mut j = job();
    j.pending[2].keep = vec!["t.rs::kept_rule".into()];
    let p = prompt(&j);
    assert!(
        !p.contains("Delete the tests an approved list names"),
        "{p}"
    );
    assert!(p.contains("`t.rs::kept_rule`"), "{p}");
    let mut j = job();
    j.pending[2].delete = vec!["t.rs::old_rule".into()];
    let p = prompt(&j);
    assert!(!p.contains("refuses this build to lose"), "{p}");
    assert!(p.contains("`t.rs::old_rule`"), "{p}");
}

#[test]
fn scenario_stale_marker_in_a_file_the_build_may_not_change() {
    // req:orchestrator.run/pending-prompt@d236f866
    let mut j = job();
    j.wiring = true;
    let x = &mut j.pending[0];
    x.markers.clear();
    x.foreign = vec![("crates/c/t.rs:8".into(), "c".into())];
    let p = prompt(&j);
    let list = &p[p.find("## Pending requirements").unwrap()..];
    let x = &list[list.find("`b/x` behind `22222222`").expect("b/x behind")
        ..list.find("`b/y` unbuilt").unwrap()];
    for needle in [
        "`crates/c/t.rs:8` in `c`'s file",
        "`clm land` refuses this build's edit",
        "`land/build-holds-own-files`",
        "`clm check` reports",
        "unless an exemption covers its file",
        "`check/tests-cite-their-owner`",
        "add a `question` item whose `blocks` names `b/x`",
    ] {
        assert!(x.contains(needle), "{needle} missing from {x}");
    }
    assert!(!x.contains("Re-cite"), "{x}");
}

#[test]
fn the_fixed_text_allows_a_re_cite_in_another_components_file_only_without_wiring() {
    // req:orchestrator.run/pending-prompt@d236f866
    let without = prompt(&job());
    assert!(
        without.contains("wherever its file sits, is allowed"),
        "{without}"
    );
    let mut j = job();
    j.wiring = true;
    let with = prompt(&j);
    assert!(!with.contains("wherever its file sits"), "{with}");
    assert!(
        with.contains("Re-cite a marker of your own requirement in your own files"),
        "{with}"
    );
}

#[test]
fn scenario_blocked_requirement_left_out() {
    // req:orchestrator.run/pending-prompt@d236f866
    let p = prompt(&job());
    let list = &p[p.find("## Pending requirements").unwrap()..];
    assert!(!list.contains("- `b/z` unbuilt"), "{list}");
    assert!(list.contains("`b/z`, blocked by `backlog/q`"), "{list}");
    assert!(list.contains("`b/w`, blocked by `c/z`"), "{list}");
}

#[test]
fn a_line_waiting_on_its_own_id_names_the_components_that_cite_it() {
    // req:orchestrator.run/pending-prompt@d236f866
    let mut j = job();
    j.blocked = vec![
        Blocked {
            id: "b/gone".into(),
            by: vec![],
            cited_in: vec!["c".into(), "d".into()],
        },
        Blocked {
            id: "b/old".into(),
            by: vec!["backlog/q".into()],
            cited_in: vec!["c".into()],
        },
    ];
    let p = prompt(&j);
    assert!(
        p.contains("- `b/gone`, waiting on `c` and `d`, whose files still cite it\n"),
        "{p}"
    );
    assert!(
        p.contains(
            "- `b/old`, blocked by `backlog/q`, and waiting on `c`, whose files still cite it\n"
        ),
        "{p}"
    );
}

#[test]
fn resume_prompt_says_the_branch_exists() {
    // req:orchestrator.run/run@5c35d326
    let mut j = job();
    j.resumed = true;
    assert!(prompt(&j).contains("`impl/b` already exists"));
    assert!(!prompt(&job()).contains("already exists"));
}

#[test]
fn scenario_capability_tests_component_waits_for_a_changed_rule() {
    // req:orchestrator.run/capability-waits-for-changed-rule@831f33e5
    let mut j = job();
    j.capability_tests = true;
    let p = prompt(&j);
    for needle in [
        "a rule it exercises is still `behind`",
        "stop at that capability and leave it pending",
        "`serving-rules-order-capabilities`",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn scenario_other_component_has_no_capability_wait() {
    // req:orchestrator.run/capability-waits-for-changed-rule@831f33e5
    let p = prompt(&job());
    assert!(!p.contains("serving-rules-order-capabilities"), "{p}");
    assert!(!p.contains("leave it pending"), "{p}");
}

#[test]
fn scenario_prompt_slot() {
    // req:orchestrator.run/prompt-slot@7c75a139
    for resumed in [false, true] {
        let mut j = job();
        j.resumed = resumed;
        assert!(prompt(&j).contains("CLM_SLOT=2"));
    }
    let mut j = job();
    j.slot = 7;
    assert!(prompt(&j).contains("CLM_SLOT=7"));
}

fn with_setup(resumed: bool) -> Job {
    let mut j = job();
    j.resumed = resumed;
    j.setup = Some("scripts/setup.sh".into());
    j
}

#[test]
fn scenario_setup_declared() {
    // req:orchestrator.run/prompt-setup@1c7d490f
    for resumed in [false, true] {
        let p = prompt(&with_setup(resumed));
        // Before the first place the prompt tells the session to run tests by hand.
        let at = p.find("`clm setup`").expect("names clm setup");
        for later in ["Build each pending requirement", "Your build's slot is"] {
            assert!(at < p.find(later).unwrap(), "{later}: {p}");
        }
    }
}

#[test]
fn scenario_setup_nothing_declared() {
    // req:orchestrator.run/prompt-setup@1c7d490f
    for resumed in [false, true] {
        let mut j = job();
        j.resumed = resumed;
        let p = prompt(&j);
        assert!(!p.contains("clm setup"), "{p}");
        assert!(p.contains("declares no setup step"), "{p}");
    }
}
