//! The `build` skill, `plugin/skills/build/SKILL.md`: what its text tells Claude.
// clm:reads docs/guide/**
// clm:reads CLAUDE.md
// clm:reads README.md
// clm:reads AGENTS.md

use std::fs;
use std::path::{Path, PathBuf};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plugin() -> PathBuf {
    repo().join("plugin")
}

/// The YAML frontmatter and the body of a markdown file.
fn frontmatter(path: &Path) -> (serde_yaml::Value, String) {
    let text = fs::read_to_string(path).unwrap();
    let rest = text.strip_prefix("---\n").expect("frontmatter");
    let (yaml, body) = rest.split_once("\n---\n").expect("frontmatter end");
    (serde_yaml::from_str(yaml).unwrap(), body.to_owned())
}

/// The first sentence of `text` holding `needle`: sentences end at a newline or at a
/// period followed by a space, so file names such as `change.md` don't cut one short.
fn sentence_with<'a>(text: &'a str, needle: &str) -> &'a str {
    text.lines()
        .flat_map(|l| l.split(". "))
        .find(|s| s.contains(needle))
        .unwrap_or_else(|| panic!("no sentence mentions {needle}"))
}

/// The top-level numbered steps of a skill, each with the lines under it.
fn numbered_steps(body: &str) -> Vec<String> {
    let mut steps = Vec::new();
    let mut step: Option<String> = None;
    for line in body.lines() {
        let numbered = line
            .split_once(". ")
            .is_some_and(|(n, _)| !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()));
        // A new number starts a step; other unindented text ends one.
        if numbered || !(line.is_empty() || line.starts_with(' ')) {
            steps.extend(step.take());
        }
        if numbered {
            step = Some(String::new());
        }
        if let Some(step) = step.as_mut() {
            step.push_str(line);
            step.push('\n');
        }
    }
    steps.extend(step);
    steps
}

/// What a skill lacks of `must` and names of `must_not`, one line per gap naming the skill
/// and the text. Empty when it says it all.
fn text_gaps(skill: &str, body: &str, must: &[&str], must_not: &[&str]) -> Vec<String> {
    let mut gaps: Vec<String> = must
        .iter()
        .filter(|needle| !body.contains(*needle))
        .map(|needle| format!("{skill}: SKILL.md must mention {needle}"))
        .collect();
    gaps.extend(
        must_not
            .iter()
            .filter(|needle| body.contains(*needle))
            .map(|needle| format!("{skill}: SKILL.md must not mention {needle}")),
    );
    gaps
}

/// The body of `skill`'s `SKILL.md`.
fn skill_body(skill: &str) -> String {
    frontmatter(&plugin().join(format!("skills/{skill}/SKILL.md"))).1
}

/// Asserts `gaps` reports `needle` for `skill`, and nothing else's.
fn assert_reports(gaps: &[String], skill: &str, needle: &str) {
    assert!(
        !gaps.is_empty()
            && gaps
                .iter()
                .all(|g| g.starts_with(&format!("{skill}:")) && g.contains(needle)),
        "{needle}: {gaps:?}"
    );
}

fn text_files_under(dir: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            text_files_under(&path, out);
        } else if let Ok(text) = fs::read_to_string(&path) {
            let rel = path.strip_prefix(repo()).unwrap_or(&path);
            out.push((rel.to_string_lossy().into_owned(), text));
        }
    }
}

/// What `build-subagents` asks of a `build` skill. Empty when it says it all.
fn subagent_gaps(body: &str) -> Vec<String> {
    let mut gaps: Vec<String> = [
        "clm run <component>",
        "Agent",
        "background",
        "isolation",
        "isolation: \"worktree\"",
        "manual",
        "landed <sha>",
        "stopped:",
        "exits 0",
        "own context",
    ]
    .into_iter()
    .filter(|needle| !body.contains(needle))
    .map(|needle| format!("build: SKILL.md must mention {needle}"))
    .collect();
    // Builds are subagents that land their own branch, so the headless runs' list and
    // logs are gone, and so is watching a PR.
    gaps.extend(
        [
            "clm next --interactive",
            ".clm/logs",
            "clm watch",
            "PR #<number>",
        ]
        .into_iter()
        .filter(|needle| body.contains(needle))
        .map(|needle| format!("build: SKILL.md must not mention {needle}")),
    );
    gaps
}

#[test]
fn build_launches_subagents() {
    // req:plugin.build/build-subagents@002f1bf4
    let (_, body) = frontmatter(&plugin().join("skills/build/SKILL.md"));
    assert_eq!(subagent_gaps(&body), Vec::<String>::new());
    // build without isolation
    let gaps = subagent_gaps(&body.replace("isolation", ""));
    assert!(
        gaps.contains(&"build: SKILL.md must mention isolation".to_owned()),
        "{gaps:?}"
    );
    // build watching
    let gaps = subagent_gaps(&format!("{body}\nRun `clm watch <pr>`.\n"));
    assert_eq!(gaps, ["build: SKILL.md must not mention clm watch"]);
    for old in ["clm next --interactive", ".clm/logs", "PR #<number>"] {
        let gaps = subagent_gaps(&format!("{body}\n`{old}`\n"));
        assert_eq!(gaps, [format!("build: SKILL.md must not mention {old}")]);
    }
    // The subagent is launched only on a clean exit, with the printed prompt.
    let launch = sentence_with(&body, "run `clm run <component> --review-model <alias>`");
    for needle in [
        "clm run <component>",
        "exits 0",
        "stdout",
        "Agent",
        "background",
    ] {
        assert!(launch.contains(needle), "{needle}: {launch}");
    }
    // The mode is mentioned once, before the first build, and never refused.
    let mode = sentence_with(&body, "manual");
    for needle in ["once", "permission mode", "manual mode they pause"] {
        assert!(mode.contains(needle), "{needle}: {mode}");
    }
    assert_eq!(
        body.matches("permission mode").count(),
        1,
        "build: SKILL.md must mention the permission mode once"
    );
    assert!(
        body.find("manual").unwrap() < body.find("clm run <component>").unwrap(),
        "build: SKILL.md must mention the permission mode before the first build"
    );
    assert!(body.contains("Never refuse a mode"), "{body}");
    // The closing line says what happened, and the node's work stays in its build.
    let landed = sentence_with(&body, "`landed <sha>`");
    assert!(landed.contains("closing line"), "{landed}");
    let stopped = sentence_with(&body, "`stopped: <reason>`");
    assert!(stopped.contains("tell the user"), "{stopped}");
    let own = sentence_with(&body, "own context");
    assert!(own.contains("Never"), "{own}");
    // A build that stopped without landing is built again the same way, until it lands or
    // stops twice for the same reason.
    let retry = sentence_with(&body, "stopped without landing");
    for needle in [
        "same way",
        "until it lands",
        "twice",
        "same reason",
        "isn't one you resume",
    ] {
        assert!(retry.contains(needle), "{needle}: {retry}");
    }
}

/// What `build-finishes-builds` asks of a `build` skill.
fn finish_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "clm run --finish <component>",
            "exits 3",
            "no free build place",
            "max_builds",
            "clm.local.toml",
            "left <path>",
        ],
        &["CARGO_TARGET_DIR", "git worktree prune"],
    )
}

#[test]
fn build_finishes_builds() {
    // req:plugin.build/build-finishes-builds@ade8f7bd
    let body = skill_body("build");
    assert_eq!(finish_gaps(&body), Vec::<String>::new());
    // build without finishing
    let gaps = finish_gaps(&body.replace("clm run --finish", "elided"));
    assert_reports(&gaps, "build", "clm run --finish");
    let gaps = finish_gaps(&format!("{body}\nSet `CARGO_TARGET_DIR`.\n"));
    assert_reports(&gaps, "build", "CARGO_TARGET_DIR");
    let gaps = finish_gaps(&format!("{body}\nRun `git worktree prune`.\n"));
    assert_reports(&gaps, "build", "git worktree prune");
    let gaps = finish_gaps(&body.replace("left <path>", "elided"));
    assert_reports(&gaps, "build", "left <path>");
    // A worktree `clm run --finish` couldn't remove goes to the human as printed.
    let left = sentence_with(&body, "left <path>");
    for needle in [
        "to the human as it stands",
        "couldn't remove",
        "go on, leaving the directory",
        "to them",
    ] {
        assert!(left.contains(needle), "{needle}: {left}");
    }
    // Builds launch only from the latest `clm next`, which runs again after each returns.
    let latest = sentence_with(&body, "latest `clm next`");
    assert!(latest.contains("launch builds only"), "{latest}");
    let again = sentence_with(&body, "run `clm next` again");
    assert!(again.contains("returns"), "{again}");
    // Exit 3 launches nothing; finishing comes first, whether it landed or stopped.
    let full = sentence_with(&body, "When `clm run` exits 3");
    for needle in ["no free place", "launch nothing for that component"] {
        assert!(full.contains(needle), "{needle}: {full}");
    }
    let finish = sentence_with(&body, "clm run --finish <component>");
    for needle in [
        "landed or stopped",
        "before retrying or launching",
        "usage or rate limit",
        "`resume-sound-builds`",
    ] {
        assert!(finish.contains(needle), "{needle}: {finish}");
    }
    let cap = sentence_with(&body, "max_builds");
    for needle in [
        "none of your own builds is running",
        "cap is reached",
        "other sessions' builds or drafts",
        "run step 5",
        "in place of an empty queue",
        "clm next --wait",
        "whichever of the two said so",
    ] {
        assert!(cap.contains(needle), "{needle}: {cap}");
    }
    // A full cap on `clm next` is no empty queue.
    let stderr = sentence_with(&body, "no free build place");
    for needle in [
        "prints nothing",
        "stderr",
        "treat it as no free place",
        "never tell the human nothing is left",
    ] {
        assert!(stderr.contains(needle), "{needle}: {stderr}");
    }
    // Steps 4 and 6 tell the full cap from a drained queue.
    for needle in [
        "with no `no free build place` line on stderr",
        "or only the `no free build place` line",
    ] {
        assert!(body.contains(needle), "{needle}");
    }
    // full cap read as an empty queue
    let gaps = finish_gaps(&body.replace("no free build place", "elided"));
    assert_reports(&gaps, "build", "no free build place");
    // A build's worktree is `clm run --finish`'s to remove, never `git worktree remove`'s.
    for line in body.lines().flat_map(|l| l.split(". ")) {
        if line.contains("git worktree remove") {
            assert!(line.contains(".claude/worktrees/archive"), "{line}");
        }
    }
}

/// What `build-archive-branch` asks of a `build` skill: the nested branch, and
/// never a bare `impl/archive`, which the queue would read as a claim on a node `archive`.
fn archive_branch_gaps(body: &str) -> Vec<String> {
    let mut gaps = text_gaps("build", body, &["impl/archive/<id>"], &[]);
    let bare = body
        .match_indices("impl/archive")
        .any(|(at, n)| !body[at + n.len()..].starts_with('/'));
    if bare {
        gaps.push(
            "build: SKILL.md names a bare `impl/archive`; name `impl/archive/<id>`".to_owned(),
        );
    }
    gaps
}

#[test]
fn build_archives_on_a_nested_branch() {
    // req:plugin.build/build-archive-branch@1f264572
    let body = skill_body("build");
    assert_eq!(archive_branch_gaps(&body), Vec::<String>::new());
    // archive branch read as a claim
    for bare in [
        "on `impl/archive` cut",
        "on impl/archive cut",
        "on impl/archive",
    ] {
        let gaps = archive_branch_gaps(&format!("{body}\nArchive {bare}.\n"));
        assert_reports(&gaps, "build", "impl/archive/<id>");
    }
    let named = sentence_with(&body, "`impl/archive/<id>`, `<id>`");
    for needle in ["first change", "origin/main"] {
        assert!(named.contains(needle), "{needle}: {named}");
    }
    let refused = sentence_with(&body, "already exists on `origin`");
    for needle in ["delete your local branch", "leave the archive"] {
        assert!(refused.contains(needle), "{needle}: {refused}");
    }
}

/// What `build-reads-origin` asks of a `build` skill.
fn origin_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "origin/main",
            "clm status",
            "views-name-their-commit",
            "origin/impl/<component>",
            "clm pending --at origin/main",
        ],
        &[],
    )
}

#[test]
fn build_reads_origin() {
    // req:plugin.build/build-reads-origin@c1f35a76
    let body = skill_body("build");
    assert_eq!(origin_gaps(&body), Vec::<String>::new());
    for needle in [
        "views-name-their-commit",
        "origin/impl/<component>",
        "clm pending --at origin/main",
    ] {
        let gaps = origin_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "build", needle);
    }
    let state = sentence_with(&body, "session-start");
    for needle in ["clm next", "origin/main", "clm status", "clm view", "never"] {
        assert!(state.contains(needle), "{needle}: {state}");
    }
    let trust = sentence_with(&body, "disagree");
    assert!(trust.contains("`clm next` is right"), "{trust}");
    let pull = sentence_with(&body, "Never pull, merge or sync");
    assert!(pull.contains("own checkout"), "{pull}");
    let build = sentence_with(&body, "origin/impl/<component>");
    for needle in ["clm wrap-up", "never", "git -C <worktree>"] {
        assert!(build.contains(needle), "{needle}: {build}");
    }
    let summary = sentence_with(&body, "clm pending --at origin/main");
    for needle in ["clm next", "without `--at`", "question"] {
        assert!(summary.contains(needle), "{needle}: {summary}");
    }
}

/// What `build-watches` asks of a `build` skill.
fn watch_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "clm next --wait",
            "run_in_background",
            "step 1",
            "TaskStop",
            "next-wait",
        ],
        &[],
    )
}

#[test]
fn build_watches() {
    // req:plugin.build/build-watches@67db2340
    let body = skill_body("build");
    assert_eq!(watch_gaps(&body), Vec::<String>::new());
    // build without the wait
    let gaps = watch_gaps(&body.replace("clm next --wait", "elided"));
    assert_reports(&gaps, "build", "clm next --wait");
    let offer = sentence_with(&body, "offer the human once per session");
    assert!(offer.contains("watch for new work"), "{offer}");
    let stays = sentence_with(&body, "stays on for the rest of the session");
    for needle in ["without asking", "keep watching"] {
        assert!(stays.contains(needle), "{needle}: {stays}");
    }
    let failed = sentence_with(&body, "exits non-zero");
    for needle in ["tell the human what it printed", "stop watching"] {
        assert!(failed.contains(needle), "{needle}: {failed}");
    }
    let fresh = sentence_with(&body, "start again at step 1");
    for needle in ["fresh `clm next`", "not repeating the notice"] {
        assert!(fresh.contains(needle), "{needle}: {fresh}");
    }
    let setting = sentence_with(&body, "no setting");
    assert!(
        setting.contains("does not outlive the session"),
        "{setting}"
    );
    let stop = sentence_with(&body, "says to stop");
    assert!(stop.contains("TaskStop"), "{stop}");
}

/// What `build-says-what-is-ready` asks of a `build` skill.
fn ready_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "needs-alone-order",
            "component-reconciliation",
            "#### Needs",
            "depends_on",
        ],
        &[
            "dependencies are all settled",
            "settles the components that depend on it",
        ],
    )
}

#[test]
fn build_says_what_is_ready() {
    // req:plugin.build/build-says-what-is-ready@d0ad7ac6
    let body = skill_body("build");
    assert_eq!(ready_gaps(&body), Vec::<String>::new());
    // a build skill that orders by dependencies
    let gaps = ready_gaps(&format!("{body}\nwhose dependencies are all settled"));
    assert_reports(&gaps, "build", "dependencies are all settled");
    let gaps = ready_gaps(&format!(
        "{body}\nit settles the components that depend on it"
    ));
    assert_reports(&gaps, "build", "settles the components that depend on it");
    let ready = sentence_with(&body, "no blocking backlog item");
    for needle in [
        "needs-alone-order",
        "component-reconciliation",
        "#### Needs",
    ] {
        assert!(ready.contains(needle), "{needle}: {ready}");
    }
    let order = sentence_with(&body, "does not order builds");
    assert!(order.contains("depends_on"), "{order}");
}

/// What `build-unclosed-builds` asks of a `build` skill.
fn unclosed_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &["landed impl/<component>", "unclosed-build-reads-finish"],
        &[],
    )
}

#[test]
fn build_unclosed_builds() {
    // req:plugin.build/build-unclosed-builds@7df50a2e
    let body = skill_body("build");
    assert_eq!(unclosed_gaps(&body), Vec::<String>::new());
    // a build skill that counts every unclosed build as a stop
    let bare = body.replace("landed impl/<component>", "landed");
    assert_reports(&unclosed_gaps(&bare), "build", "landed impl/<component>");
    let finish = sentence_with(&body, "landed impl/<component>");
    assert!(finish.contains("clm run --finish <component>"), "{finish}");
    assert!(
        finish.contains("the resume rule above holds back"),
        "{finish}"
    );
    for needle in [
        "tell the human that it landed without its closing line",
        "don't count it as a stop",
        "never by reading `origin` with a `git` command of your own",
    ] {
        assert!(finish.contains(needle), "{needle}: {finish}");
    }
    let steps = numbered_steps(&body);
    let step = steps
        .iter()
        .find(|s| s.contains("landed impl/<component>"))
        .expect("a step holds the rule");
    assert!(
        step.contains("hasn't returned"),
        "the step that reads a build's return"
    );
}

/// What `build-resumes-sound-builds` asks of a `build` skill.
fn resume_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "usage or rate limit",
            "full disk",
            "SendMessage",
            "at most once",
            "one at a time",
            "resume-sound-builds",
        ],
        &[],
    )
}

#[test]
fn build_resumes_sound_builds() {
    // req:plugin.build/build-resumes-sound-builds@0fb00352
    let body = skill_body("build");
    assert_eq!(resume_gaps(&body), Vec::<String>::new());
    // build without the resume
    let gaps = resume_gaps(&body.replace("SendMessage", "elided"));
    assert_reports(&gaps, "build", "SendMessage");
    // build that resumes only limited builds
    let gaps = resume_gaps(&body.replace("full disk", "elided"));
    assert_reports(&gaps, "build", "full disk");
    // build that rebuilds a recurring outside stop
    let gaps = resume_gaps(&body.replace("one at a time", "elided"));
    assert_reports(&gaps, "build", "one at a time");
    // The judgement comes first and has criteria.
    let judge = sentence_with(&body, "judge by why it stopped");
    for needle in [
        "`stopped: <reason>`",
        "resume that same subagent",
        "build the component again",
    ] {
        assert!(judge.contains(needle), "{needle}: {judge}");
    }
    let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(at("judge by why it stopped") < at("A subagent that ends without"));
    assert!(at("judge by why it stopped") < at("is built again the same way"));
    let sound = sentence_with(&body, "Resume a build whose work is sound");
    for needle in ["outside the work", "network outage", "host failure"] {
        assert!(sound.contains(needle), "{needle}: {sound}");
    }
    let limited = sentence_with(&body, "Resume a build stopped by a usage or rate limit");
    for needle in [
        "full disk",
        "each time",
        "never build it again",
        "bounds each resume",
    ] {
        assert!(limited.contains(needle), "{needle}: {limited}");
    }
    let once = sentence_with(&body, "Resume at most once for the same reason");
    for needle in [
        "check or test that keeps failing",
        "network outage",
        "tool or host failure",
        "already passed",
        "second such stop",
    ] {
        assert!(once.contains(needle), "{needle}: {once}");
    }
    for needle in ["usage", "full disk"] {
        assert!(!once.contains(needle), "{needle}: {once}");
    }
    let again = sentence_with(&body, "Build again, with the retry below");
    for needle in ["approach", "ran out of context"] {
        assert!(again.contains(needle), "{needle}: {again}");
    }
    let recurs = sentence_with(&body, "When the same outside cause stops builds again");
    for needle in [
        "it recurred",
        "land checks",
        "one at a time",
        "launch no new build until they have returned",
    ] {
        assert!(recurs.contains(needle), "{needle}: {recurs}");
    }
    let blocked = sentence_with(&body, "Neither resume nor build again");
    assert!(
        blocked.contains("backlog item or a spec change"),
        "{blocked}"
    );
    // A resumed build is not finished, and counts as running.
    let counted = sentence_with(&body, "For a build you resume");
    for needle in [
        "don't run `clm run --finish <component>`",
        "still running",
        "the cap",
    ] {
        assert!(counted.contains(needle), "{needle}: {counted}");
    }
    let timing = sentence_with(&body, "Resume it once the cause has cleared");
    assert!(timing.contains("limit has reset"), "{timing}");
    let tell = sentence_with(&body, "Tell the human why it stopped");
    assert!(tell.contains("what to clear"), "{tell}");
    let resume = sentence_with(&body, "Resume it with `SendMessage`");
    for needle in ["deferred", "carry on", "that one will never report back"] {
        assert!(resume.contains(needle), "{needle}: {resume}");
    }
    let failed = sentence_with(&body, "no longer exists");
    for needle in [
        "count it as a stop",
        "clm run --finish <component>",
        "never retry the resume",
    ] {
        assert!(failed.contains(needle), "{needle}: {failed}");
    }
}

/// What `build-waiting-builds` asks of a `build` skill.
fn waiting_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "waiting: <command>",
            "whatever its last line says",
            "background-work-stopped",
        ],
        &[],
    )
}

#[test]
fn build_waiting_builds() {
    // req:plugin.build/build-waiting-builds@b49b8327
    let body = skill_body("build");
    assert_eq!(waiting_gaps(&body), Vec::<String>::new());
    // build without the wait for a waiting build
    let gaps = waiting_gaps(&body.replace("waiting: <command>", "elided"));
    assert_reports(&gaps, "build", "waiting: <command>");
    let rule = sentence_with(&body, "whatever its last line says");
    for needle in [
        "`stopped:` line included",
        "unless that line is `landed <sha>`",
        "hasn't returned",
    ] {
        assert!(rule.contains(needle), "{needle}: {rule}");
    }
    // It comes before the rule for a subagent that ends without a closing line.
    let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(at("whatever its last line says") < at("A subagent that ends without"));
    let handling = sentence_with(&body, "don't tell the human it stopped");
    for needle in [
        "don't run `clm run --finish <component>`",
        "don't build the component again",
        "still running",
    ] {
        assert!(handling.contains(needle), "{needle}: {handling}");
    }
    for needle in [
        "marks as still running background work of its own",
        "for the cap and for when `clm next` is due again",
        "free its place and remove its worktree",
    ] {
        assert!(body.contains(needle), "{needle}");
    }
    let next = sentence_with(&body, "once that command has ended");
    assert!(next.contains("as the build's return"), "{next}");
}

/// What `build-stops-landed-builds` asks of a `build` skill.
fn leftover_gaps(body: &str) -> Vec<String> {
    text_gaps("build", body, &["leftover task"], &[])
}

#[test]
fn build_stops_landed_builds() {
    // req:plugin.build/build-stops-landed-builds@ee9902e8
    let body = skill_body("build");
    assert_eq!(leftover_gaps(&body), Vec::<String>::new());
    // build without stopping a landed build's leftover task
    let gaps = leftover_gaps(&body.replace("leftover task", "elided"));
    assert_reports(&gaps, "build", "leftover task");
    let rule = sentence_with(&body, "leftover task");
    assert!(rule.contains("landed <sha>"), "{rule}");
    let after = &body[body.find(rule).unwrap() + rule.len()..];
    let stop = sentence_with(after, "TaskStop");
    for needle in [
        "clm run --finish <component>",
        "ToolSearch",
        "by the task ID its notifications carry",
    ] {
        assert!(stop.contains(needle), "{needle}: {stop}");
    }
    let read = sentence_with(after, "as for any landed build");
    assert!(read.contains("landed <sha>"), "{read}");
    let ended = sentence_with(after, "already ended");
    assert!(ended.contains("go on"), "{ended}");
    // It sits right after the waiting rule and before the resume rule.
    let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(at("whatever its last line says") < at("leftover task"));
    assert!(at("leftover task") < at("judge by why it stopped"));
}

#[test]
fn build_reports_workarounds() {
    // req:plugin.build/build-reports-workarounds@75374dd5
    let body = skill_body("build");
    let summary = sentence_with(&body, "each `workaround` item");
    for needle in ["`reconsider-<component>-<slug>`", "summary", "/clm:health"] {
        assert!(summary.contains(needle), "{needle}: {summary}");
    }
    // build without the pointer
    for needle in ["reconsider-<component>-<slug>", "/clm:health"] {
        let gaps = text_gaps(
            "build",
            &body.replace(needle, "elided"),
            &["reconsider-<component>-<slug>", "/clm:health"],
            &[],
        );
        assert_reports(&gaps, "build", needle);
    }
}

/// What `skill-named-build` asks of a file: no mention of the retired skill, with its line.
fn old_skill_name_gaps(files: &[(String, String)]) -> Vec<String> {
    let old = ["/clm:implement", "skills/implement/"];
    let mut gaps = Vec::new();
    for (name, text) in files {
        for (n, line) in text.lines().enumerate() {
            for word in old.iter().filter(|word| line.contains(**word)) {
                gaps.push(format!("{name}:{}: names the old skill `{word}`", n + 1));
            }
        }
    }
    gaps
}

#[test]
fn build_skill_is_named_build() {
    // req:plugin.build/skill-named-build@3434a2db
    // The skill is `skills/build/SKILL.md`, with `skills/implement/` gone, and its description
    // holds `approved specs` and `not compile`.
    assert!(plugin().join("skills/build/SKILL.md").is_file());
    assert!(!plugin().join("skills/implement").exists());
    let (front, body) = frontmatter(&plugin().join("skills/build/SKILL.md"));
    let description = front["description"].as_str().unwrap();
    assert!(description.starts_with("Build"), "{description}");
    for needle in ["approved specs", "not compile"] {
        assert!(description.contains(needle), "{needle}: {description}");
    }
    assert!(body.trim_start().starts_with("# Build"), "{body}");
    // A guide page or skill that still says `/clm:implement` is reported with its line.
    let stale = [(
        "docs/guide/using-clm.md".to_owned(),
        format!("one\nRun {}{} to build.\n", "/clm:", "implement"),
    )];
    assert_eq!(
        old_skill_name_gaps(&stale),
        ["docs/guide/using-clm.md:2: names the old skill `/clm:implement`"]
    );
    let stale = [("x.md".to_owned(), "see skills/implement/ here\n".to_owned())];
    assert_eq!(
        old_skill_name_gaps(&stale),
        ["x.md:1: names the old skill `skills/implement/`"]
    );
    // The shipped plugin, guide, CLAUDE.md and README say none.
    let mut files = Vec::new();
    text_files_under(&plugin(), &mut files);
    text_files_under(&repo().join("docs/guide"), &mut files);
    // `AGENTS.md` too, when the repository has one.
    for name in ["CLAUDE.md", "AGENTS.md", "README.md"] {
        if name == "AGENTS.md" && !repo().join(name).exists() {
            continue;
        }
        files.push((
            name.to_owned(),
            fs::read_to_string(repo().join(name)).unwrap(),
        ));
    }
    let gaps = old_skill_name_gaps(&files);
    assert!(gaps.is_empty(), "{gaps:#?}");
}

/// What the skill lacks of the bug-drafting rule (`build-drafts-bugs`).
fn drafts_gaps(body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        "build",
        body,
        &[
            "clm triage",
            "clm draft <slug>...",
            "clm draft --finish <id>",
            "proposed <id> review",
            "build-drafts-bugs",
            "clm draft --fold <id>...",
            "proposed <batch> review <url>",
            "approve-in-session",
            "only `group` lines",
        ],
        &[],
    );
    if body.lines().any(|p| p.contains("fix reported defects")) {
        gaps.push("build: a paragraph holds `fix reported defects`".to_owned());
    }
    gaps
}

#[test]
fn build_drafts_bugs() {
    // req:plugin.build/build-drafts-bugs@c5168e54
    let body = skill_body("build");
    assert_eq!(drafts_gaps(&body), Vec::<String>::new());
    for needle in [
        "clm triage",
        "clm draft <slug>...",
        "clm draft --finish <id>",
        "proposed <id> review",
        "build-drafts-bugs",
        "clm draft --fold <id>...",
        "proposed <batch> review <url>",
        "approve-in-session",
        "only `group` lines",
    ] {
        let gaps = drafts_gaps(&body.replace(needle, "elided"));
        assert!(
            gaps.iter().any(|g| g.contains(needle)),
            "{needle}: {gaps:?}"
        );
    }
    // A skill that keeps its defect-fix step is reported.
    let gaps = drafts_gaps(&format!("{body}\nfix reported defects\n"));
    assert_reports(&gaps, "build", "fix reported defects");
    // The draft launch: unchanged stdout, background, worktree, the session's own model,
    // before the builds, and exit 3 stops the round's drafts.
    let step = sentence_with(&body, "run `clm draft <slug>...`");
    for needle in [
        "only when it exits 0",
        "unchanged",
        "background",
        "isolation: \"worktree\"",
        "session's own model",
    ] {
        assert!(step.contains(needle), "{needle}: {step}");
    }
    let para = body
        .lines()
        .find(|l| l.contains("run `clm draft <slug>...`"))
        .unwrap();
    for needle in [
        "before launching builds",
        "no draft of this session has taken",
        "exits 3",
        "never pass `--answers`",
        "`human`, `idea`, `deferred` or `workaround`",
        "background-work-stopped",
        "`landed <sha>`",
        "`stopped: <reason>`",
        "`left <slug>: <reason>`",
    ] {
        assert!(para.contains(needle), "{needle}");
    }
    // The fold: only its own drafts, in `group` order, `model: "sonnet"`, one approval.
    let fold = body
        .lines()
        .find(|l| l.contains("clm draft --fold <id>..."))
        .unwrap();
    for needle in [
        "model: \"sonnet\"",
        "only those",
        "`## Intent`",
        "`## Decisions`",
        "`## Open questions`",
        "`clm approve spec/<batch>`",
        "`clm land spec/<batch>`",
    ] {
        assert!(fold.contains(needle), "{needle}");
    }
    // The drain ends only when nothing is left to draft either, and a draft's items aren't
    // taken twice; after the archive lands, triage runs again.
    for needle in [
        "prints no `group` line whose items no draft of this session has taken",
        "run `clm triage` again",
    ] {
        assert!(body.contains(needle), "{needle}");
    }
}

#[test]
fn build_follows_agent_harness() {
    // req:plugin.build/build-follows-agent-harness@3e0df57b
    let body = skill_body("build");
    let must = [
        "clm agent-harness",
        "isolated-worktrees",
        "git worktree add .claude/worktrees/agent-<component> origin/main",
        "subagents",
        "model-choice",
        "--review-model",
    ];
    assert_eq!(text_gaps("build", &body, &must, &[]), Vec::<String>::new());
    // Each rule is tied to the capability that is `no`, in the paragraph that runs the check.
    let para = body
        .lines()
        .find(|l| l.contains("clm agent-harness"))
        .unwrap();
    for needle in [
        "Where `isolated-worktrees` is `no`, add each build's worktree",
        "tell the build, before the launcher `clm run` printed, to work in that directory",
        "`clm run --finish` and `clm wrap-up` tidy it",
        "Where `subagents` is `no`, build the components one at a time in this session itself",
        "Where `model-choice` is `no`, launch with this session's own model and pass its name as `--review-model`",
    ] {
        assert!(para.contains(needle), "{needle}");
    }
    // The check comes before the first launch.
    let step = body
        .lines()
        .find(|l| l.contains("clm agent-harness"))
        .unwrap();
    assert!(step.contains("before your first launch"));
}
