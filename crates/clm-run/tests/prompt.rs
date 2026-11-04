//! The prompt `clm run` builds, read as text.

use clm_run::{BUDGET, Item, Job, SKILLS, prompt};

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
            },
            Item {
                id: "b/y".into(),
                version: Some("33333333".into()),
                state: "unbuilt".into(),
                diffs: vec![],
                markers: vec![],
            },
            Item {
                id: "b/gone".into(),
                version: None,
                state: "obsolete".into(),
                diffs: vec![],
                markers: vec!["t.rs:9".into()],
            },
        ],
        blocked: vec![
            ("b/z".into(), vec!["backlog/q".into()]),
            ("b/w".into(), vec!["c/z".into()]),
        ],
        bundle: Some("# Spine\nbundle text".into()),
        resumed: false,
        check: None,
        review_model: None,
        capability_tests: false,
        read_from_main: false,
    }
}

#[test]
fn names_the_skills_plan_review_and_sync_points() {
    // req:orchestrator.run/run@9445ebd9
    let p = prompt(&job());
    let (first, rest) = p.split_once('\n').unwrap();
    assert_eq!(
        first,
        "Run `git switch impl/b` and check that `git branch --show-current` prints `impl/b` \
         before doing anything else."
    );
    assert!(
        rest.trim_start().starts_with("You are running unattended"),
        "{p}"
    );
    for (skill, path) in &job().skills {
        assert!(
            p.contains(&format!("`{skill}`, at `{path}`")),
            "{skill}: {p}"
        );
    }
    for needle in [
        "confirm a tool call",
        "not a question for you to wait on",
        "`landed <sha>`",
        "`stopped: <reason>`",
        "read and follow that file",
        "never invoke an installed copy",
        "superpowers:writing-plans",
        "superpowers:test-driven-development",
        "superpowers:verification-before-completion",
        ".clm/plans/b.md",
        "specs/b/SPEC.md",
        "executing-plans",
        "subagent-driven-development",
        "using-git-worktrees",
        "finishing-a-development-branch",
        "/code-review medium",
        "clm:spec-auditor",
        "## Plan",
        "## Review",
        "## Sync",
        "clm rebase",
        "spec changed b",
        "clm pending b",
        "Then land with `clm land`",
        "A line `landed <sha> ...` means the branch is on `main`",
        "`awaiting review: ...`",
        "`gate/up-to-date`, or a conflict with `main`",
        "A failing check means fix it",
        "When the same failure comes back after a fix, or for anything else, stop",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    assert!(p.ends_with("# Spine\nbundle text"), "{p}");
}

#[test]
fn scenario_prompt_commands() {
    // req:orchestrator.run/prompt-commands@9008204d
    let p = prompt(&job());
    for needle in ["rebase ok", "rebase failed"] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    for forbidden in [
        "one command per Bash call",
        "`cd`",
        "exits 0",
        "exits 2",
        "exit code",
    ] {
        assert!(!p.contains(forbidden), "{forbidden} in {p}");
    }
    assert!(
        p.contains(
            "When Claude Code refuses a write, don't write it another way, such as through \
             Bash: treat it as a step only a human can take"
        ),
        "{p}"
    );
}

#[test]
fn scenario_prompt_pr_body() {
    // req:orchestrator.run/prompt-pr-body@ac48f44f
    let p = prompt(&job());
    for needle in [
        "`Write` to `.clm/pr-body.md`",
        "clm pr open impl/b --title <title> --body-file .clm/pr-body.md",
        "clm pr body .clm/pr-body.md",
        "Then land with `clm land`",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    for forbidden in [
        "gh pr create",
        "--body ",
        "gh pr edit",
        "clm watch",
        "gh run rerun",
    ] {
        assert!(!p.contains(forbidden), "{forbidden} in {p}");
    }
}

#[test]
fn scenario_ask_by_backlog() {
    // req:orchestrator.run/ask-by-backlog@7db6f6f9
    let p = prompt(&job());
    for needle in [
        "kind `question` or `human-task`",
        "`blocks` listing the requirements",
        "Build the rest",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn scenario_delete_fixed_items() {
    // req:orchestrator.run/delete-fixed-items@7f2d8d6c
    let p = prompt(&job());
    assert!(
        p.contains(
            "fixes an item that a live change lists in `resolves`, delete its \
             `backlog/<slug>.md` on your own branch, this one, so the requirements it blocked \
             are released when that branch lands"
        ),
        "{p}"
    );
    assert!(!p.contains("own PR"), "{p}");
}

#[test]
fn scenario_deferred_work_done() {
    // req:orchestrator.run/delete-fixed-items@7f2d8d6c
    let p = prompt(&job());
    for needle in [
        "does the work of a `deferred` item whose `components` name your component",
        "`Resolves: backlog/<slug>`",
        "delete its `backlog/<slug>.md` on your own branch",
        "`question`, `human-task` or `idea` item you can't delete that way goes under `## Backlog` in the pull request body",
        "it under `## Backlog` in the pull request body for a spec branch to make",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn scenario_scratch_files() {
    // req:orchestrator.run/scratch-files@a7e8c4bd
    let mut resumed = job();
    resumed.resumed = true;
    for p in [prompt(&job()), prompt(&resumed)] {
        let para = p
            .split("\n\n")
            .find(|x| x.contains("running unattended"))
            .expect("preamble paragraph");
        for needle in [
            ".clm/",
            "scratch",
            "scratchpad",
            "any other directory shared with",
            "beside your plan and pull request body",
        ] {
            assert!(para.contains(needle), "{needle} missing from {para}");
        }
    }
}

#[test]
fn scenario_prompt_serves() {
    // req:orchestrator.run/prompt-serves@bf9d2c22
    let p = prompt(&job());
    for needle in [
        "# Capabilities served",
        "An architectural constraint is proven by the rules its `#### Enforced by` names \
         and is never cited by a test",
        "names the entry it serves",
    ] {
        assert!(p.contains(needle), "{needle}: {p}");
    }
    assert!(!p.contains(concat!("req", ":architecture/")), "{p}");
    assert!(!p.contains("architecture requirement"), "{p}");
    assert!(
        !p.contains("public boundary"),
        "only the capability-tests component is told: {p}"
    );
}

#[test]
fn capability_tests_job_is_told_its_capabilities() {
    // req:orchestrator.run/prompt-serves@bf9d2c22
    let p = prompt(&Job {
        capability_tests: true,
        ..job()
    });
    for needle in [
        "This component builds the capability tests: each capability under that heading is one \
         its tests prove at the project's public boundary",
        "the capability component has no rules to serve them",
    ] {
        assert!(p.contains(needle), "{needle}: {p}");
    }
}

#[test]
fn scenario_prompt_workaround() {
    // req:orchestrator.run/prompt-workaround@70f30126
    for job in [
        job(),
        Job {
            check: Some("scripts/check.sh".into()),
            resumed: true,
            ..job()
        },
    ] {
        let p = prompt(&job);
        for needle in [
            "clm workaround b/<slug> --title <title>",
            "on stdin",
            "you needed a special case or workaround to satisfy a scenario",
            "A flag changes nothing about the work",
            "faking something the requirement doesn't mention",
            "Design of another component",
            "raise a spec change",
            "can't observe",
            "commit the file with your work",
            "never blocks",
            "re-record it at the new version: delete your flag file and run `clm workaround` again",
            "or delete it when the new text removes the workaround",
            "requirement you flagged on this branch",
            "## Workarounds",
            "under `## Workarounds` in the pull request body list your workaround flags",
            "never blocks the build or the land, with one exception",
            "When `clm workaround` exits 3, also when you re-record a flag",
            "that isn't a failure, so keep the flag",
            "stop working on that requirement",
            "a `question` item that names the capability and blocks the requirement",
            "build the rest",
        ] {
            assert!(p.contains(needle), "{needle} missing from {p}");
        }
        assert!(
            p.contains("`## Plan`, `## Review`, `## Sync`, `## Workarounds`, `## Fixes in other components`, `## Left pending` and `## CLM defects`"),
            "{p}"
        );
    }
}

#[test]
fn scenario_workaround_binary_without_the_command() {
    // req:orchestrator.run/prompt-workaround@70f30126
    for job in [
        job(),
        Job {
            check: Some("scripts/check.sh".into()),
            resumed: true,
            ..job()
        },
    ] {
        let p = prompt(&job);
        assert!(
            p.contains(
                "When the `clm` on your `PATH` lacks `clm workaround`, list the flag under \
                 `## Workarounds` alone, write no flag file by hand, and carry on."
            ),
            "{p}"
        );
    }
}

#[test]
fn scenario_no_shared_install() {
    // req:orchestrator.run/no-shared-install@4715e1bb
    let p = prompt(&job());
    for needle in [
        "Never install or replace the `clm` binary on your `PATH`",
        "`cargo install`",
        "the orchestrating session and the other builds run that binary",
        "When that binary is missing or lacks a command the work needs, stop and say why",
        "such as with `cargo run`",
        "installing other tools",
        "A `clm` that runs but only lacks `clm workaround` is not such a case: carry on",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn scenario_removal_breaks_dependant() {
    // req:orchestrator.run/removal-breaks-dependant@fb1ba857
    let p = prompt(&job());
    let obsolete = p
        .find("For an `obsolete` requirement, remove the code and tests its markers mark")
        .expect("the general obsolete instruction stays");
    let rule = p
        .find("Never edit another component's code or tests to absorb what your component removes from or adds to its Contract, except")
        .expect("the removal rule");
    assert!(obsolete < rule, "beside the obsolete instruction: {p}");
    for needle in [
        "leave the requirement unbuilt",
        "add a `question` backlog item whose `components` name both components",
        "with `blocks` naming the requirement",
        "saying which component uses what",
        "the CLI's dispatch, the workspace manifest and its lockfile, aren't another component's code",
        "the compile-only and test-fixture fixes the removal or the addition forces (`architecture/path-ownership`, `forced-fixes-cover-additions`)",
        "only name what you remove",
        "an assertion on a removed field or state",
        "and nothing that component's code does depends on it",
        "name each one under `## Fixes in other components` in your pull request body",
        "still uses what the requirement removes, or because what it adds needs more there than such a fix",
        "keep the old path working and that component building as it stands",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn scenario_addition_forces_a_fix() {
    // req:orchestrator.run/removal-breaks-dependant@fb1ba857
    let p = prompt(&job());
    for needle in [
        "or only build a value that gains a field (a struct literal, in code or in a fixture, that must now name the added field)",
        "sets that field to the value under which the provider behaves as before",
        "so the other component's behaviour doesn't change",
        "A rename is not such a fix (`rename-adds-then-removes`)",
        "neither is an arm for an added variant that another component matches exhaustively",
        "an added field with no value under which the provider behaves as before",
        "since those decide what the other component does",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn removal_rule_for_an_obsolete_requirement_is_in_every_prompt() {
    // req:orchestrator.run/removal-breaks-dependant@fb1ba857
    let mut j = job();
    j.pending.retain(|i| i.state != "obsolete");
    let p = prompt(&j);
    for needle in [
        "keep the code the other component still uses",
        "no `clm:remove-by` marker",
        "add a `deferred` item without `blocks`",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn push_retries_transient_rejections_only() {
    // req:orchestrator.run/prompt-commands@9008204d
    let p = prompt(&job());
    for needle in [
        "`remote rejected` with `(failure)`",
        "`remote: fatal error`",
        "wait a few seconds and push again, at most twice more, before you stop",
        "`pre-receive hook declined`",
        "`stale info`",
        "Never retry",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    // Said once, where the prompt first names `--force-with-lease`.
    let first = p.find("--force-with-lease").unwrap();
    let retry = p.find("`remote rejected`").unwrap();
    assert!(first < retry, "{p}");
    assert_eq!(p.matches("`remote rejected`").count(), 1, "{p}");
}

#[test]
fn scenario_budget_in_land_terms() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let p = prompt(&job());
    assert!(
        p.contains(&format!(
            "you may open your pull request and land once the tree is green with at most {BUDGET} \
             requirements built"
        )),
        "{p}"
    );
    assert!(
        p.contains("When `clm rebase` reports `spec changed b`"),
        "{p}"
    );
    for forbidden in ["open its PR", "open your PR"] {
        assert!(!p.contains(forbidden), "{forbidden} in {p}");
    }
}

#[test]
fn scenario_check_command_named() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    let mut j = job();
    j.check = Some("scripts/check.sh".into());
    let p = prompt(&j);
    // Both places, after the review's fixes and before landing, are the first and last
    // needles below; the mentions aren't counted, since `background-work-stopped` names
    // the command too.
    for needle in [
        "re-run the checks: run `clm land --check-only`, which runs `scripts/check.sh`, the \
         command `clm.toml` names, and then `clm check`, as `clm land` does",
        "from the worktree's top level as one plain command, exactly as written",
        "adding no `cd`, absolute path, variable, pipe or redirection",
        "When Claude Code refuses or denies that command",
        "report the refusal with `clm defect`",
        "list it under `## CLM defects`",
        "land with `clm land`, which runs the whole command",
        "never report them as the check",
        "pass the repo's checks again with `clm land --check-only`, which runs \
         `scripts/check.sh` and then `clm check`, as `clm land` does,",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
    // Never told to run the configured command by its path: it is named only as what
    // `clm land --check-only` runs, once in each place.
    assert_eq!(p.matches("`scripts/check.sh`").count(), 2, "{p}");
    assert_eq!(p.matches("which runs `scripts/check.sh`").count(), 2, "{p}");
    assert!(!p.contains("run `scripts/check.sh`"), "{p}");
    assert!(!p.contains("with `scripts/check.sh`"), "{p}");
    // `cd` is named only for that one command (`prompt-commands`).
    assert_eq!(p.matches("`cd`").count(), 1, "{p}");
    assert!(p.contains("adding no `cd`"), "{p}");
}

#[test]
fn scenario_tree_held_during_the_check() {
    // req:orchestrator.run/check-tree-still@c2b25402
    let mut j = job();
    j.check = Some("scripts/check.sh".into());
    let p = prompt(&j);
    for needle in [
        "While `clm land --check-only` runs, in the foreground or the background, change \
         nothing in your worktree: no commit, no edit to a tracked file, no `clm rebase` and \
         no `git` command that moves `HEAD`",
        "judges the working tree as it stands",
        "a check that reads `HEAD`",
        "leave a verdict on a tree you no longer have",
        "Work outside git, such as writing `.clm/pr-body.md`, may go on meanwhile",
        "run `clm land --check-only` again once the check has ended",
    ] {
        assert!(p.contains(needle), "{needle} missing from {p}");
    }
}

#[test]
fn scenario_tree_not_held_without_check_command() {
    // req:orchestrator.run/check-tree-still@c2b25402
    let p = prompt(&job());
    assert!(
        !p.to_lowercase()
            .contains("while `clm land --check-only` runs"),
        "{p}"
    );
}

#[test]
fn scenario_no_check_command() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    let p = prompt(&job());
    assert!(!p.contains("one plain command"), "{p}");
    assert!(!p.contains("--check-only"), "{p}");
    assert!(p.contains("re-run the checks."), "{p}");
    assert!(p.contains("pass the repo's checks again,"), "{p}");
}

#[test]
fn names_no_target_directory() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    let mut j = job();
    j.check = Some("scripts/check.sh".into());
    for p in [prompt(&j), prompt(&job())] {
        assert!(!p.contains("CARGO_TARGET_DIR"), "{p}");
        assert!(!p.contains("target/build"), "{p}");
    }
}

#[test]
fn scenario_behind_and_obsolete() {
    // req:orchestrator.run/pending-prompt@dea4856a
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
    assert!(
        list.contains("remove the code and tests marked at t.rs:9"),
        "{list}"
    );
    assert!(
        list.contains("Re-cite each marker") && list.contains("another component's `paths`"),
        "{list}"
    );
    assert!(list.contains("crates/c/t.rs:8"), "{list}");
    assert!(p.contains("req:<id>@<version>"), "{p}");
    assert!(p.contains("## Left pending"), "{p}");
    assert!(p.contains(&format!("at most {BUDGET} requirements")), "{p}");
    assert!(p.contains("Nothing is re-planned"), "{p}");
}

#[test]
fn scenario_blocked_requirement_left_out() {
    // req:orchestrator.run/pending-prompt@dea4856a
    let p = prompt(&job());
    let list = &p[p.find("## Pending requirements").unwrap()..];
    assert!(!list.contains("- `b/z` unbuilt"), "{list}");
    assert!(list.contains("`b/z`, blocked by `backlog/q`"), "{list}");
    assert!(list.contains("`b/w`, blocked by `c/z`"), "{list}");
    assert!(list.contains("build nothing that relies on them"), "{list}");
}

#[test]
fn scenario_background_work_stopped() {
    // req:orchestrator.run/background-work-stopped@46642229
    let mut checked = job();
    checked.check = Some("scripts/check.sh".into());
    let mut resumed = job();
    resumed.resumed = true;
    for p in [prompt(&job()), prompt(&checked), prompt(&resumed)] {
        let paras: Vec<&str> = p
            .split("\n\n")
            .filter(|para| para.contains("`stopped: <reason>`"))
            .collect();
        let [para] = paras[..] else {
            panic!("one final-message paragraph: {paras:?}");
        };
        for needle in [
            "completion notice",
            "`pgrep -f`",
            "a setting of the tool call, not a change to the command",
            "Before you write your final message, landed or stopped, stop every other \
             background task you started that is still running",
            "wait for every subagent you launched to return",
        ] {
            assert!(para.contains(needle), "{needle} missing from {para}");
        }
        for forbidden in [
            "`cd`",
            "exit code",
            "clm watch",
            "TaskStop",
            "TaskOutput",
            "KillShell",
            "`Monitor`",
        ] {
            assert!(!para.contains(forbidden), "{forbidden} in {para}");
        }
    }
}

#[test]
fn scenario_a_land_still_running() {
    // req:orchestrator.run/background-work-stopped@46642229
    let mut checked = job();
    checked.check = Some("scripts/check.sh".into());
    for p in [prompt(&job()), prompt(&checked)] {
        let paras: Vec<&str> = p
            .split("\n\n")
            .filter(|para| para.contains("`stopped: <reason>`"))
            .collect();
        let [para] = paras[..] else {
            panic!("one final-message paragraph: {paras:?}");
        };
        for needle in [
            "Wait for a `clm land` you started",
            "however long that takes",
            "queue",
            "many minutes",
            "Never write your final message, landed or stopped, while one of those is still running",
            "never stop one",
            "frees your place and tidies your worktree",
        ] {
            assert!(para.contains(needle), "{needle} missing from {para}");
        }
    }
    assert!(prompt(&checked).contains("a `clm land --check-only` you started"));
}

#[test]
fn scenario_a_turn_ends_while_waiting() {
    // req:orchestrator.run/background-work-stopped@46642229
    let mut checked = job();
    checked.check = Some("scripts/check.sh".into());
    let mut resumed = job();
    resumed.resumed = true;
    for p in [prompt(&job()), prompt(&checked), prompt(&resumed)] {
        let paras: Vec<&str> = p
            .split("\n\n")
            .filter(|para| para.contains("`stopped: <reason>`"))
            .collect();
        let [para] = paras[..] else {
            panic!("one final-message paragraph: {paras:?}");
        };
        for needle in [
            "`waiting: <command>`",
            "not your final message",
            "neither `landed <sha>` nor `stopped: <reason>`",
        ] {
            assert!(para.contains(needle), "{needle} missing from {para}");
        }
    }
}

#[test]
fn scenario_why_no_pipe() {
    // req:orchestrator.run/prompt-check-command@6b68f4d5
    let mut j = job();
    j.check = Some("scripts/check.sh".into());
    let p = prompt(&j);
    let paras: Vec<&str> = p
        .split("\n\n")
        .filter(|para| para.contains("exactly as written"))
        .collect();
    let [para] = paras[..] else {
        panic!("one paragraph: {paras:?}");
    };
    for needle in [
        "tail",
        "reports the last command's result instead of the check's",
        "can read as passing",
        "last line",
        "`checks pass`",
        "nothing to gain by cutting it",
        "the name of the command that failed",
        "keeps a long command's output whole",
    ] {
        assert!(para.contains(needle), "{needle} missing from {para}");
    }
    assert!(!prompt(&job()).contains("can read as passing"));
}

#[test]
fn resume_prompt_says_the_branch_exists() {
    // req:orchestrator.run/run@9445ebd9
    let mut j = job();
    j.resumed = true;
    let p = prompt(&j);
    for s in [
        "`impl/b` already exists",
        "Start with `clm rebase`",
        "land the same branch",
        "also run `clm pr body .clm/pr-body.md`",
    ] {
        assert!(p.contains(s), "{s}: {p}");
    }
    assert!(!prompt(&job()).contains("already exists"));
}

#[test]
fn scenario_prompt_spot_checks() {
    // req:orchestrator.run/prompt-spot-checks@96c6ba17
    let mut resumed = job();
    resumed.resumed = true;
    for p in [prompt(&job()), prompt(&resumed)] {
        let paras: Vec<&str> = p.split("\n\n").collect();
        let at = paras
            .iter()
            .position(|x| x.contains("mutation"))
            .expect("mutation paragraph");
        for needle in [
            "commit the work first",
            "compiles",
            "named test",
            "touch",
            "zsh",
            "git status",
            "under `## Review` in the pull request body",
            "## Review",
        ] {
            assert!(
                paras[at].contains(needle),
                "{needle} missing: {}",
                paras[at]
            );
        }
        assert!(!paras[at].contains("spec-auditor"));
        let auditor = paras
            .iter()
            .position(|x| x.contains("spec-auditor"))
            .expect("auditor paragraph");
        assert!(at < auditor, "mutation paragraph must precede the review");
    }
}

#[test]
fn scenario_no_old_command_names() {
    // req:orchestrator.run/prompt-commands@9008204d
    let mut resumed = job();
    resumed.resumed = true;
    let mut checked = job();
    checked.check = Some("scripts/check.sh".into());
    for p in [prompt(&job()), prompt(&resumed), prompt(&checked)] {
        for old in ["sync", "checkpoint", "page"] {
            assert!(!p.contains(&format!("clm {old}")), "clm {old} in {p}");
        }
        for old in ["sync ok", "sync incomplete", "sync failed"] {
            assert!(!p.contains(old), "{old} in {p}");
        }
        assert!(!p.to_lowercase().contains("page"), "page in {p}");
        assert!(p.contains("pull request"), "{p}");
    }
}

#[test]
fn scenario_review_every_diff() {
    // req:orchestrator.run/review-every-diff@12a07b11
    let mut resumed = job();
    resumed.resumed = true;
    for p in [prompt(&job()), prompt(&resumed)] {
        let ok = p.split("\n\n").any(|para| {
            para.contains("spec-auditor")
                && [
                    "/code-review",
                    "whatever the diff's size",
                    "never a reason to skip",
                    "no findings",
                    "Name both reviewers under `## Review`",
                ]
                .iter()
                .all(|needle| para.contains(needle))
        });
        assert!(ok, "no review paragraph says every diff is reviewed: {p}");
    }
}
