//! The plugin under `plugin/`: its package, hooks, skills, agent and `bin/clm` shim.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use tempfile::TempDir;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn plugin() -> PathBuf {
    repo().join("plugin")
}

fn json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap()
}

fn workspace_version() -> String {
    let text = fs::read_to_string(repo().join("Cargo.toml")).unwrap();
    let mut in_package = false;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_package = line == "[workspace.package]";
        } else if in_package && let Some(v) = line.strip_prefix("version = ") {
            return v.trim_matches('"').to_owned();
        }
    }
    panic!("no [workspace.package] version");
}

/// A plugin's `plugin.json` names the workspace version, and a failure names both.
fn assert_workspace_version(manifest: &Value, plugin: &str) {
    let (version, workspace) = (&manifest["version"], workspace_version());
    assert!(
        version.as_str() == Some(workspace.as_str()),
        "{plugin}'s plugin.json version {version} differs from the workspace version {workspace}"
    );
}

#[test]
fn package() {
    // req:plugin/package@c9c402a7
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    let listed = market["plugins"].as_array().unwrap();
    assert_eq!(listed[0]["name"], "clm", "{market}");
    assert_eq!(listed[0]["source"], "./plugin");
    let manifest = json(&plugin().join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm");
    assert_workspace_version(&manifest, "plugin");
}

#[test]
fn manifest_owner() {
    // req:plugin/manifest-owner@17c45976
    let manifest = json(&plugin().join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["author"]["name"], "Christopher Lunny");
    assert_eq!(manifest["license"], "MIT");
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    assert_eq!(market["owner"]["name"], "Christopher Lunny");
}

#[test]
fn panel_package() {
    // req:plugin.panel/panel-package@e80d5d13
    let market = json(&repo().join(".claude-plugin/marketplace.json"));
    let names: Vec<_> = market["plugins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["name"].as_str().unwrap(), p["source"].as_str().unwrap()))
        .collect();
    assert_eq!(
        names,
        [("clm", "./plugin"), ("clm-panel", "./plugin-panel")],
        "{market}"
    );
    let panel = repo().join("plugin-panel");
    assert_eq!(
        json(&panel.join("hooks/hooks.json")),
        serde_json::json!({ "modules": ["./register.tsx"] })
    );
    assert!(panel.join("hooks/register.tsx").is_file());
    assert!(
        json(&plugin().join("hooks/hooks.json"))
            .get("modules")
            .is_none()
    );
    let manifest = json(&panel.join(".claude-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm-panel");
    assert_eq!(manifest["dependencies"], serde_json::json!(["clm"]));
    assert_workspace_version(&manifest, "plugin-panel");
    let package = json(&panel.join("package.json"));
    // It runs the tests, after the type-check (`type-check`).
    let test = package["scripts"]["test"].as_str().unwrap_or_default();
    assert!(test.ends_with("vitest run"), "{package}");
    assert!(panel.join("package-lock.json").is_file());
    let npmrc = fs::read_to_string(panel.join(".npmrc")).unwrap();
    assert!(
        npmrc.lines().any(|l| l.trim() == "ignore-scripts=true"),
        "{npmrc}"
    );
}

#[test]
fn hooks_wired() {
    // req:plugin/hooks-wired@fe0e2818
    let hooks = json(&plugin().join("hooks/hooks.json"));
    let commands = |event: &str, matcher: Option<&str>| -> Vec<String> {
        hooks["hooks"][event]
            .as_array()
            .unwrap_or_else(|| panic!("no {event} in {hooks}"))
            .iter()
            .filter(|g| matcher.is_none_or(|m| g["matcher"] == m))
            .flat_map(|g| g["hooks"].as_array().unwrap().iter())
            .map(|h| h["command"].as_str().unwrap().to_owned())
            .collect()
    };
    let launcher = "\"${CLAUDE_PLUGIN_ROOT}/bin/clm\" hook ";
    assert_eq!(
        commands("PreToolUse", Some("Bash")),
        [format!("{launcher}guard")]
    );
    assert_eq!(
        commands("SessionStart", None),
        [format!("{launcher}session-start")]
    );
    assert_eq!(commands("Stop", None), [format!("{launcher}autosync")]);
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
/// How `propose` and `init` read `clm rebase`'s outcome, and the reading no skill gives.
const REBASE_RULE: &str = "until its last line is `rebase ok`";
const REBASE_BY_EXIT_CODE: &str = "`clm rebase` until it exits 0";
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

/// What `skills` asks of the skill named `skill`, one line per gap naming the skill and
/// what it lacks or must not name. Empty when it says it all.
fn skill_gaps(skill: &str, body: &str) -> Vec<String> {
    let needles: &[&str] = match skill {
        "propose" => &[
            "clm hashes",
            "clm check",
            "clm apply",
            "clm rebase",
            "clm gate",
            "`spec review`",
            "`human_burdens`",
            "clm pr open",
            "clm approve",
            "clm land",
            "clm wrap-up",
            "## Intent",
            REBASE_RULE,
        ],
        // Each change `clm wrap-up` reports `complete <id>` is archived through a code
        // branch it lands.
        "build" => &[
            "clm next",
            "clm run",
            "isolation",
            "clm rebase",
            "clm pending",
            "clm land",
            "clm wrap-up",
            "`complete <id>`",
            "impl/archive",
        ],
        // `clm rebase`'s outcome is its last line, never its exit code.
        "init" => &[REBASE_RULE],
        _ => &[],
    };
    let mut gaps: Vec<String> = [LAND_RULE, APPROVE_RULE]
        .iter()
        .chain(needles)
        .filter(|needle| !body.contains(*needle))
        .map(|needle| format!("{skill}: SKILL.md must name `{needle}`"))
        .collect();
    // The host is reached through `clm pr`, and approvals are `clm approve`: no labels.
    let forbidden = RETIRED.iter();
    // Changes declare no order against each other (`node-reconciliation`).
    let order: &[&str] = if skill == "propose" {
        &["`prefer_after`", "`after`"]
    } else {
        &[]
    };
    gaps.extend(
        forbidden
            .chain(order)
            .filter(|needle| body.contains(*needle))
            .map(|needle| format!("{skill}: SKILL.md must not name `{needle}`")),
    );
    if body.contains(REBASE_BY_EXIT_CODE) {
        gaps.push(format!(
            "{skill}: SKILL.md must not say {REBASE_BY_EXIT_CODE}: its last line, `rebase ok`, says the outcome"
        ));
    }
    gaps
}

#[test]
fn skills() {
    // req:plugin/skills@b943c3b9
    let skills: Vec<String> = fs::read_dir(plugin().join("skills"))
        .unwrap()
        .map(Result::unwrap)
        .filter(|entry| entry.file_type().unwrap().is_dir())
        .map(|entry| entry.file_name().into_string().unwrap())
        .collect();
    for skill in ["propose", "build"] {
        assert!(skills.iter().any(|s| s == skill), "{skills:?}");
    }
    for skill in &skills {
        let (fm, body) = frontmatter(&plugin().join(format!("skills/{skill}/SKILL.md")));
        assert!(
            fm["description"].as_str().is_some_and(|d| !d.is_empty()),
            "{skill}: no description"
        );
        assert_eq!(skill_gaps(skill, &body), Vec::<String>::new());
    }
    let (_, propose) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    let (_, build) = frontmatter(&plugin().join("skills/build/SKILL.md"));
    // skill without the rule
    let gaps = skill_gaps("propose", &propose.replace("clm approve", "elided"));
    assert!(
        gaps.contains(&format!("propose: SKILL.md must name `{APPROVE_RULE}`")),
        "{gaps:?}"
    );
    let gaps = skill_gaps("build", &build.replace(APPROVE_RULE, "elided"));
    assert_eq!(
        gaps,
        [format!("build: SKILL.md must name `{APPROVE_RULE}`")]
    );
    // skill without sync
    let gaps = skill_gaps("build", &build.replace("clm rebase", "elided"));
    assert_eq!(gaps, ["build: SKILL.md must name `clm rebase`"]);
    // skill without land
    let gaps = skill_gaps("propose", &propose.replace("clm land", "elided"));
    assert!(
        gaps.iter()
            .all(|g| g.starts_with("propose:") && g.contains("clm land")),
        "{gaps:?}"
    );
    assert!(!gaps.is_empty());
    // skill that orders changes
    let gaps = skill_gaps("propose", &format!("{propose}\nSet `prefer_after`.\n"));
    assert!(
        gaps.len() == 1 && gaps[0].starts_with("propose:") && gaps[0].contains("prefer_after"),
        "{gaps:?}"
    );
    // build without archiving
    let gaps = skill_gaps("build", &build.replace("complete", "done"));
    assert!(
        gaps.iter()
            .any(|g| g.starts_with("build:") && g.contains("complete")),
        "{gaps:?}"
    );
    // sync read by its exit code
    for (skill, body) in [("propose", &propose), ("init", &skill_body("init"))] {
        let said = body.replace(REBASE_RULE, "until it exits 0");
        let gaps = skill_gaps(skill, &said);
        assert!(
            !gaps.is_empty()
                && gaps
                    .iter()
                    .all(|g| g.starts_with(&format!("{skill}:")) && g.contains("rebase ok")),
            "{gaps:?}"
        );
        assert!(
            gaps.iter().any(|g| g.contains(REBASE_BY_EXIT_CODE)),
            "{skill}: {gaps:?}"
        );
    }
    for (skill, body) in [("propose", &propose), ("init", &skill_body("init"))] {
        let rule = sentence_with(body, REBASE_RULE);
        assert!(rule.contains("never from its exit code"), "{skill}: {rule}");
    }
    // skill naming a tier
    let soft = "A `SOFT` change is reviewed with `clm review <id>` after it lands.";
    let gaps = skill_gaps("propose", &format!("{propose}\n{soft}\n"));
    assert!(
        gaps.contains(&"propose: SKILL.md must not name `SOFT`".to_owned()),
        "{gaps:?}"
    );
    let hard = "Write `gate: HARD` into `change.md`.";
    let gaps = skill_gaps("init", &format!("{}\n{hard}\n", skill_body("init")));
    assert!(
        gaps.contains(&"init: SKILL.md must not name `gate: `".to_owned()),
        "{gaps:?}"
    );
    // A branch whose gate prints `spec review` as its first line lands after the human's
    // `clm approve`, and any other spec branch on checks; each outcome is bound to its own
    // token, so swapping them fails.
    let rule = sentence_with(&propose, "lands only after the human's `clm approve`");
    for needle in [
        "first line",
        "is `spec review`, it lands only after the human's `clm approve`",
        "is `spec`, it lands on checks",
    ] {
        assert!(rule.contains(needle), "{needle}: {rule}");
    }
    // Landing goes by the same first line.
    let land = &numbered_steps(&propose)[10];
    for needle in [
        "first line of your last `clm gate` run",
        "For `spec`, run `clm land spec/<id>` at once",
        "For `spec review`, ask for the approval first",
    ] {
        assert!(
            land.contains(needle),
            "propose: step 11 must say {needle}: {land}"
        );
    }
    // skill naming a label
    let label =
        "gh api -X POST repos/{owner}/{repo}/issues/<pr>/labels -f 'labels[]=spec-approved'";
    let gaps = skill_gaps("propose", &format!("{propose}\nApprove with `{label}`.\n"));
    assert!(
        gaps.contains(&"propose: SKILL.md must not name `gh api`".to_owned()),
        "{gaps:?}"
    );
    // The archive is `clm rebase` on that branch and a code branch it lands, never a push to
    // main.
    let sentence = sentence_with(&build, "impl/archive");
    for needle in ["clm rebase", "code branch", "land"] {
        assert!(
            sentence.contains(needle),
            "build must archive with {needle}: {sentence}"
        );
    }
}

fn names_the_old_hash_command(text: &str) -> bool {
    text.match_indices("clm hash").any(|(at, needle)| {
        !text[at + needle.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

#[test]
fn skills_name_clm_hashes() {
    // req:plugin/skills@b943c3b9
    // Claude Code's worktree isolation refuses `clm hash`, reading it as the shell builtin.
    assert!(names_the_old_hash_command(
        "rebase stale deltas on `clm hash`."
    ));
    assert!(names_the_old_hash_command("clm hash"));
    assert!(!names_the_old_hash_command(
        "bases from `clm hashes <component>`"
    ));
    let mut files: Vec<PathBuf> = fs::read_dir(plugin().join("skills"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("SKILL.md"))
        .collect();
    files.extend(
        fs::read_dir(plugin().join("agents"))
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .filter(|path| path.extension().is_some_and(|e| e == "md")),
    );
    assert!(files.len() >= 4, "skills and agents: {files:?}");
    for file in files {
        let text = fs::read_to_string(&file).unwrap();
        assert!(
            !names_the_old_hash_command(&text),
            "{} names `clm hash`; name `clm hashes`",
            file.display()
        );
    }
}

/// The paragraph of the fix step: what it must hold, all in one paragraph.
const DEFECT_FIX_STEP: [&str; 4] = [
    "fix reported defects",
    "clm claim <component> --fix backlog/<slug>",
    "origin/main",
    "clm wrap-up",
];

/// What `defect-check-before-pr` asks of the skill named `skill`: the check before a page,
/// listing every defect with each fixed one's fix, and a paragraph that fixes the reported
/// ones once the session's branches have landed. Empty when it says it all.
fn defect_gaps(skill: &str, body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        skill,
        body,
        &[
            "clm defect",
            "## CLM defects",
            "neither reported nor fixed",
            "each fixed one with its fix",
            "naming with `--component` the component a fix would lie in",
            "unless the session-start rule has it folded into the branch",
        ],
        &[],
    );
    let fix_step = body
        .lines()
        .any(|paragraph| DEFECT_FIX_STEP.iter().all(|n| paragraph.contains(n)));
    if !fix_step {
        gaps.push(format!(
            "{skill}: SKILL.md must have a paragraph that holds `fix reported defects`, \
             `clm claim <component> --fix backlog/<slug>`, `origin/main` and `clm wrap-up`"
        ));
    }
    gaps
}

#[test]
fn skills_check_for_defects_before_a_pr() {
    // req:plugin/defect-check-before-pr@3c314926
    for skill in ["propose", "build"] {
        let body = skill_body(skill);
        assert_eq!(defect_gaps(skill, &body), Vec::<String>::new());
        // The fix runs in a worktree of the session's own, after its branches land.
        let step = body
            .lines()
            .find(|p| p.contains("fix reported defects"))
            .unwrap();
        for needle in [
            "one at a time",
            "one round only",
            "worktree",
            "never in the checkout the session started in",
            "isolated under",
            "leave the item filed",
            "deleting its branch on `origin`",
            "git branch -D",
            "Fold a defect you meet during such a fix when it lies in that fix's component",
            "before `clm wrap-up`",
            "summary",
        ] {
            assert!(step.contains(needle), "{skill}: {needle}: {step}");
        }
    }
    let build = skill_body("build");
    // skill without the check
    let gaps = defect_gaps("build", &build.replace("## CLM defects", "elided"));
    assert_reports(&gaps, "build", "## CLM defects");
    // skill without fixed defects
    let propose = skill_body("propose");
    let gaps = defect_gaps(
        "propose",
        &propose.replace("each fixed one with its fix", "elided"),
    );
    assert_reports(&gaps, "propose", "each fixed one with its fix");
    // skill without the fix step: each of its parts missing from the paragraph
    let step = build
        .lines()
        .find(|p| p.contains("fix reported defects"))
        .unwrap();
    for needle in DEFECT_FIX_STEP {
        let body = build.replace(step, &step.replace(needle, "elided"));
        let gaps = defect_gaps("build", &body);
        assert_reports(&gaps, "build", "fix reported defects");
    }
    // `init` checks for defects before its page too.
    let init = skill_body("init");
    for needle in ["clm defect", "## CLM defects"] {
        assert!(
            init.contains(needle),
            "init: SKILL.md must mention {needle}"
        );
    }
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
    // req:plugin/init-skill@8f00075e
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
        "Deploy, release and scheduled workflows stay",
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

/// The pages `clm guide` prints: `docs/guide/<page>.md`, `index` for its README, and
/// `spec-format` for `docs/spec-format.md`.
fn guide_pages() -> Vec<String> {
    let mut pages: Vec<String> = fs::read_dir(repo().join("docs/guide"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .filter_map(|name| name.strip_suffix(".md").map(str::to_owned))
        .map(|page| {
            if page == "README" {
                "index".to_owned()
            } else {
                page
            }
        })
        .collect();
    // `docs/spec-format.md` is a page too (`spec-format-in-the-guide`).
    pages.push("spec-format".to_owned());
    // `constraints` is generated from the carried entries of `ARCHITECTURE.md`
    // (`init/guide-constraints-page`).
    pages.push("constraints".to_owned());
    pages.sort();
    pages
}

/// The guide page a `propose` skill names by path (`propose-adds-glossary-terms`).
const GLOSSARY_PATH: &str = "docs/guide/glossary.md";

/// What `method-pointers` asks of every skill and agent: no `docs/guide/` path, and every
/// `clm guide <page>` it names one of `pages`. One line per gap naming the file and text.
fn guide_gaps(name: &str, text: &str, pages: &[String]) -> Vec<String> {
    let mut gaps: Vec<String> = Vec::new();
    // The glossary is the one page a skill names by path (`propose-adds-glossary-terms`):
    // it is a spec path of this repository that a spec branch edits.
    if text.replace(GLOSSARY_PATH, "").contains("docs/guide/") {
        gaps.push(format!(
            "{name}: must not name docs/guide/; name `clm guide <page>`"
        ));
    }
    for (at, needle) in text.match_indices("clm guide ") {
        let page: String = text[at + needle.len()..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        if !page.is_empty() && !pages.contains(&page) {
            gaps.push(format!(
                "{name}: names `clm guide {page}`, no such page {page}"
            ));
        }
    }
    gaps
}

#[test]
fn skills_point_at_the_guide() {
    // req:plugin/method-pointers@3db98468
    let pages = guide_pages();
    assert!(pages.contains(&"shaping-the-graph".to_owned()), "{pages:?}");
    let mut files: Vec<PathBuf> = fs::read_dir(plugin().join("skills"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("SKILL.md"))
        .collect();
    files.extend(
        fs::read_dir(plugin().join("agents"))
            .unwrap()
            .map(|entry| entry.unwrap().path()),
    );
    for file in &files {
        let text = fs::read_to_string(file).unwrap();
        assert_eq!(
            guide_gaps(&file.display().to_string(), &text, &pages),
            Vec::<String>::new()
        );
    }
    // a repository path; a page that doesn't exist
    let gaps = guide_gaps("x", "Read `docs/guide/shaping-the-graph.md`.", &pages);
    assert!(
        gaps.len() == 1 && gaps[0].contains("docs/guide/"),
        "{gaps:?}"
    );
    let gaps = guide_gaps("x", "Run `clm guide no-such-page`.", &pages);
    assert!(
        gaps.len() == 1 && gaps[0].contains("no-such-page"),
        "{gaps:?}"
    );
    assert!(guide_gaps("x", "Run `clm guide <page>`.", &pages).is_empty());
    // init explains the shape before the node map is drafted, and points at the flow.
    let init = skill_body("init");
    for needle in [
        "clm guide shaping-the-graph",
        "clm guide using-clm",
        "clm guide how-it-works",
    ] {
        assert_reports(
            &text_gaps("init", &init.replace(needle, "elided"), &[needle], &[]),
            "init",
            needle,
        );
    }
    let shape = init.find("clm guide shaping-the-graph").unwrap();
    for drafted in ["**A new project:**", "Run `clm survey`"] {
        assert!(
            shape < init.find(drafted).unwrap(),
            "init: `clm guide shaping-the-graph` comes before {drafted}"
        );
    }
    let explain = &init[init.find("explain the graph's shape").unwrap()..shape];
    for needle in [
        "one build session holds",
        "spec and its code",
        "nesting the carved-out part as its child",
        "shared component",
        "size budget is a visible state, not an error",
    ] {
        assert!(explain.contains(needle), "{needle}: {explain}");
    }
    let summary = numbered_steps(&init)
        .into_iter()
        .find(|s| s.contains("End with a summary"))
        .expect("init: SKILL.md must end with a summary step");
    assert!(
        summary.contains("clm guide using-clm"),
        "init: summary must name `clm guide using-clm`: {summary}"
    );
    assert!(
        summary.contains("clm guide how-it-works"),
        "init: summary must name `clm guide how-it-works`: {summary}"
    );
}

#[test]
fn skills_run_a_builder_check() {
    // req:plugin/builder-check@db60baf8
    // The check is a numbered step of its own, which says all of this itself, so text
    // elsewhere in the skill can't stand in for it.
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    let steps = numbered_steps(&body);
    let check = steps
        .iter()
        .position(|s| s.contains("Run a builder check"))
        .expect("skills/propose/SKILL.md must give the builder check its own numbered step");
    for needle in [
        "fresh subagent",
        "## Open questions",
        "`backlog/<slug>.md`, of kind `question` or `human-task`",
        "`blocks`",
        "clm gate",
    ] {
        assert!(
            steps[check].contains(needle),
            "skills/propose/SKILL.md's builder check must mention {needle}"
        );
    }
    let blocks = sentence_with(&steps[check], "`blocks`");
    assert!(
        blocks.contains("requirements"),
        "skills/propose/SKILL.md's `blocks` must name the requirements that wait: {blocks}"
    );
    // A requirement waiting on another node's is a `#### Needs` block, not a backlog item.
    let needs = steps[check]
        .lines()
        .find(|l| l.contains("`#### Needs`"))
        .expect("skills/propose/SKILL.md's builder check must mention `#### Needs`");
    for needle in [
        "another component's pending requirement",
        "`- <component>/<slug>`",
        "`requirement-needs`",
        "not a backlog item",
    ] {
        assert!(
            needs.contains(needle),
            "skills/propose/SKILL.md's `#### Needs` must mention {needle}: {needs}"
        );
    }
    assert!(
        !needs.contains("`question`"),
        "skills/propose/SKILL.md's `#### Needs` must not offer a backlog item instead: {needs}"
    );
    // A removed or renamed part of a Contract another node's code uses needs the
    // requirements that move those nodes off it, which a REMOVED-only delta can't carry.
    let removal = steps[check]
        .lines()
        .find(|l| l.contains("`contract-removal-needs`"))
        .expect("skills/propose/SKILL.md's builder check must mention `contract-removal-needs`");
    for needle in [
        "removes or renames",
        "another component's code uses",
        "`REMOVED Requirements`",
        "`expand-contract`",
        "search the code of the other components",
    ] {
        assert!(removal.contains(needle), "{needle}: {removal}");
    }
    // A rename is two requirements of the provider: an adding one beside the old name, and a
    // removing one that needs each user's move.
    let rename = steps[check]
        .lines()
        .find(|l| l.contains("`rename-adds-then-removes`"))
        .expect("skills/propose/SKILL.md's builder check must mention `rename-adds-then-removes`");
    for needle in [
        "two requirements of the provider",
        "offers the new name beside the old",
        "a second name, never a widened type",
        "needs each user's move",
        "no cycle",
        "`clm:remove-by`",
        "`expand-contract`",
    ] {
        assert!(rename.contains(needle), "{needle}: {rename}");
    }
    assert!(
        removal.contains(
            "exhaustive ones (a `match`, a `switch` with no default, a map keyed by the type) included"
        ),
        "{removal}"
    );
    // A removal that can't carry that `#### Needs` goes in a second, deferred change.
    let deferred = steps[check]
        .lines()
        .find(|l| l.contains("`contract-step-deferred`"))
        .expect("skills/propose/SKILL.md's builder check must mention `contract-step-deferred`");
    for needle in [
        "the removing component is required and the component that must move is optional",
        "second change on the same branch",
        "`proposed`",
        "its `components` are the contract change's components",
        "with the version it must be current at",
        "`kind: deferred`",
        "`resolves` doesn't name the item",
        "`clm triage`",
    ] {
        assert!(deferred.contains(needle), "{needle}: {deferred}");
    }
    // Work a requirement relies on from another node is that node's requirement.
    let work = steps[check]
        .lines()
        .find(|l| l.contains("builds nothing"))
        .expect("skills/propose/SKILL.md's builder check must mention `builds nothing`");
    for needle in [
        "`dependency-work-is-a-requirement`",
        "no requirement of that component states",
        "`#### Needs`",
    ] {
        assert!(work.contains(needle), "{needle}: {work}");
    }
    // The builder asks which not-yet-current requirements each one relies on, and a
    // same-node reliance carries the needs of the requirement it waits on.
    let order = steps[check]
        .lines()
        .find(|l| l.contains("`builder-check-asks-order`"))
        .expect("skills/propose/SKILL.md's builder check must mention `builder-check-asks-order`");
    for needle in [
        "for each requirement the change adds or modifies",
        "`clm pending`",
        "`none`",
        "its own component",
        "carries that block's needs",
    ] {
        assert!(order.contains(needle), "{needle}: {order}");
    }
    // skill without contract removals, dependency work, the order ask or same-node needs
    for needle in [
        "contract-removal-needs",
        "contract-step-deferred",
        "rename-adds-then-removes",
        "builds nothing",
        "builder-check-asks-order",
        "its own component",
    ] {
        let gaps = text_gaps("propose", &body.replace(needle, "elided"), &[needle], &[]);
        assert_reports(&gaps, "propose", needle);
    }
    let pr = steps
        .iter()
        .position(|s| s.contains("clm pr open"))
        .expect("skills/propose/SKILL.md must open the pull request with clm pr open");
    assert!(
        check < pr,
        "skills/propose/SKILL.md must run the builder check before opening the pull request"
    );
    // `build` lists the blocking items when it stops, in its summary at the end.
    let (_, body) = frontmatter(&plugin().join("skills/build/SKILL.md"));
    let end = numbered_steps(&body)
        .into_iter()
        .find(|s| s.contains("At the end"))
        .expect("skills/build/SKILL.md must have a step for the end");
    for needle in ["`question`", "`human-task`", "requirements they block"] {
        assert!(
            end.contains(needle),
            "skills/build/SKILL.md's last step must mention {needle}"
        );
    }
    // Stopping early still ends with that summary.
    let stop = body
        .lines()
        .find(|l| l.starts_with("Stop and tell the user"))
        .expect("skills/build/SKILL.md must say when to stop");
    assert!(
        stop.contains("step 5's summary"),
        "skills/build/SKILL.md must end an early stop with step 5's summary"
    );
}

#[test]
fn builder_check_reapplies_through_rebase() {
    // req:plugin/builder-check@db60baf8
    // By the builder check the change is `applied`, and `clm apply` refuses an applied
    // change (`apply/change-status`): a delta edited then reaches `specs/` through
    // `clm rebase`, which regenerates it from the applied deltas.
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    let steps = numbered_steps(&body);
    let check = steps
        .iter()
        .find(|s| s.contains("Run a builder check"))
        .expect("skills/propose/SKILL.md must give the builder check its own numbered step");
    let needs = check
        .lines()
        .find(|l| l.contains("`#### Needs`"))
        .expect("skills/propose/SKILL.md's builder check must mention `#### Needs`");
    assert!(
        needs.contains("clm rebase"),
        "skills/propose/SKILL.md must re-apply an edited delta through `clm rebase`: {needs}"
    );
    for (i, step) in steps.iter().enumerate() {
        assert!(
            !step.contains("re-run `clm apply"),
            "skills/propose/SKILL.md step {} re-runs `clm apply` on an applied change",
            i + 1
        );
    }
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

#[test]
fn propose_brainstorms() {
    // req:plugin/propose-brainstorms@3e173409
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    for needle in [
        "clm superpowers brainstorming",
        "## Approaches",
        "docs/superpowers/",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    // A prohibition, not a permission: the never comes before the path in its own
    // sentence, since a markdown line can hold a whole paragraph.
    let sentence = sentence_with(&body, "docs/superpowers/");
    let before = &sentence[..sentence.find("docs/superpowers/").unwrap()];
    assert!(
        before.contains("never") || before.contains("Never"),
        "propose: SKILL.md must say never to write under docs/superpowers/"
    );
    // Brainstorming comes before the change is written.
    let brainstorm = body.find("clm superpowers brainstorming").unwrap();
    let change = body
        .find("/change.md` with the intent")
        .expect("change.md step");
    assert!(
        brainstorm < change,
        "propose: SKILL.md must brainstorm before writing change.md"
    );
}

#[test]
fn spec_auditor() {
    // req:plugin/spec-auditor@312da3a0
    let (fm, body) = frontmatter(&plugin().join("agents/spec-auditor.md"));
    assert_eq!(fm["name"], "spec-auditor");
    assert!(fm["description"].as_str().is_some_and(|d| !d.is_empty()));
    let tools: Vec<&str> = fm["tools"]
        .as_str()
        .expect("tools listed")
        .split(',')
        .map(str::trim)
        .collect();
    for tool in ["Edit", "Write", "NotebookEdit"] {
        assert!(!tools.contains(&tool), "spec-auditor may not have {tool}");
    }
    for needle in [
        "clm context",
        "clm pending",
        "scenario",
        "**Implementation:**",
        "current version",
        "would fail",
        "Never edit",
    ] {
        assert!(
            body.contains(needle),
            "spec-auditor prompt lacks `{needle}`"
        );
    }
    // It is launched with a node ID; there are no tasks or plans to read.
    for needle in ["change ID", "plan.md", "CLM task", "task's"] {
        assert!(
            !body.contains(needle) && !fm["description"].as_str().unwrap().contains(needle),
            "spec-auditor must not mention {needle}"
        );
    }
}

/// What of `needles` the auditor's prompt leaves out, one line per missing piece naming
/// the agent and the text. Empty when it says it all.
fn auditor_gaps(needles: &[&str], body: &str) -> Vec<String> {
    needles
        .iter()
        .filter(|needle| !body.contains(*needle))
        .map(|needle| format!("spec-auditor: must name `{needle}`"))
        .collect()
}

/// What `auditor-flags-unstated-fixes` asks of the auditor's prompt.
fn unstated_fix_gaps(body: &str) -> Vec<String> {
    auditor_gaps(&["code-fix-needs-a-written-rule"], body)
}

#[test]
fn spec_auditor_flags_unstated_fixes() {
    // req:plugin/auditor-flags-unstated-fixes@084d9c14
    let (_, body) = frontmatter(&plugin().join("agents/spec-auditor.md"));
    assert_eq!(unstated_fix_gaps(&body), Vec::<String>::new());
    // The instruction: report each test whose cited text doesn't state what it checks.
    let rule = sentence_with(&body, "code-fix-needs-a-written-rule");
    for needle in [
        "each test",
        "at the cited version",
        "doesn't state the behaviour the test checks",
        "naming the test and the requirement",
    ] {
        assert!(rule.contains(needle), "{needle}: {rule}");
    }
    // agent without the instruction
    let gaps = unstated_fix_gaps(&body.replace("code-fix-needs-a-written-rule", "elided"));
    assert_eq!(
        gaps,
        ["spec-auditor: must name `code-fix-needs-a-written-rule`"]
    );
}

/// What `health-skill` asks of the `health` skill: it reads `clm health`, has the slimmer
/// look at the bloated nodes, and acts on nothing itself.
fn health_skill_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "health",
        body,
        &[
            "clm health",
            "reconsider",
            "score",
            "spec-slimmer",
            "`design-heavy`",
            "`history-words`",
            "`outlier-size`",
            "`unexercised`",
            "/clm:propose",
            "clm guide using-clm",
            "edit no spec",
            "file no backlog item",
            "land nothing",
        ],
        &["clm apply", "clm defect", "clm friction"],
    )
}

#[test]
fn health_skill() {
    // req:plugin/health-skill@71b58876
    let (fm, body) = frontmatter(&plugin().join("skills/health/SKILL.md"));
    assert!(fm["description"].as_str().is_some_and(|d| !d.is_empty()));
    assert_eq!(health_skill_gaps(&body), Vec::<String>::new());
    // skill without the command
    let gaps = health_skill_gaps(&body.replace("clm health", "elided"));
    assert_reports(&gaps, "health", "clm health");
    // skill without the slimmer
    let gaps = health_skill_gaps(&body.replace("spec-slimmer", "elided"));
    assert_reports(&gaps, "health", "spec-slimmer");
    // skill that acts
    let gaps = health_skill_gaps(&format!("{body}\nThen run `clm apply <id>`.\n"));
    assert_reports(&gaps, "health", "clm apply");
}

/// The commands that would change the auditor's checkout, and the read of `origin/main`
/// that doesn't, which `spec-auditor-keeps-checkout` has it name.
const AUDITOR_CHECKOUT: [&str; 5] = [
    "git switch",
    "git checkout",
    "git stash",
    "git reset",
    "git show origin/main:",
];

/// What `spec-auditor-keeps-checkout` asks of the auditor's prompt.
fn auditor_checkout_gaps(body: &str) -> Vec<String> {
    auditor_gaps(&AUDITOR_CHECKOUT, body)
}

#[test]
fn spec_auditor_keeps_its_checkout() {
    // req:plugin/spec-auditor-keeps-checkout@ab433b2d
    let (_, body) = frontmatter(&plugin().join("agents/spec-auditor.md"));
    assert_eq!(auditor_checkout_gaps(&body), Vec::<String>::new());
    let never = sentence_with(&body, "git stash");
    for needle in ["Never", "checkout"] {
        assert!(never.contains(needle), "{needle}: {never}");
    }
    // The pending list on `origin/main` needs no checkout of it.
    let pending = sentence_with(&body, "pending list on `origin/main`");
    for needle in ["clm pending <component>", "current version", "removes"] {
        assert!(pending.contains(needle), "{needle}: {pending}");
    }
    // auditor that changes directory: unattended sessions refuse a `cd` followed by `git`.
    let cd = sentence_with(&body, "`cd`");
    for needle in ["never", "wrap one in `("] {
        assert!(cd.contains(needle), "{needle}: {cd}");
    }
    // auditor that may switch the checkout
    for needle in AUDITOR_CHECKOUT {
        let gaps = auditor_checkout_gaps(&body.replace(needle, "elided"));
        assert_eq!(gaps, [format!("spec-auditor: must name `{needle}`")]);
    }
}

/// The tools an agent's frontmatter lists.
fn agent_tools(fm: &serde_yaml::Value) -> Vec<String> {
    fm["tools"]
        .as_str()
        .expect("tools listed")
        .split(',')
        .map(|tool| tool.trim().to_owned())
        .collect()
}

/// What `spec-slimmer` asks of the slimmer's frontmatter and prompt.
fn slimmer_gaps(fm: &serde_yaml::Value, body: &str) -> Vec<String> {
    let tools = agent_tools(fm);
    let mut gaps: Vec<String> = ["Edit", "Write", "NotebookEdit"]
        .into_iter()
        .filter(|tool| tools.iter().any(|t| t == tool))
        .map(|tool| format!("spec-slimmer: may not have `{tool}`"))
        .collect();
    gaps.extend(
        [
            "clm context <component>",
            "SPEC.md",
            "`paths`",
            "quote",
            "restates a requirement",
            "narrates what changed",
            "implementation detail",
            "duplicates another",
            "prove the same thing",
            "code no requirement needs",
            "Never edit",
            "severity",
        ]
        .into_iter()
        .filter(|needle| !body.contains(needle))
        .map(|needle| format!("spec-slimmer: must name `{needle}`")),
    );
    gaps
}

#[test]
fn spec_slimmer() {
    // req:plugin/spec-slimmer@8de89fe3
    let (fm, body) = frontmatter(&plugin().join("agents/spec-slimmer.md"));
    assert_eq!(fm["name"], "spec-slimmer");
    assert!(fm["description"].as_str().is_some_and(|d| !d.is_empty()));
    assert_eq!(slimmer_gaps(&fm, &body), Vec::<String>::new());
    // agent that could edit
    let mut editing = fm.clone();
    editing["tools"] = format!("{}, Edit", fm["tools"].as_str().unwrap()).into();
    let gaps = slimmer_gaps(&editing, &body);
    assert_eq!(gaps, ["spec-slimmer: may not have `Edit`"]);
    // agent without the context command
    let gaps = slimmer_gaps(&fm, &body.replace("clm context", "elided"));
    assert_eq!(gaps, ["spec-slimmer: must name `clm context <component>`"]);
}

/// What `issue-verifier` asks of the verifier's frontmatter and prompt.
fn verifier_gaps(fm: &serde_yaml::Value, body: &str) -> Vec<String> {
    let tools = agent_tools(fm);
    let mut gaps: Vec<String> = tools
        .iter()
        .filter(|tool| !["Read", "Glob", "Grep"].contains(&tool.as_str()))
        .map(|tool| format!("issue-verifier: may not have `{tool}`"))
        .collect();
    gaps.extend(
        ["Read", "Glob", "Grep"]
            .into_iter()
            .filter(|tool| !tools.iter().any(|t| t == tool))
            .map(|tool| format!("issue-verifier: must have `{tool}`")),
    );
    gaps.extend(
        [
            ".clm/intake/",
            "untrusted",
            "ARCHITECTURE.md",
            "CAPABILITIES.md",
            "backlog/",
            "specs/",
            "changes/",
            "docs/decisions/",
            "`paths`",
            "`real`",
            "`fixed`",
            "`duplicate`",
            "`out of scope`",
            "`issues`",
            "quoting",
            "copying no sentence",
            "overlaps",
            "Assign no severity",
            "never edit files",
            "nothing else",
            "together",
            "backlog kind",
            "clm contribution show",
            "`contribution` skill",
        ]
        .into_iter()
        .filter(|needle| !body.contains(needle))
        .map(|needle| format!("issue-verifier: must name `{needle}`")),
    );
    let description = fm["description"].as_str().unwrap_or_default();
    gaps.extend(
        [
            "issues",
            "pull requests",
            "backlog",
            "specs",
            "code",
            "ARCHITECTURE.md",
            "read-only",
        ]
        .into_iter()
        .filter(|needle| !description.contains(needle))
        .map(|needle| format!("issue-verifier: description must name `{needle}`")),
    );
    gaps
}

#[test]
fn issue_verifier() {
    // req:plugin/issue-verifier@28b4ce10
    let (fm, body) = frontmatter(&plugin().join("agents/issue-verifier.md"));
    assert_eq!(fm["name"], "issue-verifier");
    assert!(fm["description"].as_str().is_some_and(|d| !d.is_empty()));
    assert_eq!(verifier_gaps(&fm, &body), Vec::<String>::new());
    // verifier that could run commands
    let mut running = fm.clone();
    running["tools"] = format!("{}, Bash", fm["tools"].as_str().unwrap()).into();
    assert_eq!(
        verifier_gaps(&running, &body),
        ["issue-verifier: may not have `Bash`"]
    );
    // prompt that drops the untrusted-data rule
    let gaps = verifier_gaps(&fm, &body.replace("untrusted", "elided"));
    assert_eq!(gaps, ["issue-verifier: must name `untrusted`"]);
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
    // req:plugin/build-subagents@2b7cae34
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
    for old in ["clm next --interactive", "PR #<number>"] {
        let gaps = subagent_gaps(&format!("{body}\n`{old}`\n"));
        assert_eq!(gaps, [format!("build: SKILL.md must not mention {old}")]);
    }
    // The subagent is launched only on a clean exit, with the printed prompt.
    let launch = sentence_with(&body, "isolation: \"worktree\"");
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
    for needle in ["same way", "until it lands", "twice", "same reason"] {
        assert!(retry.contains(needle), "{needle}: {retry}");
    }
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

/// What `build-finishes-builds` asks of a `build` skill.
fn finish_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "clm run --finish <component>",
            "exits 3",
            "max_builds",
            "clm.local.toml",
        ],
        &["CARGO_TARGET_DIR"],
    )
}

#[test]
fn build_finishes_builds() {
    // req:plugin/build-finishes-builds@0c3d6e0a
    let body = skill_body("build");
    assert_eq!(finish_gaps(&body), Vec::<String>::new());
    // build without finishing
    let gaps = finish_gaps(&body.replace("clm run --finish", "elided"));
    assert_reports(&gaps, "build", "clm run --finish");
    let gaps = finish_gaps(&format!("{body}\nSet `CARGO_TARGET_DIR`.\n"));
    assert_reports(&gaps, "build", "CARGO_TARGET_DIR");
    // Builds launch only from the latest `clm next`, which runs again after each returns.
    let latest = sentence_with(&body, "latest `clm next`");
    assert!(latest.contains("launch builds only"), "{latest}");
    let again = sentence_with(&body, "run `clm next` again");
    assert!(again.contains("returns"), "{again}");
    // Exit 3 launches nothing; finishing comes first, whether it landed or stopped.
    let full = sentence_with(&body, "exits 3");
    for needle in ["no free place", "launch nothing for that component"] {
        assert!(full.contains(needle), "{needle}: {full}");
    }
    let finish = sentence_with(&body, "clm run --finish <component>");
    for needle in [
        "landed or stopped",
        "before retrying or launching",
        "usage or rate limit",
        "`resume-limited-builds`",
    ] {
        assert!(finish.contains(needle), "{needle}: {finish}");
    }
    let cap = sentence_with(&body, "max_builds");
    for needle in ["none of your own builds is running", "cap is reached"] {
        assert!(cap.contains(needle), "{needle}: {cap}");
    }
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
    // req:plugin/build-archive-branch@1f264572
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
        ],
        &[],
    )
}

#[test]
fn build_reads_origin() {
    // req:plugin/build-reads-origin@6963f670
    let body = skill_body("build");
    assert_eq!(origin_gaps(&body), Vec::<String>::new());
    for needle in ["views-name-their-commit", "origin/impl/<component>"] {
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
}

/// What `session-worktrees` asks of each skill that cuts a branch in the human's session.
fn worktree_gaps(skill: &str, body: &str) -> Vec<String> {
    let must: &[&str] = match skill {
        "propose" => &[
            "git worktree add .claude/worktrees/<id> -b spec/<id> origin/main",
            "git worktree remove",
            "git switch -c spec/<id> origin/main",
        ],
        "init" => &["git worktree add .claude/worktrees/<YYYYMMDD>-init"],
        "build" => &[
            "git worktree add .claude/worktrees/archive -b impl/archive/<id> origin/main",
            "git worktree remove",
            "git switch -c impl/archive/<id> origin/main",
            "git switch --detach origin/main",
        ],
        _ => &[],
    };
    text_gaps(skill, body, must, &[])
}

#[test]
fn skills_cut_branches_in_worktrees() {
    // req:plugin/session-worktrees@6d976f97
    for skill in ["propose", "init", "build"] {
        let body = skill_body(skill);
        assert_eq!(worktree_gaps(skill, &body), Vec::<String>::new());
        // a skill switching the shared checkout, or leaving its worktree behind
        let gaps = worktree_gaps(skill, &body.replace("git worktree add", "git switch -c"));
        assert_reports(&gaps, skill, "git worktree add");
        if skill != "init" {
            let gaps = worktree_gaps(skill, &body.replace("git worktree remove", "elided"));
            assert_reports(&gaps, skill, "git worktree remove");
        }
    }
    // propose works in the worktree, never switching the checkout it started in, and
    // removes the worktree once the branch has landed, before `clm wrap-up`.
    let propose = skill_body("propose");
    let never = sentence_with(&propose, "never switch the checkout");
    assert!(never.contains("started in"), "{never}");
    let remove = propose.find("git worktree remove").unwrap();
    assert!(remove < propose.rfind("clm wrap-up").unwrap());
    let landed = sentence_with(&propose, "git worktree remove");
    for needle in ["landed", "change back to the directory you started in"] {
        assert!(landed.contains(needle), "{needle}: {landed}");
    }
    // propose with no step for an isolated session
    let gaps = worktree_gaps("propose", &propose.replace("git switch -c", "elided"));
    assert_reports(&gaps, "propose", "git switch -c");
    // A session whose checkout lies under `.claude/worktrees/` cuts the branch in place,
    // removes no worktree, and switches off the branch once landed, before the checkpoint.
    // It is the checkout the session started in that decides, not a worktree step 1 added.
    let isolated = sentence_with(&propose, "git switch -c spec/<id> origin/main");
    for needle in [".claude/worktrees/", "every git command", "started in"] {
        assert!(isolated.contains(needle), "{needle}: {isolated}");
    }
    // propose counting its own worktree as isolated: the skills' own worktrees, named by a
    // change ID or `archive`, don't count, whoever started there.
    for needle in ["`<YYYYMMDD>-<slug>`", "`archive`", "`defects`"] {
        assert!(isolated.contains(needle), "propose: {needle}: {isolated}");
    }
    // Such a session adds its worktree under the main checkout, never nested in its own.
    let main = sentence_with(&propose, "git worktree add <main>/.claude/worktrees/<id>");
    for needle in ["`git worktree list`", "first", "nested"] {
        assert!(main.contains(needle), "propose: {needle}: {main}");
    }
    let detach = sentence_with(&propose, "removes no worktree");
    assert!(
        detach.contains("git switch --detach origin/main"),
        "{detach}"
    );
    for needle in ["landed", "before `clm wrap-up`"] {
        assert!(detach.contains(needle), "{needle}: {detach}");
    }
    assert!(isolated.contains("do all later work there"), "{isolated}");
    // The landing step says the same where it removes the worktree, before its checkpoint.
    let landing = sentence_with(&propose, "removes nothing");
    assert!(
        landing.contains("git switch --detach origin/main"),
        "{landing}"
    );
    let at = propose.find(landing).unwrap();
    assert!(
        propose[at..].contains("Then run `clm wrap-up`"),
        "propose: the isolated session detaches before `clm wrap-up`"
    );
    // A resumed `spec/<id>` is switched to in place, or the session stops.
    let resume = sentence_with(&propose, "git switch spec/<id>");
    assert!(resume.contains("say so and stop"), "{resume}");
    // init cuts its spec branch after the bootstrap push.
    let init = skill_body("init");
    assert!(
        init.find("pushes this one commit").unwrap() < init.find("git worktree add").unwrap(),
        "init: the worktree comes after the bootstrap push"
    );
    // build removes the archive worktree once it has landed.
    let build = skill_body("build");
    let landed = sentence_with(&build, "remove the worktree with `git worktree remove");
    assert!(landed.contains("landed"), "{landed}");
    // build archiving with no step for an isolated session
    let gaps = worktree_gaps(
        "build",
        &build.replace("git switch -c impl/archive/<id>", "elided"),
    );
    assert_reports(&gaps, "build", "git switch -c impl/archive/<id>");
    // build fixing defects with no step for an isolated session: the paragraph holding
    // `fix reported defects` must say `isolated under`, as `propose`'s step 1 does.
    let step = build
        .lines()
        .find(|p| p.contains("fix reported defects"))
        .unwrap();
    for needle in [
        "isolated under",
        "before each `clm claim <component> --fix backlog/<slug>`",
        "add and remove no worktree",
        "or its claim or push is refused",
    ] {
        assert!(step.contains(needle), "build: {needle}: {step}");
    }
    let archive = sentence_with(&build, "git switch -c impl/archive/<id> origin/main");
    for needle in [
        "removes no worktree",
        "git switch --detach origin/main",
        "clm wrap-up",
    ] {
        assert!(archive.contains(needle), "{needle}: {archive}");
    }
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
    // req:plugin/triage-caps-drafts@d3a894ba
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
    // req:plugin/triage-skill@d7e0d41f
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
    // req:plugin/triage-answers@4cbcff5f
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
            "contribution",
        ],
        &[],
    )
}

#[test]
fn triage_takes_the_inbox_in() {
    // req:plugin/triage-contributions@8b647918
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
    let pull = sentence_with(step, "For each `pull <n>` line");
    assert!(pull.contains("`contribution` skill's steps"), "{pull}");
    assert!(step.contains("run `clm triage` again"), "{step}");
    let summary = sentence_with(&body, "end with a summary");
    assert!(
        summary.contains("the issues taken in with their items"),
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
    // req:plugin/triage-menu@3c61eff2
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
        ("*Contributions*", "`issue <n> resolved` and `pull` lines"),
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

/// The overlap check as the CLI takes it: `--node` once per node (`propose-claims`).
const PROPOSALS_FORM: &str = "clm proposals --component <component> [--component <component>]...";

/// How a `propose` skill writes the overlap check. Every inline `` `clm proposals --node` ``
/// command must be exactly [`PROPOSALS_FORM`], and there must be one. Empty when it is.
fn node_syntax_gaps(body: &str) -> Vec<String> {
    let mut gaps: Vec<String> = body
        .match_indices("`clm proposals --component")
        .map(|(at, _)| {
            let command = &body[at + 1..];
            &command[..command.find('`').unwrap_or(command.len())]
        })
        .filter(|command| *command != PROPOSALS_FORM)
        .map(|command| {
            format!("propose: SKILL.md writes `{command}`; write the form `{PROPOSALS_FORM}`")
        })
        .collect();
    if !body.contains(PROPOSALS_FORM) {
        gaps.push(format!(
            "propose: SKILL.md must write the form `{PROPOSALS_FORM}`"
        ));
    }
    gaps
}

#[test]
fn propose_claims_items_before_designing() {
    // req:plugin/propose-claims@63373bf0
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    assert_eq!(node_syntax_gaps(&body), Vec::<String>::new());
    // one flag for several nodes: the ambiguous form is reported, naming the right one,
    // even when another step still writes the right form.
    let ambiguous = body.replacen(
        PROPOSALS_FORM,
        "clm proposals --component <component>...",
        1,
    );
    let gaps = node_syntax_gaps(&ambiguous);
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    let gaps =
        node_syntax_gaps(&body.replacen(PROPOSALS_FORM, "clm proposals --component <a> <b>", 1));
    assert_eq!(gaps.len(), 1, "{gaps:?}");
    let ambiguous = body.replace(PROPOSALS_FORM, "clm proposals --component <component>...");
    let gaps = node_syntax_gaps(&ambiguous);
    assert!(
        !gaps.is_empty() && gaps.iter().all(|g| g.contains(PROPOSALS_FORM)),
        "{gaps:?}"
    );
    for needle in [
        "clm proposals",
        "clm claim backlog/",
        "clm proposals --component",
        "refused",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    let steps = numbered_steps(&body);
    let position = |needle: &str| {
        steps
            .iter()
            .position(|s| s.contains(needle))
            .unwrap_or_else(|| panic!("propose: SKILL.md has no step with {needle}"))
    };
    // Items are left out, then claimed, before brainstorming.
    let proposals = position("clm proposals");
    let claim = position("clm claim backlog/");
    let brainstorm = position("clm superpowers brainstorming");
    assert!(
        proposals < brainstorm,
        "propose: `clm proposals` before brainstorming"
    );
    assert!(
        claim < brainstorm,
        "propose: `clm claim backlog/` before brainstorming"
    );
    // Right after claiming, the spec branch holds the items.
    for needle in ["commit", "push", "`from`", "`resolves`", "status: proposed"] {
        assert!(
            steps[claim].contains(needle),
            "propose: the claim step must mention {needle}"
        );
    }
    let refused = sentence_with(&steps[claim], "refused");
    assert!(
        refused.contains("stop") && refused.contains("holds"),
        "propose: a refused claim stops, telling what holds the item: {refused}"
    );
    // The overlap check runs once the nodes are known, and again before the page, and the
    // human decides.
    let deltas = position("deltas/<component>.md");
    let pr = position("clm pr open");
    let overlap: Vec<usize> = steps
        .iter()
        .enumerate()
        .filter(|(_, s)| s.contains("clm proposals --component"))
        .map(|(i, _)| i)
        .collect();
    assert!(
        overlap.iter().any(|&i| i < deltas),
        "propose: `clm proposals --component` before the deltas, got {overlap:?}"
    );
    assert!(
        overlap.iter().any(|&i| i > deltas && i <= pr),
        "propose: `clm proposals --component` again before the page, got {overlap:?}"
    );
    for needle in ["carry on", "fold", "wait"] {
        assert!(
            steps[overlap[0]].contains(needle),
            "propose: the overlap check must offer to {needle}"
        );
    }
    // Only a `proposed` or `claimed` line is a question; a `main` line needs none.
    for &i in &overlap {
        let ask = sentence_with(&steps[i], "`proposed` or `claimed` line");
        assert!(ask.contains("ask"), "{ask}");
    }
    let main = sentence_with(&body, "`main` line");
    for needle in ["applied on `main`", "ask nothing"] {
        assert!(main.contains(needle), "{needle}: {main}");
    }
    // A claimed item already fixed stays in `resolves`: deleted, and `## Intent` says why.
    let fixed = sentence_with(&body, "already fixed");
    for needle in ["`resolves`", "`backlog/<slug>.md`", "`## Intent`"] {
        assert!(fixed.contains(needle), "{needle}: {fixed}");
    }
    // skill without already-fixed items; asks about applied changes
    for needle in ["already fixed", "`main` line"] {
        let gaps = claim_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
    assert_eq!(claim_gaps(&body), Vec::<String>::new());
}

/// What `propose-claims` asks of a `propose` skill beyond the overlap check's form.
fn claim_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &[
            "clm claim backlog/",
            "clm proposals --component",
            "already fixed",
            "`main` line",
        ],
        &[],
    )
}

/// What `propose-archives-design-only` asks of a `propose` skill: archiving, and no sentence
/// of the paragraph naming `archive-in-spec-pr` that has a conflict under `specs/` resolved
/// by hand, since `clm rebase` regenerates `specs/` for an archived change (`sync/reapply`).
fn design_only_gaps(body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        "propose",
        body,
        &["clm archive", "status: done", "archive-in-spec-pr"],
        &[],
    );
    let by_hand = body
        .lines()
        .filter(|paragraph| paragraph.contains("archive-in-spec-pr"))
        .flat_map(|paragraph| paragraph.split(". "))
        .any(|sentence| sentence.contains("specs/") && sentence.contains("by hand"));
    if by_hand {
        gaps.push(
            "propose: an archived change's conflict under `specs/` isn't resolved by hand"
                .to_owned(),
        );
    }
    gaps
}

#[test]
fn propose_archives_design_only_changes() {
    // req:plugin/propose-archives-design-only@1cb1a00e
    let body = skill_body("propose");
    assert_eq!(design_only_gaps(&body), Vec::<String>::new());
    // skill without archiving
    let gaps = design_only_gaps(&body.replace("clm archive", "elided"));
    assert_reports(&gaps, "propose", "clm archive");
    // skill resolving `specs/` by hand on an archived change
    let said = body.replacen(
        "(`archive-in-spec-pr`).",
        "(`archive-in-spec-pr`). Resolve a conflict under `specs/` by hand.",
        1,
    );
    assert_ne!(said, body);
    let gaps = design_only_gaps(&said);
    assert_reports(&gaps, "propose", "by hand");
    // A change with no requirement added, modified or removed is archived in the spec
    // branch, after the builder check and as the last commit before the page.
    let archive = body
        .find("clm archive <id>")
        .expect("propose: `clm archive <id>`");
    let check = body.find("Run a builder check").unwrap();
    let page = body.find("clm pr open spec/<id>").unwrap();
    assert!(
        check < archive && archive < page,
        "propose: archive between check and page"
    );
    let when = sentence_with(&body, "archive-in-spec-pr");
    for needle in ["add, modify and remove no requirement", "nothing to build"] {
        assert!(when.contains(needle), "{needle}: {when}");
    }
    let how = sentence_with(&body, "clm archive <id>");
    for needle in [
        "last commit before opening the pull request",
        "status: done",
        "`resolves`",
        "clm check",
    ] {
        assert!(how.contains(needle), "{needle}: {how}");
    }
}

#[test]
fn propose_sets_priority() {
    // req:plugin/propose-priority@b217daba
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    for needle in [
        "`priority`",
        "check/priority-caps",
        "highest priority first",
        "clm backlog",
        "`P5`",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    // The human picks which holder to demote.
    let sentence = sentence_with(&body, "check/priority-caps");
    assert!(
        sentence.contains("ask") && sentence.contains("demote"),
        "propose: on check/priority-caps, ask the human which holder to demote: {sentence}"
    );
    assert!(
        body.contains("never choosing one yourself") || body.contains("never choose one yourself"),
        "propose: SKILL.md must leave the demotion to the human"
    );
    // The change's priority comes from its items, and P5 all round means none.
    let sentence = sentence_with(&body, "highest `priority`");
    for needle in ["`from`", "`resolves`", "`P5`"] {
        assert!(sentence.contains(needle), "{sentence}");
    }
    // Suggestions follow `clm backlog`'s order.
    let sentence = sentence_with(&body, "highest priority first");
    assert!(sentence.contains("clm backlog"), "{sentence}");
    // propose suggests only ideas, and points at triage for the rest.
    assert_eq!(propose_triage_gaps(&body), Vec::<String>::new());
    let sentence = sentence_with(&body, "/clm:triage");
    for needle in [
        "only the remaining `idea` items",
        "`bug`, `question`, `human-task`, `deferred` and `workaround`",
        "no `idea` item is left",
        "triage-owns-backlog-work",
        "of any kind the user names is still taken as the intent",
    ] {
        assert!(sentence.contains(needle), "{needle}: {sentence}");
    }
    // propose without the triage pointer
    assert_reports(
        &propose_triage_gaps(&body.replace("/clm:triage", "")),
        "propose",
        "/clm:triage",
    );
}

/// What `propose-priority` asks of the `propose` skill about ideas and triage.
fn propose_triage_gaps(body: &str) -> Vec<String> {
    text_gaps("propose", body, &["idea", "/clm:triage"], &[])
}

#[test]
fn propose_elaborates_ideas() {
    // req:plugin/propose-elaborates-ideas@839ccda3
    let full = skill_body("propose");
    // The rules sit in a section of their own, outside the numbered steps.
    let section = full
        .split("\n## ")
        .find(|s| s.starts_with("Elaborating an idea"))
        .expect("propose: a section on elaborating an idea");
    let body = section.to_owned();
    assert_eq!(elaboration_gaps(&body), Vec::<String>::new());
    for needle in [
        "when the claim is refused",
        "`title`, `kind: idea` and `components`",
        "no `change.md` and no delta",
        "never landing at every pause",
        "`rebase ok`",
        "`code`",
        "afresh from `origin/main`",
        "once the claim has lapsed",
    ] {
        assert!(
            body.contains(needle),
            "propose: the section must mention {needle}"
        );
    }
    // skill without elaboration
    assert_reports(
        &elaboration_gaps(&body.replace("## Settled", "")),
        "propose",
        "## Settled",
    );
    for needle in [
        "git ls-remote origin 'refs/heads/spec/*-idea-<slug>'",
        "spec/<YYYYMMDD>-idea-<slug>",
        "## Intent",
        "## Approaches",
        "propose/<slug>",
        "autosync/spec/<YYYYMMDD>-idea-<slug>",
        "clm branch delete",
        "elaborate-ideas-on-main",
        "from: backlog/<slug>",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
}

/// What `propose-elaborates-ideas` asks of the `propose` skill.
fn elaboration_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &["## Settled", "## Open questions", "-idea-", "clm wrap-up"],
        &[],
    )
}

#[test]
fn propose_nests_splits() {
    // req:plugin/propose-nests-splits@79979cdd
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    for needle in [
        "specs/<component>/<part>/SPEC.md",
        "`<component>.<part>`",
        "`slim-components`",
        "`nested-splits`",
        "clm guide shaping-the-graph",
        "clm survey --components",
        "`splits-import-one-way`",
        "outside: [\"**\"]",
    ] {
        assert!(
            body.contains(needle),
            "propose: SKILL.md must mention {needle}"
        );
    }
    assert!(
        !body.replace(GLOSSARY_PATH, "").contains("docs/guide/"),
        "propose: SKILL.md must not name docs/guide/"
    );
    // The carved-out part becomes a child of the component it came from.
    let sentence = sentence_with(&body, "specs/<component>/<part>/SPEC.md");
    assert!(
        sentence.contains("make the carved-out part a child of the component it came from"),
        "propose: a carved-out part becomes a child of its node: {sentence}"
    );
    // The product's ID stays on the half that stays.
    let sentence = sentence_with(&body, "product's ID");
    assert!(
        sentence.contains("Keep the product's ID on the half that stays"),
        "{sentence}"
    );
    // Requirements move unchanged: removed from the parent, added to the child.
    let sentence = sentence_with(&body, "Move each requirement unchanged");
    let removed = sentence.find("REMOVED from the parent").expect(sentence);
    let added = sentence.find("ADDED").expect(sentence);
    let child = sentence.find("to the child").expect(sentence);
    assert!(removed < added && added < child, "{sentence}");
    // Seams only, never a size budget.
    let sentence = sentence_with(&body, "architectural seams");
    assert!(
        sentence.contains("never") && sentence.contains("budget"),
        "propose: never split for a budget alone: {sentence}"
    );
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
    // req:plugin/build-watches@67db2340
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
    // req:plugin/build-says-what-is-ready@d0ad7ac6
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
    // req:plugin/build-unclosed-builds@5ceba149
    let body = skill_body("build");
    assert_eq!(unclosed_gaps(&body), Vec::<String>::new());
    // a build skill that counts every unclosed build as a stop
    let bare = body.replace("landed impl/<component>", "landed");
    assert_reports(&unclosed_gaps(&bare), "build", "landed impl/<component>");
    let finish = sentence_with(&body, "landed impl/<component>");
    assert!(finish.contains("clm run --finish <component>"), "{finish}");
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

/// What `build-resumes-limited-builds` asks of a `build` skill.
fn resume_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "build",
        body,
        &[
            "usage or rate limit",
            "SendMessage",
            "resume-limited-builds",
        ],
        &[],
    )
}

#[test]
fn build_resumes_limited_builds() {
    // req:plugin/build-resumes-limited-builds@7dbd065a
    let body = skill_body("build");
    assert_eq!(resume_gaps(&body), Vec::<String>::new());
    // build without the resume
    let gaps = resume_gaps(&body.replace("SendMessage", "elided"));
    assert_reports(&gaps, "build", "SendMessage");
    let resume = sentence_with(&body, "resume that same subagent");
    for needle in ["SendMessage", "deferred", "carry on"] {
        assert!(resume.contains(needle), "{needle}: {resume}");
    }
    let limited = sentence_with(&body, "hasn't stopped");
    let rule = &body[body.find(limited).unwrap()..];
    let rule = &rule[..rule.find("A subagent that ends without").unwrap()];
    for needle in ["closing line", "don't run `clm run --finish <component>`"] {
        assert!(rule.contains(needle), "{needle}: {limited}");
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
    // req:plugin/build-waiting-builds@b49b8327
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
    // req:plugin/build-stops-landed-builds@ee9902e8
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
    // It sits right after the waiting rule and before the limit rule.
    let at = |needle: &str| body.find(needle).unwrap_or_else(|| panic!("{needle}"));
    assert!(at("whatever its last line says") < at("leftover task"));
    assert!(at("leftover task") < at("hasn't stopped"));
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
    // req:plugin/triage-waiting-drafts@31624f18
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
    // req:plugin/triage-enters-branch-worktree@fcd0a66e
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
    assert!(body.contains("There, rebase, resolve, commit, push and land."));
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
    // req:plugin/triage-checks-splits@e29c4008
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

/// The sentences of `text` that tell Claude to push other than with `clm branch push`: a
/// raw push is refused once `clm rebase` has rebased the branch onto `origin/main`.
fn plain_pushes(text: &str) -> Vec<&str> {
    text.lines()
        .flat_map(|l| l.split(". "))
        .filter(|s| s.split(|c: char| !c.is_alphanumeric()).any(|w| w == "push"))
        .filter(|s| !s.contains("clm branch push") && !s.contains("ever push"))
        .collect()
}

#[test]
fn propose_pushes_with_lease_after_rebase() {
    // req:plugin/skills@b943c3b9
    // The skills push with `clm branch push` (`skills-push-with-clm`)
    // after `clm rebase`. Steps 10 and 12 and the code-fix path push after a rebase.
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    let steps = numbered_steps(&body);
    let fixes = &body[body
        .find("## Code fixes")
        .expect("a `## Code fixes` section")..];
    for (name, text) in [
        ("step 10", steps[9].as_str()),
        ("step 12", steps[11].as_str()),
        ("## Code fixes", fixes),
    ] {
        assert!(text.contains("clm") && !text.is_empty(), "{name}");
        assert_eq!(
            plain_pushes(text),
            Vec::<&str>::new(),
            "propose: {name} must push with `clm branch push` after `clm rebase`"
        );
    }
    // A plain push is caught; a `clm branch push` and the rule against pushing `main` are not.
    assert_eq!(
        plain_pushes("Write the body and push, then open the page."),
        ["Write the body and push, then open the page."]
    );
    assert!(plain_pushes("Push with `clm branch push`, then land.").is_empty());
    assert!(plain_pushes("Never push `main` by any means but `clm land`.").is_empty());
}

/// What a `propose` skill's code-fix path leaves out (`code-fix-resolves-bug`), one line
/// per missing piece naming it. Empty when the path is all there.
fn code_fix_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &[
            "Resolves: backlog/",
            "clm claim <component> --fix",
            "fixes-count-where-cited",
        ],
        &[],
    )
}

#[test]
fn propose_offers_code_fixes() {
    // req:plugin/propose-code-fixes@528d2c04
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    assert_eq!(code_fix_gaps(&body), Vec::<String>::new());
    // A test file lies in the nodes its markers cite too, so its fix claims the cited node.
    let cited = sentence_with(&body, "fixes-count-where-cited");
    for needle in [
        "`req:` markers",
        "`paths`",
        "claims the component the test cites",
    ] {
        assert!(cited.contains(needle), "{needle}: {cited}");
    }
    // Each scenario: a skill without the piece fails, naming it.
    for needle in [
        "Resolves: backlog/",
        "clm claim <component> --fix",
        "fixes-count-where-cited",
    ] {
        let gaps = code_fix_gaps(&body.replace(needle, "elided"));
        assert!(
            gaps.len() == 1 && gaps[0].contains(needle),
            "{needle}: {gaps:?}"
        );
    }
    // Brainstorming is where the offer is made: a code-only bug gets a code branch, on a yes.
    let steps = numbered_steps(&body);
    let brainstorm = steps
        .iter()
        .find(|s| s.contains("clm superpowers brainstorming"))
        .expect("propose: SKILL.md has a brainstorming step");
    let offer = sentence_with(brainstorm, "code branch");
    for needle in [
        "`kind: bug`",
        "code branch instead of a spec branch",
        "on a no",
        "changes nothing under `specs/`, `changes/`",
        "`components`",
        "offer",
        "Code fixes",
    ] {
        assert!(offer.contains(needle), "{needle}: {offer}");
    }
    // The path itself, in order: drop the spec branch, claim the node for the fix, fix it
    // test first, delete the item with the trailer, gate `code`, open the page and land.
    let path = code_fix_section(&body);
    let order = [
        "`spec/<id>`",
        "`autosync/spec/<id>`",
        "clm claim <component> --fix backlog/<slug>",
        "test first",
        "`backlog/<slug>.md`",
        "`Co-Authored-By`",
        "`Resolves: backlog/<slug>`",
        "`clm gate --base origin/main`",
        "clm branch push",
        "clm pr open",
        "clm land",
    ];
    let mut at = 0;
    for needle in order {
        let found = path[at..]
            .find(needle)
            .unwrap_or_else(|| panic!("propose: `## Code fixes` must name {needle} after {at}"));
        at += found + needle.len();
    }
    let drop = sentence_with(path, "`spec/<id>`");
    for needle in ["delete", "`origin`", "`autosync/spec/<id>`"] {
        assert!(drop.contains(needle), "{needle}: {drop}");
    }
    let gate = sentence_with(path, "clm gate --base origin/main");
    assert!(gate.contains("`code`"), "{gate}");
    // A fix the path can't carry stays a spec branch.
    let fallback = sentence_with(path, "stays a spec branch");
    for needle in [
        "more than one component",
        "`components`",
        "spec change",
        "refused",
    ] {
        assert!(fallback.contains(needle), "{needle}: {fallback}");
    }
}

/// The `## Code fixes` section of a `propose` skill, up to the next `## ` heading.
fn code_fix_section(body: &str) -> &str {
    let path = &body[body
        .find("## Code fixes")
        .expect("a `## Code fixes` section")..];
    &path[..path[3..].find("\n## ").map_or(path.len(), |end| end + 3)]
}

/// What `code-fix-needs-a-written-rule` asks of a `propose` skill's code-fix section, one
/// line per missing piece naming the skill and the text. Empty when it says it all.
fn written_rule_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        code_fix_section(body),
        &["code-fix-needs-a-written-rule", "as written"],
        &[],
    )
}

#[test]
fn propose_code_fixes_need_a_written_rule() {
    // req:plugin/code-fix-needs-a-written-rule@35aca908
    let (_, body) = frontmatter(&plugin().join("skills/propose/SKILL.md"));
    assert_eq!(written_rule_gaps(&body), Vec::<String>::new());
    let section = code_fix_section(&body);
    // Only a behaviour a requirement already states takes the code-branch route.
    let only = sentence_with(section, "as written");
    for needle in [
        "only when",
        "requirement or scenario",
        "cites that requirement",
    ] {
        assert!(only.contains(needle), "{needle}: {only}");
    }
    // Otherwise a spec branch writes the rule first, and the fix follows citing it.
    let otherwise = sentence_with(section, "code-fix-needs-a-written-rule");
    for needle in [
        "spec branch",
        "adds the requirement or scenario",
        "`impl/<component>`",
    ] {
        assert!(otherwise.contains(needle), "{needle}: {otherwise}");
    }
    // skill without the rule: each missing piece is reported, naming the skill.
    for needle in ["code-fix-needs-a-written-rule", "as written"] {
        let gaps = written_rule_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
}

#[path = "../../clm-git/tests/exec/mod.rs"]
mod exec;
use exec::write_exec as executable;

/// The release targets, with the `uname -s` and `uname -m` that map to each.
const TARGETS: [(&str, &str, &str); 4] = [
    ("Linux", "x86_64", "x86_64-unknown-linux-musl"),
    ("Linux", "aarch64", "aarch64-unknown-linux-musl"),
    ("Darwin", "x86_64", "x86_64-apple-darwin"),
    ("Darwin", "arm64", "aarch64-apple-darwin"),
];

/// Each release target once.
fn release_targets() -> std::collections::BTreeSet<&'static str> {
    TARGETS.iter().map(|(_, _, target)| *target).collect()
}

/// A copy of the shim in a plugin dir of version 9.9.9, a fake `curl` serving the assets in
/// `assets/`, a `gh` that logs its calls, and an empty cache.
struct Shim {
    dir: TempDir,
}

impl Shim {
    fn new() -> Shim {
        let dir = TempDir::new().unwrap();
        let d = dir.path();
        for sub in [
            "plugin/bin",
            "plugin/.claude-plugin",
            "fakebin",
            "assets",
            "cache",
        ] {
            fs::create_dir_all(d.join(sub)).unwrap();
        }
        executable(
            &d.join("plugin/bin/clm"),
            &fs::read_to_string(plugin().join("bin/clm")).unwrap(),
        );
        fs::write(
            d.join("plugin/.claude-plugin/plugin.json"),
            "{\n  \"name\": \"clm\",\n  \"version\": \"9.9.9\"\n}\n",
        )
        .unwrap();
        // `curl -fsSL -o <file> <url>`: logs its arguments, copies the asset the url names, and
        // fails like `-f` does when there is none.
        executable(
            &d.join("fakebin/curl"),
            "#!/bin/sh\necho \"$*\" >> \"$SHIM_TEST/curl.log\"\n\
             while [ $# -gt 1 ]; do [ \"$1\" = -o ] && out=$2; shift; done\n\
             cp \"$SHIM_TEST/assets/$(basename \"$1\")\" \"$out\" 2>/dev/null || exit 22\n",
        );
        // The shim has no use for `gh`: any call is logged, and fails.
        executable(
            &d.join("fakebin/gh"),
            "#!/bin/sh\necho \"$*\" >> \"$SHIM_TEST/gh.log\"\nexit 1\n",
        );
        Shim { dir }
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.dir.path().join(rel)
    }

    /// Serves a release whose `clm` records its arguments, with
    /// checksums that match unless `corrupt`.
    fn serve(&self, corrupt: bool) {
        let stage = self.path("stage");
        fs::create_dir_all(&stage).unwrap();
        executable(
            &stage.join("clm"),
            "#!/bin/sh\necho \"cached clm $*\" >> \"$SHIM_TEST/runs.log\"\n",
        );
        for target in release_targets() {
            let asset = format!("clm-{target}.tar.gz");
            let ok = Command::new("tar")
                .args(["czf", &format!("../assets/{asset}"), "clm"])
                .current_dir(&stage)
                .status()
                .unwrap();
            assert!(ok.success());
            let sum = Command::new("sh")
                .args(["-c", &format!("sha256sum {asset} || shasum -a 256 {asset}")])
                .current_dir(self.path("assets"))
                .output()
                .unwrap();
            let mut sum = String::from_utf8(sum.stdout).unwrap();
            if corrupt {
                sum = format!("{}{}", "0".repeat(64), &sum[64..]);
            }
            fs::write(self.path(&format!("assets/{asset}.sha256")), sum).unwrap();
        }
    }

    /// Installs a fake `uname` that reports the platform `system` `machine`.
    fn platform(&self, system: &str, machine: &str) {
        executable(
            &self.path("fakebin/uname"),
            &format!("#!/bin/sh\n[ \"$1\" = -s ] && echo '{system}' || echo '{machine}'\n"),
        );
    }

    fn run(&self, args: &[&str], env: &[(&str, &str)]) -> Output {
        let path = format!(
            "{}:{}",
            self.path("fakebin").display(),
            std::env::var("PATH").unwrap()
        );
        Command::new(self.path("plugin/bin/clm"))
            .args(args)
            .env("PATH", path)
            .env("SHIM_TEST", self.dir.path())
            .env("XDG_CACHE_HOME", self.path("cache"))
            .env_remove("CLM_BIN")
            .envs(env.iter().copied())
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.path(rel)).unwrap_or_default()
    }

    /// How many release assets `curl` was asked for.
    fn downloads(&self) -> usize {
        self.read("curl.log")
            .lines()
            .filter(|l| l.ends_with(".tar.gz"))
            .count()
    }

    fn cached(&self) -> bool {
        self.path("cache/clm/9.9.9/clm").exists()
    }
}

#[test]
fn shim_first_use_downloads_once() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    shim.serve(false);
    for _ in 0..2 {
        let out = shim.run(&["status"], &[]);
        assert!(out.status.success(), "{out:?}");
    }
    let curl = shim.read("curl.log");
    let url = "https://github.com/clunny/clm/releases/download/v9.9.9/clm-";
    let asked = |suffix: &str| {
        curl.lines()
            .filter(|l| l.ends_with(&format!(".tar.gz{suffix}")) && l.contains(url))
            .count()
    };
    assert_eq!((asked(""), asked(".sha256")), (1, 1), "{curl}");
    assert_eq!(shim.downloads(), 1, "{curl}");
    assert_eq!(shim.read("gh.log"), "", "the shim needs no gh");
    assert_eq!(
        shim.read("runs.log"),
        "cached clm status\ncached clm status\n"
    );
    assert!(shim.cached());
    let leftovers: Vec<_> = fs::read_dir(shim.path("cache/clm/9.9.9"))
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(leftovers, ["clm"], "the download's temp dir is cleaned up");
}

#[test]
fn shim_bad_checksum_caches_nothing() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    shim.serve(true);
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(!shim.cached());
    assert_eq!(shim.read("runs.log"), "");
}

#[test]
fn shim_without_a_release_says_how_to_install() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    fs::remove_dir(shim.path("assets")).unwrap();
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1));
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(
        err.contains("cargo install") && err.contains("v9.9.9"),
        "{err}"
    );
    assert!(!shim.cached());
    // Hooks stay silent instead, so a missing binary never breaks a session, and skip the
    // download for a while rather than retry it on every tool call.
    for _ in 0..2 {
        let out = shim.run(&["hook", "guard"], &[]);
        assert_eq!(
            (out.status.code(), out.stdout.len(), out.stderr.len()),
            (Some(0), 0, 0),
            "{out:?}"
        );
    }
    assert_eq!(shim.downloads(), 2, "status, then one hook");
    shim.run(&["status"], &[]);
    assert_eq!(shim.downloads(), 3, "commands always retry");
    assert!(!shim.cached());
}

/// Runs `hook session-start` on a shim that can't get a binary, and checks it exits 0
/// having cached nothing, with one line saying the guard is off and how to install `tag`.
fn session_start_says_the_guard_is_off(shim: &Shim, tag: &str) {
    let out = shim.run(&["hook", "session-start"], &[]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(!shim.cached());
    let said = String::from_utf8(out.stdout).unwrap();
    let install = format!("cargo install --git https://github.com/clunny/clm --tag {tag} clm");
    for words in [
        "CLM state",
        "guard",
        "`clm approve`",
        "unavailable",
        &install,
    ] {
        assert!(said.contains(words), "{words} missing from {said:?}");
    }
    for retired in ["clm review", "spec-approved"] {
        assert!(!said.contains(retired), "{retired}: {said:?}");
    }
    assert_eq!(said.lines().count(), 1, "{said:?}");
}

#[test]
fn shim_session_start_without_a_binary_says_the_guard_is_off() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    fs::remove_dir(shim.path("assets")).unwrap();
    session_start_says_the_guard_is_off(&shim, "v9.9.9");
    assert_eq!(shim.downloads(), 1);
    // The second run skips the download (`.unavailable`) and still says so.
    session_start_says_the_guard_is_off(&shim, "v9.9.9");
    assert_eq!(shim.downloads(), 1);
}

#[test]
fn shim_without_a_version_fails() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    shim.serve(false);
    fs::write(shim.path("plugin/.claude-plugin/plugin.json"), "{}\n").unwrap();
    let out = shim.run(&["status"], &[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("version"));
    // Hooks still never fail, and session start still says the guard is off.
    let out = shim.run(&["hook", "guard"], &[]);
    assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
    session_start_says_the_guard_is_off(&shim, "v<version>");
    assert_eq!(shim.read("curl.log"), "");
}

#[test]
fn shim_downloads_the_target_for_each_platform() {
    // req:plugin/shim-targets@7df28173
    for (system, machine, target) in TARGETS {
        let shim = Shim::new();
        shim.serve(false);
        shim.platform(system, machine);
        for _ in 0..2 {
            let out = shim.run(&["status"], &[]);
            assert!(out.status.success(), "{system} {machine}: {out:?}");
        }
        let curl = shim.read("curl.log");
        assert!(
            curl.lines()
                .any(|l| l.ends_with(&format!("/clm-{target}.tar.gz"))),
            "{system} {machine}: {curl}"
        );
        assert_eq!(shim.downloads(), 1, "{system} {machine}: {curl}");
        assert!(shim.cached(), "{system} {machine}: the binary isn't cached");
        assert_eq!(
            shim.read("runs.log"),
            "cached clm status\ncached clm status\n",
            "{system} {machine}"
        );
    }
}

#[test]
fn shim_maps_each_target_in_one_assignment() {
    // req:plugin/shim-targets@7df28173
    // `ci/release`'s prepare step reads the shim's `target=<target>` assignments: exactly
    // the four targets, and no others.
    let shim = fs::read_to_string(plugin().join("bin/clm")).unwrap();
    for target in release_targets() {
        assert_eq!(
            shim.matches(&format!("target={target}")).count(),
            1,
            "target={target}"
        );
    }
    let assignments = shim
        .lines()
        .filter(|l| l.trim_start().contains(") target=") && !l.contains("target= "))
        .count();
    assert_eq!(assignments, release_targets().len(), "{shim}");
}

/// What `shim-targets` forbids the shim's text to name: a Windows target or `clm.exe`.
fn windows_words(shim: &str) -> Vec<&'static str> {
    let lower = shim.to_lowercase();
    ["windows", "clm.exe"]
        .into_iter()
        .filter(|word| lower.contains(word))
        .collect()
}

#[test]
fn shim_names_no_windows_target() {
    // req:plugin/shim-targets@7df28173
    let shim = fs::read_to_string(plugin().join("bin/clm")).unwrap();
    assert_eq!(windows_words(&shim), Vec::<&str>::new());
    // The text fails the check when a comment names Windows in any case, or `clm.exe`.
    assert_eq!(windows_words("# runs on WINDOWS\n"), ["windows"]);
    assert_eq!(windows_words("exe=clm.exe\n"), ["clm.exe"]);
}

#[test]
fn shim_on_an_unsupported_platform_fails() {
    // req:plugin/binary-shim@38609dbc
    // req:plugin/shim-targets@7df28173
    // Windows's shells (MSYS2, Cygwin and the rest) get no binary either (`no-windows-support`).
    for (system, machine) in [
        ("FreeBSD", "amd64"),
        ("CYGWIN_NT-10.0", "x86_64"),
        ("MINGW64_NT-10.0", "x86_64"),
        ("UCRT64_NT-10.0-22631", "x86_64"),
    ] {
        let shim = Shim::new();
        shim.serve(false);
        shim.platform(system, machine);
        let out = shim.run(&["status"], &[]);
        assert_eq!(out.status.code(), Some(1), "{system}");
        let err = String::from_utf8_lossy(&out.stderr);
        let platform = format!("{system} {machine}");
        for words in ["no prebuilt clm", &platform, "cargo install"] {
            assert!(err.contains(words), "{words} missing from {err}");
        }
        assert_eq!(shim.read("curl.log"), "");
        let out = shim.run(&["hook", "guard"], &[]);
        assert_eq!((out.status.code(), out.stdout.len()), (Some(0), 0));
        session_start_says_the_guard_is_off(&shim, "v9.9.9");
        assert_eq!(shim.read("curl.log"), "");
        assert!(!shim.cached());
    }
}

#[test]
fn shim_prefers_clm_bin() {
    // req:plugin/binary-shim@38609dbc
    let shim = Shim::new();
    let stub = shim.path("own-clm");
    executable(
        &stub,
        "#!/bin/sh\necho \"own $*\" >> \"$SHIM_TEST/runs.log\"\n",
    );
    let out = shim.run(
        &["next", "--expiry", "2"],
        &[("CLM_BIN", stub.to_str().unwrap())],
    );
    assert!(out.status.success(), "{out:?}");
    assert_eq!(shim.read("runs.log"), "own next --expiry 2\n");
    assert_eq!(shim.read("curl.log"), "");
}

#[test]
fn view_command_falls_back_without_claiming_absence() {
    // req:plugin/view-fallback-wording@2ed0320a
    let text = fs::read_to_string(plugin().join("commands/view.md")).unwrap();
    // installed but refused: the text can't tell which, so it names both cases.
    assert!(!text.contains("isn't installed here"), "{text}");
    for needle in [
        "isn't installed or didn't load",
        "/plugin install clm-panel@clm",
        "claude plugin validate",
        "claude --debug",
        "clm status",
        "clm view $ARGUMENTS",
    ] {
        assert!(text.contains(needle), "{needle}: {text}");
    }
}

/// What `skills-write-serves` asks of the propose and init skills.
fn serves_gaps(skill: &str, body: &str) -> Vec<String> {
    text_gaps(
        skill,
        body,
        &[
            "`serves`",
            "#### Serves",
            "check/untraced",
            "holds an entry",
        ],
        &[],
    )
}

/// What `skills-write-enforcement` asks the propose and init skills' paragraph on the human
/// layers' proof to say.
const ENFORCEMENT: [&str; 11] = [
    "`#### Enforced by`",
    "`#### Untested`",
    "whole-tree check",
    "choke point",
    "the one path plus each blocked bypass",
    "or both",
    "never cites an architectural constraint",
    "check/unenforced",
    "the component `capability`",
    "stubbed",
    "human-layer-proof",
];

/// What `skills-write-enforcement` asks of the propose and init skills: the paragraph that
/// says how a capability is proven also says how an architecture entry is.
fn enforcement_gaps(skill: &str, body: &str) -> Vec<String> {
    let paragraph = body
        .lines()
        .find(|l| l.contains("A capability is proven by"))
        .unwrap_or_default();
    text_gaps(skill, paragraph, &ENFORCEMENT, &[])
}

#[test]
fn skills_write_enforcement() {
    // req:plugin/skills-write-enforcement@001e455c
    for skill in ["propose", "init"] {
        let body = skill_body(skill);
        assert_eq!(enforcement_gaps(skill, &body), Vec::<String>::new());
        // skill without the rule
        for needle in ENFORCEMENT {
            let gaps = enforcement_gaps(skill, &body.replace(needle, "elided"));
            assert_reports(&gaps, skill, needle);
        }
        // The capability test sits at the public boundary, deterministic.
        let proof = sentence_with(&body, "A capability is proven by");
        for needle in [
            "public boundary",
            "deterministic",
            "the component `capability`",
        ] {
            assert!(proof.contains(needle), "{skill}: {needle}: {proof}");
        }
    }
}

#[test]
fn skills_write_serves() {
    // req:plugin/skills-write-serves@44502341
    for skill in ["propose", "init"] {
        let body = skill_body(skill);
        assert_eq!(serves_gaps(skill, &body), Vec::<String>::new());
        // skill without the rule
        for needle in [
            "`serves`",
            "#### Serves",
            "check/untraced",
            "holds an entry",
        ] {
            let gaps = serves_gaps(skill, &body.replace(needle, "elided"));
            assert_reports(&gaps, skill, needle);
        }
    }
}

/// The model every launch names (`launch-models`), and the session's own model in words.
const SONNET: &str = "model: \"sonnet\"";
const OWN_MODEL: &str = "the session's own model";

/// What `launch-models` asks of a skill's text, one line per gap naming the skill and the
/// text it lacks. Empty when it says it all.
fn launch_model_gaps(skill: &str, body: &str) -> Vec<String> {
    let must: &[&str] = match skill {
        "build" => &["--review-model", SONNET, OWN_MODEL],
        "triage" | "propose" => &[SONNET, OWN_MODEL],
        "contribution" => &[OWN_MODEL],
        _ => &[SONNET],
    };
    text_gaps(skill, body, must, &[])
}

#[test]
fn launch_models() {
    // req:plugin/launch-models@f7550b3b
    for skill in ["build", "triage", "propose", "health", "init"] {
        let body = skill_body(skill);
        assert_eq!(launch_model_gaps(skill, &body), Vec::<String>::new());
    }
    let contribution = skill_body("contribution");
    assert_eq!(
        launch_model_gaps("contribution", &contribution),
        Vec::<String>::new()
    );
    let gaps = launch_model_gaps("contribution", &contribution.replace(OWN_MODEL, "elided"));
    assert_reports(&gaps, "contribution", OWN_MODEL);
    // Each skill's `architecture-auditor` launch names the session's own model.
    for skill in ["propose", "contribution"] {
        let text = skill_body(skill);
        let launch = sentence_with(&text, "architecture-auditor");
        assert!(launch.contains(OWN_MODEL), "{skill}: {launch}");
    }
    for agent in ["spec-auditor", "architecture-auditor", "spec-slimmer"] {
        let (fm, _) = frontmatter(&plugin().join(format!("agents/{agent}.md")));
        assert_eq!(
            fm["model"], "sonnet",
            "{agent}: frontmatter must set model: sonnet"
        );
    }
    // build launched without a model
    let body = skill_body("build");
    let gaps = launch_model_gaps("build", &body.replace(SONNET, "elided"));
    assert_reports(&gaps, "build", SONNET);
    // A retry after a stop that no backlog item or spec change explains runs on the
    // session's own model.
    let retry = sentence_with(&body, "stopped without landing");
    assert!(retry.contains(OWN_MODEL), "{retry}");
    // Each launch's own sentence names its model, not just some line of the file.
    let first = sentence_with(&body, "isolation: \"worktree\"");
    assert!(
        first.contains("--review-model") && first.contains(SONNET),
        "{first}"
    );
    for (skill, launch) in [
        ("propose", "Launch a fresh subagent"),
        ("health", "launch the `spec-slimmer` subagent"),
        ("init", "fill each component with its own subagent"),
    ] {
        let text = skill_body(skill);
        let sentence = sentence_with(&text, launch);
        assert!(sentence.contains(SONNET), "{skill}: {sentence}");
    }
    // Drafts run on the session's own model, the fold on a named one.
    let triage = skill_body("triage");
    assert!(sentence_with(&triage, "clm draft <slug>...").contains(OWN_MODEL));
    assert!(sentence_with(&triage, "clm draft --fold <id>...").contains(SONNET));
}

#[test]
fn skills_say_how_a_recommendation_is_chosen() {
    // req:plugin/recommend-the-right-design@df33f4b2
    for skill in ["triage", "propose"] {
        let (_, body) = frontmatter(&plugin().join(format!("skills/{skill}/SKILL.md")));
        for needle in ["recommend-the-right-design", "one option", "maintainable"] {
            let gaps = text_gaps(skill, &body, &[needle], &[]);
            assert!(gaps.is_empty(), "{skill}: {gaps:?}");
            let gaps = text_gaps(skill, &body.replace(needle, "elided"), &[needle], &[]);
            assert_reports(&gaps, skill, needle);
        }
        let line = body
            .lines()
            .find(|l| l.contains("`recommend-the-right-design`"))
            .unwrap_or_else(|| panic!("{skill}: a paragraph must name the rule"));
        for needle in [
            "clm context <component>",
            "one option",
            "maintainable",
            "not after",
            "only with the reason it is also the right one",
        ] {
            assert!(line.contains(needle), "{skill}: {needle}: {line}");
        }
    }
}

/// The old names `clm/renamed-commands` keeps as aliases, as the subcommand after `clm`.
const OLD_SUBCOMMANDS: [&str; 3] = ["sync", "checkpoint", "page"];
/// The old outcome-line starts: how a line `clm sync` or `clm checkpoint` ended with began.
const OLD_OUTCOMES: [&str; 6] = [
    "sync ok",
    "sync incomplete",
    "sync failed",
    "checkpoint ok",
    "checkpoint incomplete",
    "checkpoint failed",
];

/// Whether `needle` occurs in `line` as whole words: not inside a longer word.
fn has_phrase(line: &str, needle: &str) -> bool {
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
    line.match_indices(needle).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + needle.len()..].chars().next();
        !before.is_some_and(word) && !after.is_some_and(word)
    })
}

/// One line per old name in `files` (path and text): `clm` and an old subcommand, an old
/// outcome line, or a pull request called a `page` (a guide page is fine), naming the file, its line and the name.
fn old_name_gaps(files: &[(String, String)]) -> Vec<String> {
    let mut gaps = Vec::new();
    for (path, text) in files {
        for (i, line) in text.lines().enumerate() {
            let mut old: Vec<String> = OLD_SUBCOMMANDS
                .iter()
                .map(|c| format!("clm {c}"))
                .filter(|n| has_phrase(line, n))
                .collect();
            old.extend(
                OLD_OUTCOMES
                    .iter()
                    .filter(|n| has_phrase(line, n))
                    .map(|n| (*n).to_owned()),
            );
            if has_phrase(line, "page") && !line.contains("guide") {
                old.push("page".to_owned());
            }
            for name in old {
                gaps.push(format!("{path}:{}: names the old `{name}`", i + 1));
            }
        }
    }
    gaps
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

#[test]
fn plugin_names_the_renamed_commands() {
    // req:plugin/renamed-commands@bbfd9d22
    // A skill that tells Claude to run the old name before the session ends is reported with
    // its file, its line and the name.
    let stale = [(
        "plugin/skills/build/SKILL.md".to_owned(),
        "1. Build.\n2. Run `clm checkpoint` before the session ends.\n".to_owned(),
    )];
    assert_eq!(
        old_name_gaps(&stale),
        ["plugin/skills/build/SKILL.md:2: names the old `clm checkpoint`"]
    );
    let outcome = [("x".to_owned(), "Its last line is `sync ok`.\n".to_owned())];
    assert_eq!(old_name_gaps(&outcome), ["x:1: names the old `sync ok`"]);
    let body = [(
        "x".to_owned(),
        "Write the page body to a file.\n".to_owned(),
    )];
    assert_eq!(old_name_gaps(&body), ["x:1: names the old `page`"]);
    // The new names, and words that merely contain an old one, are not reported.
    let fine = [(
        "x".to_owned(),
        "Run `clm rebase`, then `clm wrap-up`; open it with `clm pr open`. `autosync` pushes.\n"
            .to_owned(),
    )];
    assert!(old_name_gaps(&fine).is_empty());
    // The shipped plugin names none.
    let mut files = Vec::new();
    text_files_under(&plugin(), &mut files);
    assert!(files.len() > 8, "{} files read", files.len());
    let gaps = old_name_gaps(&files);
    assert!(gaps.is_empty(), "{gaps:#?}");
}

/// One line per use of the old term for an entry of `ARCHITECTURE.md` in `files` (path and
/// text), or of an instruction to add a `### Requirement:` entry to it, naming the file,
/// its line and the term.
/// The heading of a node's requirement, which `ARCHITECTURE.md` no longer uses for an entry.
const OLD_ENTRY: &str = "### Requirement:";

fn old_term_gaps(files: &[(String, String)]) -> Vec<String> {
    let mut gaps = Vec::new();
    for (path, text) in files {
        for (i, line) in text.lines().enumerate() {
            let lower = line
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_lowercase();
            let mut old = Vec::new();
            for term in ["architecture requirement", "architecture requirements"] {
                if has_phrase(&lower, term) {
                    old.push(term);
                }
            }
            if line.contains("ARCHITECTURE.md") && line.contains(OLD_ENTRY) {
                old.push(OLD_ENTRY);
            }
            for term in old {
                gaps.push(format!("{path}:{}: holds the old term `{term}`", i + 1));
            }
        }
    }
    gaps
}

#[test]
fn plugin_says_architectural_constraint() {
    // req:plugin/says-architectural-constraint@c7ceabe6
    // A skill that calls an entry of `ARCHITECTURE.md` by the old term is reported with its
    // file, its line and the term.
    let stale = [(
        "plugin/skills/propose/SKILL.md".to_owned(),
        "1. Read it.\n2. Each Architecture Requirement you add needs a block.\n".to_owned(),
    )];
    assert_eq!(
        old_term_gaps(&stale),
        ["plugin/skills/propose/SKILL.md:2: holds the old term `architecture requirement`"]
    );
    let entry = [(
        "x".to_owned(),
        format!("Add a `{OLD_ENTRY} <slug>` to `ARCHITECTURE.md`.\n"),
    )];
    assert_eq!(
        old_term_gaps(&entry),
        ["x:1: holds the old term `### Requirement:`"]
    );
    // The new term, and a node's requirements, are not reported.
    let fine = [(
        "x".to_owned(),
        "Each architectural constraint is a `### Constraint: <slug>` in `ARCHITECTURE.md`; a node's requirements are proven by tests.\n"
            .to_owned(),
    )];
    assert!(old_term_gaps(&fine).is_empty());
    // The shipped plugin holds none, and the propose and init skills name the new form.
    let mut files = Vec::new();
    text_files_under(&plugin(), &mut files);
    let gaps = old_term_gaps(&files);
    assert!(gaps.is_empty(), "{gaps:#?}");
    for skill in ["propose", "init"] {
        let gaps = text_gaps(
            skill,
            &skill_body(skill),
            &["### Constraint: <slug>", "## Constraints"],
            &[],
        );
        assert!(gaps.is_empty(), "{gaps:#?}");
    }
}

#[test]
fn propose_clears_workarounds() {
    // req:plugin/propose-clears-workarounds@33b6c8ba
    let body = skill_body("propose");
    let rule = body
        .lines()
        .find(|l| l.contains("delete each `workaround` backlog item"))
        .expect("propose: SKILL.md must tell Claude to delete the workaround items a delta clears");
    for needle in [
        "modifies or removes a requirement",
        "whose `requirement` names it",
        "re-record the item's `requirement` at the version `clm hashes <component>` prints after `clm apply`",
        "the model rejects a flag at an old version",
        "delete the `reconsider-<component>-<slug>` question the change answers",
    ] {
        assert!(rule.contains(needle), "{needle}: {rule}");
    }
    // A skill that doesn't mention `workaround` is reported.
    let gaps = text_gaps(
        "propose",
        &body.replace("workaround", "elided"),
        &["workaround"],
        &[],
    );
    assert_reports(&gaps, "propose", "workaround");
}

/// Whether `text` holds `word` as a whole word at byte `at`: no letter, digit or `_`
/// directly before it, and none of those or `-` after it (so `human-task` isn't `human`).
fn word_at(text: &str, at: usize, word: &str) -> bool {
    let before = text[..at].chars().next_back();
    let after = text[at + word.len()..].chars().next();
    !before.is_some_and(|c| c.is_alphanumeric() || c == '_')
        && !after.is_some_and(|c| c.is_alphanumeric() || c == '_' || c == '-')
}

fn has_word(line: &str, word: &str) -> bool {
    line.match_indices(word)
        .any(|(at, _)| word_at(line, at, word))
}

/// The old word `line` holds, if any: `specd` as a word of its own, `friction` after `clm `
/// or as a backlog kind (`kind: friction`, or in backticks after the word kind; the plain
/// English word keeps it), `human` as a backlog kind, and `burdens` as the key (inside
/// backticks, or before `: [`). A line holding `clm:remove-by` is exempt.
fn old_word(line: &str) -> Option<&'static str> {
    if line.contains(concat!("clm:remove", "-by")) {
        return None;
    }
    if has_word(line, "specd") {
        return Some("specd");
    }
    for (word, leads) in [
        ("friction", &["clm ", "kind: ", "kind `"][..]),
        ("human", &["kind: ", "kind `"][..]),
    ] {
        for lead in leads {
            if line.match_indices(lead).any(|(at, _)| {
                let at = at + lead.len();
                line[at..].starts_with(word) && word_at(line, at, word)
            }) {
                return Some(word);
            }
        }
    }
    if line.contains("`burdens`") || line.contains("burdens: [") {
        return Some("burdens");
    }
    None
}

/// `file:line: old word` for each line of `files` (path, text) holding one.
fn old_word_gaps(files: &[(String, String)]) -> Vec<String> {
    let mut gaps = Vec::new();
    for (path, text) in files {
        for (n, line) in text.lines().enumerate() {
            if let Some(word) = old_word(line) {
                gaps.push(format!("{path}:{}: {word}", n + 1));
            }
        }
    }
    gaps
}

fn plugin_files(dir: &Path, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            plugin_files(&path, out);
        } else if let Ok(text) = fs::read_to_string(&path) {
            let rel = path.strip_prefix(plugin()).unwrap().display().to_string();
            out.push((rel, text));
        }
    }
}

#[test]
fn plugin_says_new_words() {
    // req:plugin/skills-say-new-words@8eeb6251
    let mut files = Vec::new();
    plugin_files(&plugin(), &mut files);
    assert!(files.len() > 5);
    assert_eq!(old_word_gaps(&files), Vec::<String>::new());
    let text = |name: &str| {
        files
            .iter()
            .find(|(p, _)| p == name)
            .unwrap_or_else(|| panic!("{name}"))
            .1
            .clone()
    };
    assert!(text("skills/init/SKILL.md").contains("status: unbuilt"));
    assert!(text("skills/health/SKILL.md").contains("`workaround`"));
    // Each old word is reported with its file and line; the new words and exempt lines pass.
    let fixture = |line: &str| vec![("skills/x/SKILL.md".to_string(), format!("ok\n{line}\n"))];
    for (line, word) in [
        ("`status: specd`", "specd"),
        ("run `clm friction` here", "friction"),
        ("an item of kind: friction here", "friction"),
        ("an item of kind `friction` here", "friction"),
        ("an item of kind: human here", "human"),
        ("an item of kind `human` here", "human"),
        ("the `burdens` key", "burdens"),
        ("burdens: [a]", "burdens"),
    ] {
        assert_eq!(
            old_word_gaps(&fixture(line)),
            vec![format!("skills/x/SKILL.md:2: {word}")],
            "{line}"
        );
    }
    for line in [
        "status: unbuilt",
        "a friction flag, as plain English",
        "kind: human-task and `human` lines",
        "the human_burdens key, naming burdens",
        "specd, friction and kind: human on a clm:remove-by line",
    ] {
        assert_eq!(
            old_word_gaps(&fixture(line)),
            Vec::<String>::new(),
            "{line}"
        );
    }
}

#[test]
fn build_reports_workarounds() {
    // req:plugin/build-reports-workarounds@75374dd5
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
    // req:plugin/triage-folds-renames-alone@9181638e
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

/// The old words `component-words` retires from text CLM ships: `node` and `nodes`
/// (which also cover the flags `--node` and `--nodes` and the keys `node:` and
/// `nodes:`), `spine` and `frontier stub`.
const OLD_COMPONENT_WORDS: &[&str] = &["nodes", "node", "spine", "frontier stub"];

/// The old words in `line` that stand as words of their own: no letter, digit or `_`
/// beside them, not joined by `-`, `/`, `.` or `:` to more of an ID, a path or a key,
/// and not inside a `{...}` placeholder. Ignores case.
fn old_component_words_in(line: &str) -> Vec<&'static str> {
    let chars: Vec<char> = line.to_lowercase().chars().collect();
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let joins = |c: char| matches!(c, '-' | '/' | '.' | ':');
    let mut found = Vec::new();
    for word in OLD_COMPONENT_WORDS {
        let w: Vec<char> = word.chars().collect();
        let mut i = 0;
        while i + w.len() <= chars.len() {
            if chars[i..i + w.len()] == w[..] {
                let end = i + w.len();
                let before_ok = i == 0
                    || (!is_word(chars[i - 1])
                        && !(joins(chars[i - 1]) && i > 1 && is_word(chars[i - 2])));
                let after_ok = end == chars.len()
                    || (!is_word(chars[end])
                        && !(joins(chars[end])
                            && end + 1 < chars.len()
                            && is_word(chars[end + 1])));
                let in_placeholder = i > 0 && chars[i - 1] == '{' && chars.get(end) == Some(&'}');
                if before_ok && after_ok && !in_placeholder {
                    found.push(*word);
                    i = end;
                    continue;
                }
            }
            i += 1;
        }
    }
    found
}

/// One line per old word in `files` (path and text), naming the file, its line and the
/// word, leaving out a line holding `clm:remove-by`.
fn old_component_word_gaps(files: &[(String, String)]) -> Vec<String> {
    let mut gaps = Vec::new();
    for (path, text) in files {
        for (i, line) in text.lines().enumerate() {
            if line.contains("clm:remove-by") {
                continue;
            }
            for word in old_component_words_in(line) {
                gaps.push(format!("{path}:{}: says the old word `{word}`", i + 1));
            }
        }
    }
    gaps
}

#[test]
fn skills_say_component() {
    // req:plugin/skills-say-component@092ab3ef
    // A skill that says an old word, flag or key is reported with its file, line and word.
    let stale = [(
        "plugin/skills/init/SKILL.md".to_owned(),
        "1. Run it.\n2. Run `clm survey --nodes map.yaml` for each node.\n".to_owned(),
    )];
    assert_eq!(
        old_component_word_gaps(&stale),
        [
            "plugin/skills/init/SKILL.md:2: says the old word `nodes`",
            "plugin/skills/init/SKILL.md:2: says the old word `node`",
        ]
    );
    let keys = [(
        "x".to_owned(),
        "Write `node: <id>` and `nodes: [a]`; read the spine.\n".to_owned(),
    )];
    assert_eq!(
        old_component_word_gaps(&keys),
        [
            "x:1: says the old word `nodes`",
            "x:1: says the old word `node`",
            "x:1: says the old word `spine`",
        ]
    );
    // IDs, paths and placeholders that merely hold the word, and a line marked for
    // removal, are not reported.
    let fine = [(
        "x".to_owned(),
        "See `node-reconciliation`, `slim-nodes`, `check/node-size` and {node}; the component's ancestors.\nThe old `--node` flag, clm:remove-by on this line.\n"
            .to_owned(),
    )];
    assert!(old_component_word_gaps(&fine).is_empty());
    // The shipped plugin says none, and the init skill names `clm survey --components`
    // and `components:`.
    let mut files = Vec::new();
    text_files_under(&plugin(), &mut files);
    assert!(files.len() > 8, "{} files read", files.len());
    let gaps = old_component_word_gaps(&files);
    assert!(gaps.is_empty(), "{gaps:#?}");
    let gaps = text_gaps(
        "init",
        &skill_body("init"),
        &["clm survey --components", "components:"],
        &[],
    );
    assert!(gaps.is_empty(), "{gaps:#?}");
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
    // req:plugin/skill-named-build@80390f85
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
    // The shipped plugin, guide, CLAUDE.md and README say none.
    let mut files = Vec::new();
    text_files_under(&plugin(), &mut files);
    text_files_under(&repo().join("docs/guide"), &mut files);
    for name in ["CLAUDE.md", "README.md"] {
        files.push((
            name.to_owned(),
            fs::read_to_string(repo().join(name)).unwrap(),
        ));
    }
    let gaps = old_skill_name_gaps(&files);
    assert!(gaps.is_empty(), "{gaps:#?}");
}

/// The `## Contributing back from a fork` section of a `propose` skill, up to the next `## `.
fn fork_section(body: &str) -> &str {
    let Some(at) = body.find("## Contributing back from a fork") else {
        return "";
    };
    let section = &body[at..];
    &section[..section[3..]
        .find("\n## ")
        .map_or(section.len(), |end| end + 3)]
}

/// What `propose-in-a-fork` asks of the `propose` skill, one line per missing piece.
fn fork_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        fork_section(body),
        &[
            "## Contributing back from a fork",
            "clm contribution parent",
            "clm contribution sync",
            "clm contribution open",
            "Allow edits by maintainers",
            "clm guide contributions",
            "clm land",
            "clm approve",
        ],
        &[],
    )
}

#[test]
fn propose_in_a_fork() {
    // req:plugin/propose-in-a-fork@c67f83af
    let body = skill_body("propose");
    assert_eq!(fork_gaps(&body), Vec::<String>::new());
    let section = fork_section(&body);
    // Asked once, before the branch is cut.
    let ask = sentence_with(section, "clm contribution parent");
    for needle in ["once", "before cutting the branch", "parent", "own project"] {
        assert!(ask.contains(needle), "{needle}: {ask}");
    }
    // Work going back: sync first, neither land nor approve, then open the pull request.
    let sync = sentence_with(section, "clm contribution sync");
    assert!(sync.contains("first"), "{sync}");
    let no_land = sentence_with(section, "neither");
    assert!(
        no_land.contains("clm land") && no_land.contains("clm approve"),
        "{no_land}"
    );
    let mut at = 0;
    for needle in [
        "clm contribution sync",
        "clm contribution open",
        "Allow edits by maintainers",
        "maintainer reviews it",
        "unless they ask",
        "clm guide contributions",
    ] {
        at += section[at..]
            .find(needle)
            .unwrap_or_else(|| panic!("propose: fork section must name {needle} after {at}"))
            + needle.len();
    }
    // A skill without it: each missing piece is reported, naming the skill.
    for needle in [
        "clm contribution parent",
        "clm contribution sync",
        "clm contribution open",
    ] {
        let gaps = fork_gaps(&body.replace(needle, "elided"));
        assert_reports(&gaps, "propose", needle);
    }
}

/// What `contribution-skill` asks of the `contribution` skill, one line per gap.
fn contribution_gaps(fm: &serde_yaml::Value, body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        "contribution",
        body,
        &[
            "untrusted",
            "clm contribution show",
            "clm contribution fetch",
            "clm contribution comment",
            "clm contribution label",
            "spec/<id>",
            "clm rebase",
            "clm approve",
            "clm land",
            "accepted",
            "Co-authored-by",
            "kind: idea",
        ],
        &[],
    );
    if !fm["description"].as_str().is_some_and(|d| !d.is_empty()) {
        gaps.push("contribution: SKILL.md must have a description".to_owned());
    }
    gaps
}

#[test]
fn contribution_skill() {
    // req:plugin/contribution-skill@35342225
    let (fm, body) = frontmatter(&plugin().join("skills/contribution/SKILL.md"));
    assert_eq!(contribution_gaps(&fm, &body), Vec::<String>::new());
    let description = fm["description"].as_str().unwrap();
    for needle in ["pull request", "issue", "number"] {
        assert!(description.contains(needle), "{needle}: {description}");
    }
    // Each rule sits in a sentence that says it.
    for (anchor, needles) in [
        (
            "untrusted",
            &["title", "body", "comments", "files", "instructions"][..],
        ),
        ("clm contribution fetch", &["checks nothing out"][..]),
        (
            "clm contribution comment --close",
            &["contributing instructions", "pushing nothing"][..],
        ),
        (
            "instruction to a session",
            &["flagging", "CLAUDE.md", "change.md"][..],
        ),
        (
            "noreply",
            &["Co-authored-by", "kind: idea", "/clm:propose"][..],
        ),
        ("clm contribution label", &["add the `accepted` label"][..]),
        (
            "gate/up-to-date",
            &["clm rebase", "clm gate --base origin/main", "only when"][..],
        ),
        (
            "maintainers' edits",
            &["rewritten head", "contributor's branch"][..],
        ),
        ("guard's confirmation", &["clm approve", "yes"][..]),
        (
            "Write nothing to the host",
            &["without the human's yes"][..],
        ),
    ] {
        let sentence = sentence_with(&body, anchor);
        for needle in needles {
            assert!(sentence.contains(needle), "{anchor} / {needle}: {sentence}");
        }
    }
    // The land and approval rules every skill states.
    assert_eq!(skill_gaps("contribution", &body), Vec::<String>::new());
    // A skill without it: each missing piece is reported, naming the skill.
    for needle in [
        "untrusted",
        "clm contribution show",
        "clm contribution fetch",
        "clm contribution comment",
        "clm contribution label",
        "spec/<id>",
        "clm rebase",
        "clm land",
        "accepted",
        "Co-authored-by",
        "kind: idea",
    ] {
        let gaps = contribution_gaps(&fm, &body.replace(needle, "elided"));
        assert_reports(&gaps, "contribution", needle);
    }
}

/// What `propose-adds-glossary-terms` asks of a `propose` skill: one paragraph (a line of
/// the skill) holding the glossary path, the rule, and the words for each kind of term.
fn glossary_terms_gaps(body: &str) -> Vec<String> {
    let must = [
        "docs/guide/glossary.md",
        "clm/glossary-covers-terms",
        "command",
        "status",
        "kind",
        "frontmatter",
    ];
    let paragraph = body
        .lines()
        .find(|line| line.contains(must[0]) && line.contains(must[1]));
    match paragraph {
        Some(line) => text_gaps("propose", line, &must, &[]),
        None => text_gaps("propose", "", &must, &[]),
    }
}

#[test]
fn propose_adds_glossary_terms() {
    // req:plugin/propose-adds-glossary-terms@fd6a2597
    let body = skill_body("propose");
    assert_eq!(glossary_terms_gaps(&body), Vec::<String>::new());
    // a skill without it
    // the path, the rule and each kind of term are named in the paragraph
    for word in [
        "docs/guide/glossary.md",
        "clm/glossary-covers-terms",
        "frontmatter",
    ] {
        let gaps = glossary_terms_gaps(&body.replace(word, "elided"));
        assert!(gaps.iter().any(|g| g.contains(word)), "{word}: {gaps:?}");
    }
}

/// What `architecture-auditor` asks of the agent's frontmatter and prompt, one line per gap.
fn architecture_auditor_gaps(fm: &serde_yaml::Value, body: &str) -> Vec<String> {
    let tools = agent_tools(fm);
    let mut gaps: Vec<String> = ["Edit", "Write", "NotebookEdit"]
        .into_iter()
        .filter(|tool| tools.iter().any(|t| t == tool))
        .map(|tool| format!("architecture-auditor: may not have `{tool}`"))
        .collect();
    gaps.extend(
        [
            "ARCHITECTURE.md",
            "git diff origin/main...HEAD",
            "clm context",
            "docs/decisions/",
            "untrusted",
            "no findings",
            "git switch",
            "git checkout",
            "git stash",
            "git reset",
            "`cd`",
            "severity",
            "Never edit",
            "clm guide constraints",
            "#### Replaces",
            "#### Carries",
            "Guarantee",
            "`replaceable`",
            "`default`",
        ]
        .into_iter()
        .filter(|needle| !body.contains(needle))
        .map(|needle| format!("architecture-auditor: must name `{needle}`")),
    );
    gaps
}

#[test]
fn architecture_auditor() {
    // req:plugin/architecture-auditor@e2b0c516
    let (fm, body) = frontmatter(&plugin().join("agents/architecture-auditor.md"));
    assert_eq!(fm["name"], "architecture-auditor");
    assert!(
        fm["description"]
            .as_str()
            .is_some_and(|d| d.contains("ARCHITECTURE.md"))
    );
    assert_eq!(architecture_auditor_gaps(&fm, &body), Vec::<String>::new());
    // It reads each constraint's text and reports constraints, gaps and decisions with quotes.
    for needle in [
        "each constraint's text",
        "each gap",
        "each decision",
        "quoting its sentence",
        "the element of the change",
        "`clm:remove-by`",
        "outside-contributions-are-intent",
    ] {
        assert!(body.contains(needle), "{needle}");
    }
    // auditor that can edit
    let mut with_write = fm.clone();
    with_write["tools"] = "Read, Glob, Grep, Bash, Write".into();
    let gaps = architecture_auditor_gaps(&with_write, &body);
    assert_eq!(gaps, ["architecture-auditor: may not have `Write`"]);
    // auditor that reads only the names
    let gaps = architecture_auditor_gaps(&fm, &body.replace("docs/decisions/", "elided"));
    assert_eq!(gaps, ["architecture-auditor: must name `docs/decisions/`"]);
    // auditor that ignores replacements
    let gaps = architecture_auditor_gaps(&fm, &body.replace("#### Replaces", "elided"));
    assert_eq!(gaps, ["architecture-auditor: must name `#### Replaces`"]);
    // It reads the carried constraints unless the file's entries carry them, judges each
    // replacement against the guarantee, and reports a first `#### Carries` block.
    for needle in [
        "CLM's carried constraints",
        "`carried-constraints`",
        "where they are already in it",
        "quoted",
        "each `default` constraint it drops",
        "whose entries held none",
    ] {
        assert!(body.contains(needle), "{needle}");
    }
}

/// What `help-skill` asks of the `help` skill: it answers from what the installed `clm`
/// carries and changes nothing.
fn help_skill_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "help",
        body,
        &[
            "clm guide constraints",
            "clm guide quickstart",
            "clm status",
            "clm backlog",
            "/clm:init",
            "/clm:propose",
            "/clm:build",
            "/clm:triage",
            "/clm:health",
            "/clm:contribution",
            "/clm:view",
            "`replaceable`",
            "`default`",
            "`method`",
            "say its tier and what CLM relies on",
            "never from this repository's files or a URL",
            "rather than guess",
            "clm guide <page>",
            "Help changes no file.",
        ],
        &["docs/guide/"],
    )
}

#[test]
fn help_skill() {
    // req:plugin/help-skill@67498eaa
    let (fm, body) = frontmatter(&plugin().join("skills/help/SKILL.md"));
    assert!(
        fm["description"]
            .as_str()
            .is_some_and(|d| d.contains("questions about CLM") && d.contains("new user"))
    );
    assert_eq!(help_skill_gaps(&body), Vec::<String>::new());
    // help without the guide
    let gaps = help_skill_gaps(&body.replace("clm guide constraints", "elided"));
    assert_reports(&gaps, "help", "clm guide constraints");
    // help that writes
    let gaps = help_skill_gaps(&body.replace("Help changes no file.", "elided"));
    assert_reports(&gaps, "help", "Help changes no file.");
    // It names the commands that write only to say it runs none of them.
    for command in [
        "clm claim",
        "clm apply",
        "clm land",
        "clm approve",
        "git commit",
    ] {
        assert!(body.contains(command), "{command}");
    }
}

/// What `propose-replacing-constraints` asks of the `propose` skill.
fn replacing_constraints_gaps(body: &str) -> Vec<String> {
    text_gaps(
        "propose",
        body,
        &[
            "#### Replaces",
            "clm guide constraints",
            "`carried-constraints`",
            "`method`",
            "architecture-auditor",
            "## Open questions",
            "`deterministic-enforcement`",
        ],
        &[],
    )
}

#[test]
fn propose_replacing_constraints() {
    // req:plugin/propose-replacing-constraints@44f3128f
    let body = skill_body("propose");
    assert_eq!(replacing_constraints_gaps(&body), Vec::<String>::new());
    // propose without replacements
    let gaps = replacing_constraints_gaps(&body.replace("#### Replaces", "elided"));
    assert_reports(&gaps, "propose", "#### Replaces");
    // The paragraph tells the tier, what CLM relies on, the guarantee and what is given up,
    // refuses a `method` constraint, and leaves the decision to the human.
    let paragraph = body
        .split("\n\n")
        .find(|p| p.contains("#### Replaces"))
        .expect("a paragraph on replacing constraints");
    for needle in [
        "its tier",
        "what CLM relies on",
        "what a replacement must guarantee",
        "what the project gives up",
        "Refuse to replace a `method` constraint",
        "`replaceable`",
        "`#### Enforced by` or `#### Untested`",
        "the human's approval, not the audit, decides",
        "under `## Open questions`",
        "The `architecture-auditor` judges",
        "`deterministic-enforcement`",
    ] {
        assert!(paragraph.contains(needle), "{needle}");
    }
}

/// What `architecture-audit` asks of a skill's text.
fn architecture_audit_gaps(skill: &str, body: &str) -> Vec<String> {
    let must: &[&str] = match skill {
        "propose" => &[
            "architecture-auditor",
            "## Open questions",
            "spec-architecture-audit",
        ],
        _ => &["architecture-auditor"],
    };
    text_gaps(skill, body, must, &[])
}

#[test]
fn architecture_audit() {
    // req:plugin/architecture-audit@504aa12b
    let propose = skill_body("propose");
    assert_eq!(
        architecture_audit_gaps("propose", &propose),
        Vec::<String>::new()
    );
    let steps = numbered_steps(&propose);
    let check = steps
        .iter()
        .find(|s| s.contains("Run a builder check"))
        .expect("builder check step");
    for needle in [
        "architecture-auditor",
        "## Open questions",
        "spec-architecture-audit",
        "same message",
        "only the change ID and the path of this checkout",
        "advises and never gates",
    ] {
        assert!(check.contains(needle), "{needle}");
    }
    let launch = sentence_with(check, "`architecture-auditor`");
    assert!(launch.contains(OWN_MODEL), "{launch}");
    assert!(launch.contains("only the change ID"), "{launch}");
    let fix = sentence_with(check, "each `architecture-auditor` finding");
    assert!(fix.contains("## Open questions"), "{fix}");
    assert!(check.contains("and so does the audit"), "{check}");
    let contribution = skill_body("contribution");
    assert_eq!(
        architecture_audit_gaps("contribution", &contribution),
        Vec::<String>::new()
    );
    let audit = contribution.find("architecture-auditor").unwrap();
    assert!(audit < contribution.find("Land by the first line").unwrap());
    let ask = sentence_with(&contribution, "reports findings");
    assert!(ask.contains("ask before landing"), "{ask}");
    // a propose skill without the audit
    let gaps = architecture_audit_gaps(
        "propose",
        &propose.replace("architecture-auditor", "elided"),
    );
    assert_reports(&gaps, "propose", "architecture-auditor");
}

#[test]
fn triage_revises_a_declined_fold() {
    // req:plugin/triage-revises-declined-fold@b4e8e3cf
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
        "ask again",
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

/// What `skills-push-with-clm` asks of a `SKILL.md`: no raw push.
fn raw_push_gaps(skill: &str, body: &str) -> Vec<String> {
    text_gaps(skill, body, &[], &[concat!("git", " push")])
}

#[test]
fn skills_push_with_clm() {
    // req:plugin/skills-push-with-clm@69c54954
    let skills = plugin().join("skills");
    let mut names: Vec<String> = std::fs::read_dir(&skills)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert!(!names.is_empty());
    for name in &names {
        assert_eq!(raw_push_gaps(name, &skill_body(name)), Vec::<String>::new());
    }
    for name in ["propose", "contribution", "triage", "build"] {
        let gaps = text_gaps(name, &skill_body(name), &["clm branch push"], &[]);
        assert_eq!(gaps, Vec::<String>::new());
    }
    for name in ["propose", "build"] {
        let gaps = text_gaps(name, &skill_body(name), &["clm branch delete"], &[]);
        assert_eq!(gaps, Vec::<String>::new());
    }
    // a skill with a raw push
    let raw = skill_body("triage").replace(
        "clm branch push",
        concat!("git", " push --force-with-lease"),
    );
    assert_reports(&raw_push_gaps("triage", &raw), "triage", "git push");
}

/// What `unattended-permissions-check` asks of the `build` and `triage` skills.
fn permission_check_gaps(skill: &str, body: &str) -> Vec<String> {
    text_gaps(
        skill,
        body,
        &[
            "clm init --check-permissions",
            "auto mode",
            ".claude/settings.json",
            ".claude/settings.local.json",
            "exits 1",
            "any other code",
            "never refusing",
            "Never write either file",
        ],
        &[],
    )
}

#[test]
fn skills_check_the_permission_rules() {
    // req:plugin/unattended-permissions-check@e8390775
    for name in ["build", "triage"] {
        assert_eq!(
            permission_check_gaps(name, &skill_body(name)),
            Vec::<String>::new()
        );
    }
    let build = skill_body("build");
    let gaps = text_gaps(
        "build",
        &build,
        &["clm branch push", "clm branch delete"],
        &[concat!("git", " push --force-with-lease")],
    );
    assert_eq!(gaps, Vec::<String>::new());
    // the check comes before the first launch
    let triage = skill_body("triage");
    assert!(
        triage.find("clm init --check-permissions").unwrap()
            < triage.find("AskUserQuestion").unwrap()
    );
    // skill without the check
    let gaps = permission_check_gaps(
        "triage",
        &triage.replace("clm init --check-permissions", "elided"),
    );
    assert_reports(&gaps, "triage", "clm init --check-permissions");
}

/// What `builder-check-asks-migrations` asks of the `propose` skill's builder-check step.
fn migration_gaps(step: &str) -> Vec<String> {
    text_gaps(
        "propose",
        step,
        &[
            "`clm init`",
            "migration",
            "requirement of `upgrade`",
            "`done` check",
            "`apply` or `note`",
            "upgrades-carry-migrations",
            "`required` migration is always an `apply`",
        ],
        &[],
    )
}

/// What `builder-check-capability-tests` asks of the `propose` skill's builder-check step.
fn capability_test_gaps(step: &str) -> Vec<String> {
    text_gaps(
        "propose",
        step,
        &[
            "capability tests",
            "capability-tests-follow-rules",
            "neither a requirement of `capability`",
            "nor a `#### Needs`",
            "nor a backlog item",
            "`CAPABILITIES.md` entry",
        ],
        &[],
    )
}

fn builder_check_step() -> String {
    numbered_steps(&skill_body("propose"))
        .into_iter()
        .find(|s| s.contains("Run a builder check"))
        .expect("builder check step")
}

#[test]
fn builder_check_asks_migrations() {
    // req:plugin/builder-check-asks-migrations@16ee864d
    let step = builder_check_step();
    assert_eq!(migration_gaps(&step), Vec::<String>::new());
    let gaps = migration_gaps(&step.replace("upgrades-carry-migrations", "elided"));
    assert_reports(&gaps, "propose", "upgrades-carry-migrations");
}

#[test]
fn builder_check_asks_capability_tests() {
    // req:plugin/builder-check-capability-tests@5dd84ff9
    let step = builder_check_step();
    assert_eq!(capability_test_gaps(&step), Vec::<String>::new());
    let gaps = capability_test_gaps(&step.replace("capability-tests-follow-rules", "elided"));
    assert_reports(&gaps, "propose", "capability-tests-follow-rules");
}
