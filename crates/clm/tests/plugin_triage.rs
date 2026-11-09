//! The `triage` skill, `plugin/skills/triage/SKILL.md`: what its text tells Claude to do.
// clm:reads docs/guide/**
// clm:reads CLAUDE.md
// clm:reads README.md

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

/// The landing and approval rules every skill states, word for word, so a mention of the
/// commands elsewhere can't stand in for them.
const LAND_RULE: &str = "Never push `main` by any means but `clm land`";
const APPROVE_RULE: &str = "Run `clm approve` only after the human says yes in this conversation, \
                            and only through the guard's confirmation";
/// What no skill names: the host is reached through `clm pr`, and approvals are
/// `clm approve`, so there are no labels and no `gh` calls; and a change needs review or
/// doesn't (`yes-no-review`), so there are no tiers, no declared `gate`, no significance
/// rubric and no review after a land.
const RETIRED: [&str; 13] = [
    "gh pr",
    "gh api",
    "spec-approved",
    "spec-review",
    "clm watch",
    "clm review",
    "clm significant",
    "## Significant",
    "`gate`",
    "gate: ",
    "HARD",
    "SOFT",
    "AUTO",
];

fn names_the_old_hash_command(text: &str) -> bool {
    text.match_indices("clm hash").any(|(at, needle)| {
        !text[at + needle.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

/// The first sentence of `text` holding `needle`: sentences end at a newline or at a
/// period followed by a space, so file names such as `change.md` don't cut one short.
fn sentence_with<'a>(text: &'a str, needle: &str) -> &'a str {
    text.lines()
        .flat_map(|l| l.split(". "))
        .find(|s| s.contains(needle))
        .unwrap_or_else(|| panic!("no sentence mentions {needle}"))
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

/// What `triage-caps-drafts` asks of a `triage` skill.
fn draft_cap_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
        body,
        &[
            "exits 3",
            "draft <id>",
            "clm draft --finish <id>",
            "max_builds",
            "clm.local.toml",
        ],
        &[],
    )
}

#[test]
fn triage_caps_drafts() {
    // req:plugin.triage/triage-caps-drafts@d3a894ba
    let body = skill_body("triage");
    assert_eq!(draft_cap_gaps(&body), Vec::<String>::new());
    // triage without finishing
    let gaps = draft_cap_gaps(&body.replace("clm draft --finish", "elided"));
    assert_reports(&gaps, "triage", "clm draft --finish");
    // Drafts launch in the `group` lines' order while a place is free; exit 3 holds the
    // group and the ones after it until a draft returns.
    let free = sentence_with(&body, "while a place is free");
    assert!(free.contains("order of the `group` lines"), "{free}");
    let full = sentence_with(&body, "exits 3");
    for needle in ["no free place", "the ones after it", "returns"] {
        assert!(full.contains(needle), "{needle}: {full}");
    }
    let id = sentence_with(&body, "`draft <id>`");
    assert!(id.contains("stderr"), "{id}");
    let finish = sentence_with(&body, "whatever its closing line");
    for needle in ["clm draft --finish <id>", "next waiting group"] {
        assert!(finish.contains(needle), "{needle}: {finish}");
    }
    let cap = sentence_with(&body, "max_builds");
    for needle in ["none of your own drafts is running", "cap is reached"] {
        assert!(cap.contains(needle), "{needle}: {cap}");
    }
    let summary = sentence_with(&body, "end with a summary");
    assert!(summary.contains("groups not drafted"), "{summary}");
}

/// What `triage-skill` asks of a `triage` skill, one line per gap naming the skill and what
/// it lacks or must not name. Empty when it says it all.
fn triage_skill_gaps(body: &str) -> Vec<String> {
    let mut gaps: Vec<String> = [
        "clm triage",
        "clm draft",
        "clm draft --fold",
        "Agent",
        "background",
        "isolation: \"worktree\"",
        "manual",
        "landed <sha>",
        "proposed <id> review",
        "proposed <batch> review <url>",
        "stopped:",
        "left <slug>",
        "components corrected to",
        "left <id>",
        "`open`",
        "clm approve",
        "clm approve spec/<batch>",
        "clm land spec/<batch>",
        "clm draft --finish <batch>",
        "claim released by",
        "clm land",
        "clm wrap-up",
        LAND_RULE,
        APPROVE_RULE,
    ]
    .into_iter()
    .filter(|needle| !body.contains(needle))
    .map(|needle| format!("triage: SKILL.md must mention {needle}"))
    .collect();
    gaps.extend(
        RETIRED
            .into_iter()
            .filter(|needle| body.contains(needle))
            .chain(names_the_old_hash_command(body).then_some("clm hash"))
            .map(|needle| format!("triage: SKILL.md must not mention {needle}")),
    );
    gaps
}

#[test]
fn triage_launches_drafts() {
    // req:plugin.triage/triage-skill@d7e0d41f
    let (fm, body) = frontmatter(&plugin().join("skills/triage/SKILL.md"));
    assert!(fm["description"].as_str().is_some_and(|d| !d.is_empty()));
    assert_eq!(triage_skill_gaps(&body), Vec::<String>::new());
    // The fold's own draft is finished from the session's checkout, when it isn't a folded one.
    let finish = sentence_with(&body, "clm draft --finish <batch>");
    for needle in [
        "isn't one of their IDs",
        "your own checkout, not the fold's worktree",
    ] {
        assert!(finish.contains(needle), "{needle}: {finish}");
    }
    // triage without the approval rule
    let gaps = triage_skill_gaps(&body.replace("clm approve", "elided"));
    assert!(
        !gaps.is_empty()
            && gaps
                .iter()
                .all(|g| g.starts_with("triage:") && g.contains("clm approve")),
        "{gaps:?}"
    );
    // triage naming a label, or the old commands
    for (old, named) in [
        ("Add `spec-approved`.", "spec-approved"),
        ("Run `clm watch <pr>`.", "clm watch"),
        ("Run `gh pr create`.", "gh pr"),
        ("Rebase on `clm hash`.", "clm hash"),
        ("Run `gh api repos/{owner}/{repo}/pulls`.", "gh api"),
        ("Wait for `spec-review`.", "spec-review"),
    ] {
        let gaps = triage_skill_gaps(&format!("{body}\n{old}\n"));
        assert_eq!(gaps, [format!("triage: SKILL.md must not mention {named}")]);
    }
    // The mode is mentioned once, before the first draft, and never refused.
    let mode = sentence_with(&body, "manual");
    for needle in ["once", "permission mode"] {
        assert!(mode.contains(needle), "{needle}: {mode}");
    }
    assert!(body.find("manual").unwrap() < body.find("clm draft").unwrap());
    assert_eq!(body.matches("permission mode").count(), 1, "{body}");
    assert!(body.contains("Never refuse a mode"), "{body}");
    // Each group's prompt goes, only on a clean exit, to its own background subagent.
    let launch = sentence_with(&body, "isolation: \"worktree\"");
    for needle in [
        "clm draft <slug>...",
        "`group`",
        "exits 0",
        "stdout",
        "Agent",
        "background",
        "one per group",
        "parallel",
    ] {
        assert!(launch.contains(needle), "{needle}: {launch}");
    }
    let own = sentence_with(&body, "own context");
    assert!(own.contains("Never design"), "{own}");
    assert!(own.contains("triage-revises-declined-fold"), "{own}");
    // The closing lines, and a stopped draft is never retried.
    let stopped = sentence_with(&body, "`stopped: <reason>`");
    for needle in ["tell the user", "never retry"] {
        assert!(stopped.contains(needle), "{needle}: {stopped}");
    }
    let left = sentence_with(&body, "`left <slug>: <reason>`");
    assert!(left.contains("summary"), "{left}");
    assert!(left.contains("sorted by its reason"), "{left}");
    let sorted = sentence_with(&body, "the items drafts left");
    assert!(sorted.contains("sorted by the reason"), "{sorted}");
    // The items drafts left are sorted by their reason: a nodes correction is a bug moved, which
    // the next `clm triage` drafts once the change lands, and never something for the human.
    let moved = sentence_with(&body, "components corrected to <components>`");
    for needle in ["moved", "`clm triage`", "landed"] {
        assert!(moved.contains(needle), "{needle}: {moved}");
    }
    assert!(!moved.contains("human"), "{moved}");
    let other = sentence_with(&body, "any other reason");
    for needle in [
        "left with that reason",
        "its claim released by `clm draft --finish`",
        "drafted again once what it waited on is settled",
    ] {
        assert!(other.contains(needle), "{needle}: {other}");
    }
    assert!(!other.contains("human"), "{other}");
    let summary = sentence_with(&body, "Never say a moved item");
    assert!(summary.contains("needs the human"), "{summary}");
    // Other work, `open` or `main`, is never put to the human for approval.
    let others = sentence_with(&body, "never put to the human for approval");
    for needle in ["`open`", "`main`"] {
        assert!(others.contains(needle), "{needle}: {others}");
    }
    // A draft leaves a change needing review with no page, so its closing line carries no URL.
    assert!(!body.contains("proposed <id> review <url>"), "{body}");
    // Once every draft has returned, the closings needing review are folded, in the `group` lines'
    // order, by one background subagent launched only on a clean exit.
    let fold = sentence_with(&body, "clm draft --fold <id>...");
    for needle in [
        "every draft has returned",
        "`proposed <id> review`",
        "own drafts",
        "order of the `group` lines",
        "exits 0",
        "stdout",
        "one background",
        "isolation: \"worktree\"",
    ] {
        assert!(fold.contains(needle), "{needle}: {fold}");
    }
    let fold_closing = sentence_with(&body, "`proposed <batch> review <url>`");
    assert!(
        fold_closing.contains("`stopped: <reason>`"),
        "{fold_closing}"
    );
    // A change the fold leaves keeps its branch for `/clm:propose`, whether the fold's
    // stderr or its subagent names it.
    let left_id = sentence_with(&body, "`left <id>: <reason>`");
    for needle in [
        "stderr",
        "subagent",
        "summary",
        "branch stays",
        "/clm:propose",
    ] {
        assert!(left_id.contains(needle), "{needle}: {left_id}");
    }
    // One approval for the intent of every folded change, never an `open` line's, with
    // the fold page and its body's sections.
    let ask = sentence_with(&body, "every folded change");
    for needle in ["once", "intent", "fold pull request's link"] {
        assert!(ask.contains(needle), "{needle}: {ask}");
    }
    let never_open = sentence_with(&body, "never put to the human for approval");
    assert!(never_open.contains("`open`"), "{never_open}");
    let show = sentence_with(&body, "pull request's link");
    for needle in ["## Intent", "## Decisions", "## Open questions"] {
        assert!(show.contains(needle), "{needle}: {show}");
    }
    let yes = sentence_with(&body, "clm approve spec/<batch>");
    for needle in ["yes", "guard", "clm land spec/<batch>"] {
        assert!(yes.contains(needle), "{needle}: {yes}");
    }
    // Once the fold has landed, each folded draft is finished.
    let finish = sentence_with(&body, "each folded draft");
    assert!(finish.starts_with("Once it has landed, run"), "{finish}");
    assert!(finish.contains("clm draft --finish <id>"), "{finish}");
    // A stopped fold, like a stopped draft, is never retried.
    let stopped_fold = sentence_with(&body, "`stopped` fold");
    assert!(stopped_fold.contains("not retried"), "{stopped_fold}");
    // The fold follows the drafts, the approval the fold, and the checkpoint ends it.
    let at = |needle: &str| body.find(needle).unwrap();
    assert!(at("`stopped: <reason>`") < at("clm draft --fold"));
    assert!(at("clm draft --fold") < at("every folded change"));
    assert!(at("every folded change") < at("clm wrap-up"));
    let summary = sentence_with(&body, "end with a summary");
    for needle in [
        "fold's branch",
        "changes the fold left",
        "branches needing review not folded",
        "`open`",
        "`human`",
        "items drafts left",
    ] {
        assert!(summary.contains(needle), "{needle}: {summary}");
    }
}

/// What `triage-answers` asks of a `triage` skill.
fn answers_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
        body,
        &[
            ".clm/answers.md",
            "--answers .clm/answers.md",
            "clm draft --answers .clm/answers.md <slug>",
            "## The human's answers",
        ],
        &["which no draft takes"],
    )
}

#[test]
fn triage_carries_answers() {
    // req:plugin.triage/triage-answers@4cbcff5f
    let body = skill_body("triage");
    assert_eq!(answers_gaps(&body), Vec::<String>::new());
    // triage without answers
    let gaps = answers_gaps(&body.replace("--answers", "elided"));
    assert_reports(&gaps, "triage", "--answers");
    // Saying no draft takes a `human` line's item is reported.
    let gaps = answers_gaps(&format!("{body}\nItems which no draft takes.\n"));
    assert_reports(&gaps, "triage", "which no draft takes");
    // An earlier run's answers are removed before `clm triage` runs.
    let remove = sentence_with(&body, "Remove `.clm/answers.md`");
    assert!(remove.contains("earlier run"), "{remove}");
    assert!(
        body.find("Remove `.clm/answers.md`").unwrap()
            < body.find("Then run `clm triage`").unwrap()
    );
    // The human may answer, or instruct the drafts, at any point, and later drafts carry it.
    let any = sentence_with(&body, "at any point in this session");
    for needle in [
        "`human` lines",
        "instruction for the drafts",
        "launched after",
    ] {
        assert!(any.contains(needle), "{needle}: {any}");
    }
    // What they say is kept verbatim, `.clm/` created when missing.
    let kept = sentence_with(&body, "verbatim");
    for needle in ["`.clm/answers.md`", "creating `.clm/`"] {
        assert!(kept.contains(needle), "{needle}: {kept}");
    }
    // Once it exists, every draft started gets it; every `clm` runs from the root.
    let every = sentence_with(&body, "pass `--answers .clm/answers.md`");
    assert!(
        every.contains("every `clm draft`") && every.contains("Once that file exists"),
        "{every}"
    );
    let root = sentence_with(&body, "every `clm` command");
    assert!(root.contains("checkout's root"), "{root}");
    // An answered item is drafted as its own group, under the cap, after the waiting groups.
    let own = sentence_with(&body, "clm draft --answers .clm/answers.md <slug>");
    for needle in [
        "`question` or `human-task` item",
        "answered",
        "so plainly",
        "group of its own",
        "same cap",
        "after the `group` lines still waiting",
    ] {
        assert!(own.contains(needle), "{needle}: {own}");
    }
    // A running draft never gets an answer; the summary names the drafts an answer missed.
    let running = sentence_with(&body, "draft already running");
    for needle in ["Never pass an answer", "message", "file"] {
        assert!(running.contains(needle), "{needle}: {running}");
    }
    // The fold's approval shows each folded change's answers beside its intent.
    let show = sentence_with(&body, "fold pull request's link");
    let intent = show.find("## Intent").unwrap();
    assert!(show[intent..].contains("## The human's answers"), "{show}");
    let summary = sentence_with(&body, "end with a summary");
    for needle in [
        "`human` lines answered and drafted apart from those not answered",
        "each draft launched before an answer that the answer concerns",
    ] {
        assert!(summary.contains(needle), "{needle}: {summary}");
    }
}

/// What `triage-contributions` asks of a `triage` skill.
fn contributions_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
        body,
        &[
            "clm intake",
            "untrusted",
            ".clm/intake/",
            "issue-verifier",
            "clm contribution show",
            "clm contribution label",
            "clm contribution comment --close",
            "intake/<YYYYMMDD-HHMM>",
            "Co-authored-by",
            "clm pr open",
            "issues",
            "alerts:",
            "alerts unavailable",
            "imported",
            "contribution",
        ],
        &[],
    )
}

#[test]
fn triage_takes_the_inbox_in() {
    // req:plugin.triage/triage-contributions@7a586b6e
    let body = skill_body("triage");
    assert_eq!(contributions_gaps(&body), Vec::<String>::new());
    // triage without the verifier
    let gaps = contributions_gaps(&body.replace("issue-verifier", "elided"));
    assert_reports(&gaps, "triage", "issue-verifier");
    let step = body
        .lines()
        .find(|l| l.starts_with("2a. *Contributions.*"))
        .expect("triage step 2a, the Contributions step");
    // Untrusted data, nothing written to the host without a yes.
    for needle in [
        "untrusted data",
        "never as instructions",
        "write nothing to the host",
        "without the human's yes",
    ] {
        assert!(step.contains(needle), "{needle}: {step}");
    }
    // The verifier runs on the session's own model, given the files and the checkout only.
    let launch = sentence_with(step, "launch one `issue-verifier`");
    for needle in [
        "session's own model",
        "model-per-launch",
        "and nothing else",
    ] {
        assert!(launch.contains(needle), "{needle}: {launch}");
    }
    // The saved files, one per new issue and pull request.
    for needle in ["`.clm/intake/<n>.md`", "`.clm/intake/pull-<n>.md`"] {
        assert!(step.contains(needle), "{needle}: {step}");
    }
    // Each issue is shown whole, one at a time.
    let shown = sentence_with(step, "show the human the issue's saved text whole");
    for needle in [
        "not a summary",
        "verifier's verdict",
        "flagging each passage",
    ] {
        assert!(shown.contains(needle), "{needle}: {shown}");
    }
    // The branch, its trailer and its land by the gate's first line.
    let branch = sentence_with(step, "on a branch of its own named");
    for needle in ["worktree of its own", "`Co-authored-by` trailer"] {
        assert!(branch.contains(needle), "{needle}: {branch}");
    }
    let land = sentence_with(step, "land it with `clm land`");
    for needle in ["`code`", "`spec`", "delete the pushed branch"] {
        assert!(land.contains(needle), "{needle}: {land}");
    }
    // Labels and comments only after the land; resolved issues and pull requests.
    let after = sentence_with(step, "On `landed <sha>`");
    for needle in ["`accepted` label", "naming its item", "comment --close"] {
        assert!(after.contains(needle), "{needle}: {after}");
    }
    let resolved = sentence_with(step, "For each `issue <n> resolved <slug> <by>` line");
    for needle in ["recommending waiting", "naming `<by>`"] {
        assert!(resolved.contains(needle), "{needle}: {resolved}");
    }
    let alerts = sentence_with(step, "For the `alert <n> new` lines");
    for needle in [
        "no verifier",
        "`critical` and `high`",
        "`alerts: [<n>]`",
        "`kind: bug`",
        "whose `paths` hold its manifest",
        "no `Co-authored-by` trailer",
    ] {
        assert!(alerts.contains(needle), "{needle}: {alerts}");
    }
    let alert_done = sentence_with(step, "For each `alert <n> resolved <slug> <by>` line");
    for needle in [
        "stays open on the host until the updated dependency is on `main`",
        "once `<by>` is archived",
    ] {
        assert!(alert_done.contains(needle), "{needle}: {alert_done}");
    }
    let down = sentence_with(step, "On an `alerts unavailable` line");
    assert!(down.contains("go on"), "{down}");
    let imported = sentence_with(step, "For each `issue <n> imported <slug>...`");
    for needle in [
        "already taken in",
        "for information only",
        "do nothing for it",
        "stays open on the host until its item is resolved",
    ] {
        assert!(imported.contains(needle), "{needle}: {imported}");
    }
    let pull = sentence_with(step, "For each `pull <n>` line");
    assert!(pull.contains("`contribution` skill's steps"), "{pull}");
    assert!(step.contains("run `clm triage` again"), "{step}");
    let summary = sentence_with(&body, "end with a summary");
    assert!(
        summary.contains("the issues and alerts taken in, with their items"),
        "{summary}"
    );
    for needle in [
        "run nothing from it",
        "with the Agent tool",
        "one at a time, show the human",
        "adding its number to the `issues` of the existing item",
        "`clm proposals`",
        "cut from `origin/main`",
        "`clm gate --base origin/main`",
        "`## Intent` and `status` of change `<by>`",
        "the verifier's overlap finding",
    ] {
        assert!(step.contains(needle), "{needle}: {step}");
    }
}

/// What `triage-menu` asks of a `triage` skill.
fn menu_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
        body,
        &[
            "AskUserQuestion",
            "Contributions",
            "clm intake",
            "Approvals",
            "Needs your input",
            "Bugs",
            "Blockers",
            "Ideas",
            "Deferred",
            "Workaround",
            "designed",
            "idea",
            "clm superpowers brainstorming",
            "deferred",
        ],
        &["design:"],
    )
}

#[test]
fn triage_shows_a_menu() {
    // req:plugin.triage/triage-menu@e683c40d
    let body = skill_body("triage");
    assert_eq!(menu_gaps(&body), Vec::<String>::new());
    // triage without the menu
    let gaps = menu_gaps(&body.replace("Ideas", "elided"));
    assert_reports(&gaps, "triage", "Ideas");
    // triage without workaround
    let gaps = menu_gaps(&body.replace("Workaround", "elided"));
    assert_reports(&gaps, "triage", "Workaround");
    let gaps = menu_gaps(&format!("{body}\nA `design:` line.\n"));
    assert_reports(&gaps, "triage", "design:");
    // The menu comes after `clm triage` and before any draft, picking one or more, with counts.
    let at = |needle: &str| body.find(needle).unwrap();
    assert!(at("Then run `clm triage`") < at("AskUserQuestion"));
    assert!(at("AskUserQuestion") < at("clm draft <slug>..."));
    let menu = sentence_with(&body, "AskUserQuestion");
    for needle in [
        "one or more",
        "at least one line",
        "count",
        "Before launching",
    ] {
        assert!(menu.contains(needle), "{needle}: {menu}");
    }
    let contributions = body
        .lines()
        .find(|l| l.contains("- *Contributions*"))
        .unwrap();
    for needle in [
        "`issue <n> new`",
        "counted together",
        "step 2a says",
        "imported` lines are not counted",
        "before every other choice but the *Approvals*",
    ] {
        assert!(contributions.contains(needle), "{needle}: {contributions}");
    }
    let named = sentence_with(&body, "/clm:triage bugs");
    assert!(named.contains("without the menu"), "{named}");
    let picked = sentence_with(&body, "Work only the choices picked");
    for needle in ["in this order", "each line once", "*Approvals* first"] {
        assert!(picked.contains(needle), "{needle}: {picked}");
    }
    // The choices, in their order, each naming its lines.
    let intake = sentence_with(&body, "Then run `clm triage`");
    assert!(intake.contains("`clm intake`"), "{intake}");
    let choices = [
        (
            "*Contributions*",
            "`issue <n> resolved`, `pull`, `alert <n> new` and `alert <n> resolved` lines",
        ),
        ("*Bugs*", "`group` lines, drafted as step 5 says"),
        ("*Approvals*", "`designed` lines"),
        ("*Needs your input*", "`human` lines"),
        ("*Blockers*", "highest `<n>` first"),
        ("*Ideas*", "`idea` lines"),
        ("*Deferred*", "`deferred` lines"),
        ("*Workaround*", "`workaround` lines"),
    ];
    let mut last = 0;
    for (choice, lines) in choices {
        let line = body
            .lines()
            .find(|l| l.contains(&format!("- {choice}")))
            .unwrap();
        assert!(line.contains(lines), "{choice}: {line}");
        let here = at(&format!("- {choice}"));
        assert!(last < here, "{choice} out of order");
        last = here;
    }
    let workaround = body
        .lines()
        .find(|l| l.starts_with("6a. *Deferred* and *Workaround.*"))
        .expect("triage step 6a, the Deferred and Workaround step");
    assert!(
        workaround.contains("`clm context <component>`"),
        "{workaround}"
    );
    assert!(!workaround.contains("`clm context <node>`"), "{workaround}");
    let blockers = sentence_with(&body, "- *Blockers*");
    for needle in [
        "`group`, `human`, `idea`, `deferred` and `workaround`",
        "above 0",
        "as its own choice works it",
    ] {
        assert!(blockers.contains(needle), "{needle}: {blockers}");
    }
    assert!(sentence_with(&body, "no choice").contains("`open` and `main`"));
    // Approvals: read from the spec branch, approved through the guard, landed, finished,
    // and asked before any draft launches.
    let read = sentence_with(&body, "git show origin/spec/<id>:changes/<id>/change.md");
    for needle in [
        "## Intent",
        "human_burdens",
        "## Open questions",
        "approves its intent",
        "link to its pull request when it has one",
    ] {
        assert!(read.contains(needle), "{needle}: {read}");
    }
    let yes = sentence_with(&body, "clm approve spec/<id>");
    for needle in [
        "guard",
        "clm land spec/<id>",
        "on `landed <sha>`",
        "clm draft --finish <id>",
    ] {
        assert!(yes.contains(needle), "{needle}: {yes}");
    }
    let other = sentence_with(&body, "prints anything else");
    for needle in ["leave the branch", "summary"] {
        assert!(other.contains(needle), "{needle}: {other}");
    }
    assert!(sentence_with(&body, "These asks come before any draft").contains("fold's approval"));
    // Needs your input: one at a time, with a recommendation.
    let input = sentence_with(&body, "*Needs your input.*");
    assert!(input.contains("One at a time"), "{input}");
    assert!(sentence_with(&body, "git show origin/main:backlog/<slug>.md").contains("recommend"));
    // Ideas: claimed, brainstormed here while drafts run, never drafted, and ended three ways.
    let ideas = sentence_with(&body, "*Ideas.*");
    for needle in [
        "While the drafts run",
        "one at a time",
        "never pass it to `clm draft`",
    ] {
        assert!(ideas.contains(needle), "{needle}: {ideas}");
    }
    assert!(sentence_with(&body, "Claim its item").contains("clm claim backlog/<slug>"));
    let ways = sentence_with(&body, "one of four ways");
    for needle in [
        "elaborated",
        "propose-elaborates-ideas",
        "proposed",
        "`kind` to `deferred`",
        "condition",
        "dropped",
        "`resolves`",
        "`## Intent`",
        "`propose`'s steps",
    ] {
        assert!(ways.contains(needle), "{needle}: {ways}");
    }
    assert!(sentence_with(&body, "no delta").contains("archive-in-spec-pr"));
    // Deferred and workaround: one at a time, drafted or dropped through the answers file.
    let kept = sentence_with(&body, "*Deferred* and *Workaround.*");
    for needle in ["One at a time", "keep it waiting", "drop it"] {
        assert!(kept.contains(needle), "{needle}: {kept}");
    }
    let workaround = sentence_with(&body, "`workaround` line picked");
    for needle in ["clm context <component>", "keep the flag as evidence"] {
        assert!(workaround.contains(needle), "{needle}: {workaround}");
    }
    let drafted = sentence_with(&body, "has drafted or dropped");
    for needle in [
        ".clm/answers.md",
        "clm draft --answers .clm/answers.md <slug>",
        "backlog-only change",
    ] {
        assert!(drafted.contains(needle), "{needle}: {drafted}");
    }
    // The summary names the choices not picked, with their counts, and how ideas ended.
    let summary = sentence_with(&body, "end with a summary");
    for needle in [
        "choices not picked with their counts",
        "items kept",
        "ideas brainstormed and how each ended",
    ] {
        assert!(summary.contains(needle), "{needle}: {summary}");
    }
}

/// What `triage-waiting-drafts` asks of a `triage` skill.
fn waiting_draft_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
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
fn triage_waiting_drafts() {
    // req:plugin.triage/triage-waiting-drafts@31624f18
    let body = skill_body("triage");
    assert_eq!(waiting_draft_gaps(&body), Vec::<String>::new());
    // triage without the wait for a waiting draft
    let gaps = waiting_draft_gaps(&body.replace("waiting: <command>", "elided"));
    assert_reports(&gaps, "triage", "waiting: <command>");
    let rule = sentence_with(&body, "whatever its last line says");
    for needle in [
        "`stopped:` line included",
        "unless that line is `landed <sha>` or `proposed <id> review`",
        "hasn't returned",
    ] {
        assert!(rule.contains(needle), "{needle}: {rule}");
    }
    // It comes in the step that reads a draft's return, before the finish rule.
    let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(at("whatever its last line says") < at("whatever its closing line"));
    let steps = numbered_steps(&body);
    assert!(
        steps
            .iter()
            .any(|s| s.contains("whatever its last line says")
                && s.contains("whatever its closing line")),
        "the rule and the finish rule are in different steps"
    );
    let handling = sentence_with(&body, "don't tell the human it stopped");
    for needle in [
        "don't run `clm draft --finish <id>`",
        "don't launch the next waiting group",
        "don't draft its items again",
        "still running",
    ] {
        assert!(handling.contains(needle), "{needle}: {handling}");
    }
    for needle in [
        "marks as still running background work of its own",
        "for the cap and for when every draft has returned",
        "free its place and remove its worktree",
    ] {
        assert!(body.contains(needle), "{needle}");
    }
    let next = sentence_with(&body, "once that command has ended");
    assert!(next.contains("as the draft's return"), "{next}");
}

/// What `triage-enters-branch-worktree` asks of a `triage` skill.
fn enter_worktree_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
        body,
        &[
            "EnterWorktree",
            "ExitWorktree",
            "git rev-parse --show-toplevel",
            "git worktree list",
        ],
        &[],
    )
}

#[test]
fn triage_enters_branch_worktree() {
    // req:plugin.triage/triage-enters-branch-worktree@fcd0a66e
    let body = skill_body("triage");
    assert_eq!(enter_worktree_gaps(&body), Vec::<String>::new());
    // skill without the switch
    let gaps = enter_worktree_gaps(&body.replace("EnterWorktree", "elided"));
    assert_reports(&gaps, "triage", "EnterWorktree");
    let enter = sentence_with(&body, "with the EnterWorktree tool's `path`");
    assert!(enter.contains("`spec/<batch>`"), "{enter}");
    assert!(enter.contains("`git worktree list`"), "{enter}");
    let first = sentence_with(&body, "first note your own checkout");
    for needle in [
        "`git rev-parse --show-toplevel`",
        "lone rename fold",
        "`gate/up-to-date`",
        "failing check",
    ] {
        assert!(first.contains(needle), "{needle}: {first}");
    }
    assert!(body.contains("There, rebase, resolve, commit, push with `clm branch push` and land."));
    let never = sentence_with(&body, "Never reach into that worktree");
    for needle in ["`cd`", "`git -C`", "subagent"] {
        assert!(never.contains(needle), "{needle}: {never}");
    }
    let back = sentence_with(&body, "switch back before any");
    for needle in [
        "`clm draft --finish`",
        "under `.claude/worktrees/`",
        "ExitWorktree",
        "action: \"keep\"",
        "removes neither worktree",
    ] {
        assert!(back.contains(needle), "{needle}: {back}");
    }
    let none = sentence_with(&body, "no worktree holds `spec/<batch>`");
    for needle in ["say so", "leave the branch", "summary"] {
        assert!(none.contains(needle), "{needle}: {none}");
    }
}

/// What `triage-checks-splits` asks of a `triage` skill.
fn split_check_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
        body,
        &[
            "clm survey --components",
            "outside: [\"**\"]",
            "`splits-import-one-way`",
            ".clm/cut.yaml",
        ],
        &[],
    )
}

#[test]
fn triage_checks_splits() {
    // req:plugin.triage/triage-checks-splits@e29c4008
    let body = skill_body("triage");
    assert_eq!(split_check_gaps(&body), Vec::<String>::new());
    // triage without the check
    let gaps = split_check_gaps(&body.replace("clm survey --components", "elided"));
    assert_reports(&gaps, "triage", "clm survey --components");
    let check = sentence_with(&body, "clm survey --components");
    for needle in ["exits 0", "`survey/undeclared-edge`", "`survey/cycle`"] {
        assert!(
            check.contains(needle) || body[body.find(check).unwrap()..].contains(needle),
            "{needle}: {check}"
        );
    }
}

/// What `triage-folds-renames-alone` asks of a `triage` skill.
fn lone_fold_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "triage",
        body,
        &[
            "folds alone",
            "clm draft --fold <id>",
            "changes/archive/<id>/change.md",
        ],
        &[],
    )
}

#[test]
fn triage_folds_renames_alone() {
    // req:plugin.triage/triage-folds-renames-alone@9181638e
    let body = skill_body("triage");
    assert_eq!(lone_fold_gaps(&body), Vec::<String>::new());
    // skill without the lone fold
    let gaps = lone_fold_gaps(&body.replace("folds alone", "elided"));
    assert_reports(&gaps, "triage", "folds alone");
    // Each such line is folded alone, whatever the first fold's exit, and launched as a fold.
    let lone = sentence_with(&body, "that says `folds alone`");
    for needle in [
        "exited 0 or 1",
        "with that ID alone",
        "only when it exits 0",
        "background",
        "isolation: \"worktree\"",
        "model: \"sonnet\"",
    ] {
        assert!(lone.contains(needle), "{needle}: {lone}");
    }
    let neither = sentence_with(&body, "Such a change is neither");
    for needle in [
        "branches needing review not folded",
        "changes the fold left",
        "unless its own fold exited non-zero or stopped",
    ] {
        assert!(neither.contains(needle), "{needle}: {neither}");
    }
    // One ask, once every fold has returned, reading a rename's answers from the archive,
    // approving and landing each fold.
    let ask = sentence_with(&body, "Once every fold has returned");
    for needle in ["each fold pull request's link", "every folded change"] {
        assert!(ask.contains(needle), "{needle}: {ask}");
    }
    assert!(
        body.contains("for a rename change that folded alone, read its `## The human's answers` with `git show origin/spec/<id>:changes/archive/<id>/change.md`"),
        "the rename's answers are read from the archive"
    );
    let yes = sentence_with(&body, "clm approve spec/<batch>");
    assert!(yes.contains("for each such fold"), "{yes}");
}

#[test]
fn triage_revises_a_declined_fold() {
    // req:plugin.triage/triage-revises-declined-fold@b4e8e3cf
    let body = skill_body("triage");
    let step = numbered_steps(&body)
        .into_iter()
        .find(|s| s.contains("clm approve spec/<batch>"))
        .expect("approval step");
    for needle in [
        "different design",
        "clm rebase",
        "clm branch push",
        "triage-revises-declined-fold",
        "push with `clm branch push`, and ask again",
        "the one design",
        "plain no",
        "switch into the worktree that holds `spec/<batch>`",
        "edit there each folded change",
    ] {
        assert!(step.contains(needle), "{needle}");
    }
    // a skill that only leaves the branch
    let gaps = text_gaps(
        "triage",
        &body.replace("triage-revises-declined-fold", "elided"),
        &["triage-revises-declined-fold"],
        &[],
    );
    assert_reports(&gaps, "triage", "triage-revises-declined-fold");
}
