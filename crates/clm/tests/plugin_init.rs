//! The `init` skill, `plugin/skills/init/SKILL.md`: what it tells Claude to do.

use std::fs;
use std::path::{Path, PathBuf};

fn plugin() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../plugin")
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

/// One gap per needle the text lacks.
fn lacks(skill: &str, body: &str, needles: &[&str]) -> Vec<String> {
    needles
        .iter()
        .filter(|n| !body.contains(*n))
        .map(|n| format!("{skill}: must name {n}"))
        .collect()
}

/// What `init-skill` asks the `init` skill's sentence drafting the human layers to name.
const INIT_LAYERS: [&str; 8] = [
    "`ARCHITECTURE.md`'s `## Constraints`",
    "`#### Why`",
    "clm guide spec-format",
    "root component's Design",
    "`#### Enforced by`",
    "`#### Untested`",
    "the component `capability`",
    "req:capability/",
];

/// What `init-skill` asks of an `init` skill, one line per gap naming the skill and the text
/// it lacks or must not name. Empty when it says it all.
fn init_skill_gaps(body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        "init",
        body,
        &[
            "clm survey --components",
            "clm approve",
            "bootstrap commit",
            "outside",
            "incremental-adoption",
            "scripts/clm-check.sh",
            "azure-pipelines.yml",
            "Bypass policies when pushing",
            "path filter",
        ],
        // `clm init` writes no workflow, regenerates no file and creates no labels.
        &[
            "clm.yml",
            "clm-build.sh",
            "spec-approved",
            "spec-review",
            "auto-merge",
            "status check",
            "regenerat",
            "creates the labels",
        ],
    );
    // The first spec branch drafts both human layers; the node map lives in the root's
    // Design; each architecture entry names its enforcement or why none can; the node
    // `capability` builds the capabilities' tests.
    let layers = body
        .lines()
        .flat_map(|l| l.split(". "))
        .find(|s| s.contains("`CAPABILITIES.md`'s `## Capabilities`"))
        .unwrap_or_default();
    for needle in INIT_LAYERS {
        if !layers.contains(needle) {
            gaps.push(format!(
                "init: SKILL.md's human-layers sentence must name {needle}"
            ));
        }
    }
    // The bootstrap step checks `origin/main` for what `clm init` wrote.
    let bootstrap = body
        .lines()
        .flat_map(|l| l.split(". "))
        .find(|s| s.contains("pushes this one commit"))
        .unwrap_or_default();
    for needle in ["scripts/clm-check.sh", "origin/main"] {
        if !bootstrap.contains(needle) {
            gaps.push(format!(
                "init: SKILL.md's bootstrap step must check {needle}"
            ));
        }
    }
    gaps
}

#[test]
fn init_skill() {
    // req:plugin.init/init-skill@27762fbb
    let (fm, body) = frontmatter(&plugin().join("skills/init/SKILL.md"));
    assert!(fm["description"].as_str().is_some_and(|d| !d.is_empty()));
    assert_eq!(init_skill_gaps(&body), Vec::<String>::new());
    // skill without the human layers: each part missing from the sentence is reported.
    for needle in INIT_LAYERS {
        let gaps = init_skill_gaps(&body.replace(needle, "elided"));
        let gap = format!("init: SKILL.md's human-layers sentence must name {needle}");
        assert!(gaps.contains(&gap), "{needle}: {gaps:?}");
    }
    // The node `capability` builds the capabilities' tests, not the root.
    let layers = sentence_with(&body, "`CAPABILITIES.md`'s `## Capabilities`");
    for needle in [
        "`specs/capability/SPEC.md`",
        "public boundary",
        "harness",
        "`paths`",
        "`human-layer-proof`",
        "naming the rules of the drafted components",
        "saying why no program can",
        "or both",
        "the `capability` component's build proves the capabilities",
        "`clm survey --components`",
    ] {
        assert!(layers.contains(needle), "{needle}: {layers}");
    }
    assert!(
        !body.contains("req:architecture/"),
        "init: a test never cites an architecture requirement"
    );
    // An existing CLM repository gets `propose`, not `clm init`.
    let already = sentence_with(&body, "already under CLM");
    for needle in ["propose", "without running `clm init`"] {
        assert!(already.contains(needle), "{needle}: {already}");
    }
    // `clm init` is named with what it writes, and the human sees its next steps.
    let writes = sentence_with(&body, "what it writes");
    for needle in [
        "`clm.toml`",
        "`scripts/clm-check.sh`",
        "where missing",
        "`ARCHITECTURE.md`",
        "`CAPABILITIES.md`",
        "`specs/SPEC.md`",
        "`CLAUDE.md`",
    ] {
        assert!(writes.contains(needle), "{needle}: {writes}");
    }
    for gone in ["Layer 1", "`ARCHITECTURE.md`'s node map"] {
        assert!(!body.contains(gone), "init: SKILL.md must not name {gone}");
    }
    assert!(body.contains("`Next steps (yours):`"), "{body}");
    let no_origin = sentence_with(&body, "no `origin`");
    assert!(no_origin.contains("nothing to check"), "{no_origin}");
    // With code, the human chooses all or one part first, before the map, hearing the
    // trade-off; the map's `outside` becomes the root's.
    let adopt = body.find("`incremental-adoption`").unwrap();
    assert!(adopt < body.find("Run `clm survey`").unwrap());
    let tradeoff = sentence_with(&body, "Say the trade-off");
    for needle in [
        "smaller first review",
        "finishes once that part's tests cite its requirements",
        "no test proves it",
        "drift",
        "widens the graph",
        "out of the root's `outside`",
    ] {
        assert!(tradeoff.contains(needle), "{needle}: {tradeoff}");
    }
    // The map holds the root's entry, and the human agrees to it before nodes are filled.
    let map = sentence_with(&body, "Run `clm survey` and draft");
    assert!(map.contains("an entry for the root component"), "{map}");
    let agree = sentence_with(&body, "revise it until they agree");
    assert!(agree.contains("show the human the map"), "{agree}");
    // Onboarding asks the four things; deploy, release and scheduled workflows stay, and
    // what moved is listed for the reviewer.
    for needle in [
        "what the project is about",
        "its first capabilities",
        "its architectural constraints",
        "language and stack",
        "Deploy, release and scheduled workflows and pipelines stay",
        "Keep the list of what moved for `## Intent`",
    ] {
        assert!(body.contains(needle), "init: SKILL.md must say {needle}");
    }
    // The init change edits `ARCHITECTURE.md`, so it needs the human's approval.
    let gate = sentence_with(&body, "its first line is `spec review`");
    for needle in ["`ARCHITECTURE.md`", "`CAPABILITIES.md`"] {
        assert!(gate.contains(needle), "{needle}: {gate}");
    }
    let root = sentence_with(&body, "replacing the top-level entries");
    for needle in ["`outside`", "`[]`", "all of the code"] {
        assert!(root.contains(needle), "{needle}: {root}");
    }
    // The map is checked before any node is filled.
    let map = body.find("clm survey --components").unwrap();
    let fill = body
        .find("its own subagent")
        .expect("init: SKILL.md must fill each component with its own subagent");
    assert!(map < fill, "init: the node map comes before filling nodes");
    // The nodes' builds prove the requirements and the root component's build adds the detectors;
    // there is no `finish` and there are no tasks (`node-reconciliation`).
    for needle in [
        "clm superpowers brainstorming",
        "clm init",
        "--no-detectors",
        "`bug`",
        "versioned markers",
        "root component's build",
        "`propose`'s steps",
        "clm land",
    ] {
        assert!(
            body.contains(needle),
            "init: SKILL.md must mention {needle}"
        );
    }
    // The human pushes the bootstrap commit; the workflow move is warned about first and
    // rides in that commit, never on the spec branch.
    let bootstrap = sentence_with(&body, "pushes this one commit");
    assert!(bootstrap.contains("human"), "{bootstrap}");
    let warn = body
        .find("warn the human")
        .expect("init: warn before moving workflows");
    assert!(warn < body.find("delete those workflows").unwrap());
    let moved = sentence_with(&body, "bootstrap commit, before the spec branch");
    assert!(moved.contains("never on the spec branch"), "{moved}");
    let stale = sentence_with(&body, "no longer matches a file");
    assert!(stale.contains("`outside`"), "{stale}");
    for needle in ["`finish`", "change's tasks"] {
        assert!(
            !body.contains(needle),
            "init: SKILL.md must not mention {needle}"
        );
    }
    // Each scenario: a skill without the needle, or naming a removed file, is reported.
    for needle in [
        "clm survey --components",
        "clm approve",
        "bootstrap commit",
        "incremental-adoption",
        "azure-pipelines.yml",
        "Bypass policies when pushing",
        "path filter",
    ] {
        let gaps = init_skill_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "init", needle);
    }
    let gaps = init_skill_gaps(&body.replace("outside", "elided"));
    assert!(
        gaps.contains(&"init: SKILL.md must mention outside".to_owned()),
        "{gaps:?}"
    );
    for (old, named) in [
        ("It writes `.github/workflows/clm.yml`.", "clm.yml"),
        (
            "Move the steps into `.github/scripts/clm-build.sh`.",
            "clm-build.sh",
        ),
    ] {
        let gaps = init_skill_gaps(&format!("{body}\n{old}\n"));
        assert_reports(&gaps, "init", named);
    }
    // bootstrap check: the bootstrap step names neither, though another step names the
    // script.
    let unchecked = body.replace(
        "check that `origin/main` has `clm.toml` or `scripts/clm-check.sh`",
        "check it",
    );
    assert!(unchecked.contains("scripts/clm-check.sh"));
    let gaps = init_skill_gaps(&unchecked);
    assert!(
        gaps.iter()
            .any(|g| g.starts_with("init:") && g.contains("scripts/clm-check.sh")),
        "{gaps:?}"
    );
}

/// What `init-skill-concurrent-tests` asks of the `init` skill.
fn concurrent_tests_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "init",
        body,
        &[
            "CLM_SLOT",
            "tests-run-concurrently",
            "fixed port",
            "fixed path",
            "shared between",
            "`bug` backlog item",
        ],
        &[],
    )
}

#[test]
fn init_skill_concurrent_tests() {
    // req:plugin.init/init-skill-concurrent-tests@3def0849
    let body = skill_body("init");
    assert_eq!(concurrent_tests_gaps(&body), Vec::<String>::new());
    for needle in [
        "CLM_SLOT",
        "tests-run-concurrently",
        "fixed port",
        "shared between",
    ] {
        let gaps = concurrent_tests_gaps(&body.replace(needle, "elided"));
        assert!(
            gaps.iter().any(|g| g.contains(needle)),
            "{needle}: {gaps:?}"
        );
    }
    // It tells the human once `clm init` has written the check script, and for a repository
    // with code looks for each and records it as a bug on the component that holds the fix.
    let told = sentence_with(&body, "builds several components at once");
    assert!(told.contains("`scripts/clm-check.sh`"), "{told}");
    let record = sentence_with(
        &body,
        "whose `components` name the component whose `paths` hold the fix",
    );
    assert!(record.contains("tests-run-concurrently"), "{record}");
}

/// What `init-skill-build-setup` asks of the `init` skill.
fn build_setup_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "init",
        body,
        &[
            "tests-run-in-their-own-tree",
            "[build] setup",
            "clm setup",
            "fresh worktree",
        ],
        &[],
    )
}

#[test]
fn init_skill_build_setup() {
    // req:plugin.init/init-skill-build-setup@a7cb5fa5
    let body = skill_body("init");
    assert_eq!(build_setup_gaps(&body), Vec::<String>::new());
    for needle in [
        "tests-run-in-their-own-tree",
        "[build] setup",
        "clm setup",
        "fresh worktree",
    ] {
        let gaps = build_setup_gaps(&body.replace(needle, "elided"));
        assert!(
            gaps.iter().any(|g| g.contains(needle)),
            "{needle}: {gaps:?}"
        );
    }
    // It tells the human once `clm init` has written `clm.toml`, and for a repository with
    // code proposes the command for the human to confirm.
    let told = sentence_with(&body, "nothing installed");
    assert!(told.contains("`clm.toml`"), "{told}");
    let propose = sentence_with(&body, "whether a fresh worktree needs such a command");
    assert!(propose.contains("confirm"), "{propose}");
}

const INIT_VERIFY: &[&str] = &[
    "[verify]",
    "local",
    "pipeline",
    "import",
    "change-done",
    "work-done",
    "ci-and-deployment",
    "verify-after-land",
];

#[test]
fn init_skill_asks_verify() {
    // req:plugin.init/init-skill-asks-verify@e80fbfc7
    let body = skill_body("init");
    assert_eq!(lacks("init", &body, INIT_VERIFY), Vec::<String>::new());
    for needle in INIT_VERIFY {
        let gaps = lacks("init", &body.replace(needle, "elided"), INIT_VERIFY);
        assert!(!gaps.is_empty(), "{needle}");
    }
}
