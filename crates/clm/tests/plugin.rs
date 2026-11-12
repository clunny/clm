//! The plugin under `plugin/`: its package, hooks, skills, agent and `bin/clm` shim.
// clm:reads docs/guide/**
// clm:reads CLAUDE.md
// clm:reads README.md

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
fn codex_package() {
    // req:plugin/codex-package@88f0c401
    // clm:reads .agents/plugins/marketplace.json
    // clm:reads plugin/.codex-plugin/plugin.json
    let market = json(&repo().join(".agents/plugins/marketplace.json"));
    let listed = market["plugins"].as_array().unwrap();
    let clm = listed
        .iter()
        .find(|p| p["name"] == "clm")
        .expect("clm listed");
    assert_eq!(clm["source"]["path"], "./plugin", "{market}");
    let manifest = json(&plugin().join(".codex-plugin/plugin.json"));
    assert_eq!(manifest["name"], "clm");
    assert_workspace_version(&manifest, "codex plugin");
    // One set of skills, and the hooks file the Codex wiring ships.
    assert_eq!(manifest["skills"], "./skills/");
    assert_eq!(manifest["hooks"], "./hooks/codex.json");
    assert!(plugin().join("skills").is_dir());
    // Both packages run the same shim.
    assert!(plugin().join("bin/clm").is_file());
    let claude = fs::read_to_string(plugin().join("hooks/hooks.json")).unwrap();
    assert!(claude.contains("bin/clm"), "{claude}");
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
    // req:plugin.panel/panel-package@286ccb95
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

/// The guide pages a `propose` skill names by path (`propose-adds-glossary-terms`,
/// `propose-adds-settings-sections`).
const GLOSSARY_PATH: &str = "docs/guide/glossary.md";
const SETTINGS_PATH: &str = "docs/guide/settings.md";

/// What `method-pointers` asks of every skill and agent: no `docs/guide/` path, and every
/// `clm guide <page>` it names one of `pages`. One line per gap naming the file and text.
fn guide_gaps(name: &str, text: &str, pages: &[String]) -> Vec<String> {
    let mut gaps: Vec<String> = Vec::new();
    // The glossary is the one page a skill names by path (`propose-adds-glossary-terms`):
    // it is a spec path of this repository that a spec branch edits.
    let own = if name.contains("propose") {
        text.replace(GLOSSARY_PATH, "").replace(SETTINGS_PATH, "")
    } else {
        text.to_owned()
    };
    if own.contains("docs/guide/") {
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
    // req:plugin/method-pointers@1ae4a996
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
    // propose names the pages it edits
    let both = format!("Edit `{GLOSSARY_PATH}` and `{SETTINGS_PATH}`.");
    assert!(guide_gaps("skills/propose/SKILL.md", &both, &pages).is_empty());
    assert_eq!(guide_gaps("skills/build/SKILL.md", &both, &pages).len(), 1);
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
    // req:plugin/builder-check@43c9efb5
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
    // A widened type another node matches exhaustively: the arm is a forced fix of the
    // widening's build, and the requirements that reach it through the new value need it.
    assert!(
        removal.contains("and for exhaustive uses of each type the change widens"),
        "{removal}"
    );
    let widened = steps[check]
        .lines()
        .find(|l| l.contains("`forced-fixes-cover-additions`"))
        .expect(
            "skills/propose/SKILL.md's builder check must mention `forced-fixes-cover-additions`",
        );
    for needle in [
        "widens a type another component matches exhaustively",
        "an added enum variant",
        "need nothing of the widening",
        "tested by calling that code directly",
        "adds the arm as a forced fix",
        "reach the code through the new value need the widening",
    ] {
        assert!(widened.contains(needle), "{needle}: {widened}");
    }
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
        "forced-fixes-cover-additions",
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
    // req:plugin/builder-check@43c9efb5
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

/// The skills that name `clm wrap-up` without a paragraph that also names `left <path>`
/// (`wrap-up-left-lines`), and any skill that tells Claude to run `git worktree prune`.
fn wrap_up_gaps(skills: &[(String, String)]) -> Vec<String> {
    let mut gaps = Vec::new();
    for (name, body) in skills {
        if body.contains("git worktree prune") {
            gaps.push(format!("{name}: git worktree prune"));
        }
        if body.contains("clm wrap-up")
            && !body
                .split("\n\n")
                .any(|p| p.contains("clm wrap-up") && p.contains("left <path>"))
        {
            gaps.push(format!("{name}: left <path>"));
        }
    }
    gaps
}

#[test]
fn wrap_up_left_lines() {
    // req:plugin/wrap-up-left-lines@969152a0
    let mut names: Vec<String> = fs::read_dir(plugin().join("skills"))
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_type().unwrap().is_dir())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    let skills: Vec<(String, String)> = names.iter().map(|s| (s.clone(), skill_body(s))).collect();
    assert_eq!(wrap_up_gaps(&skills), Vec::<String>::new());
    // Each skill that runs `clm wrap-up` passes the left lines on, and none of them
    // does it by pruning worktrees itself.
    for name in ["build", "triage", "propose", "contribution", "init"] {
        let body = skill_body(name);
        let p = body
            .split("\n\n")
            .find(|p| p.contains("clm wrap-up") && p.contains("left <path>"))
            .unwrap_or_else(|| panic!("{name}: no paragraph"));
        let sentence = sentence_with(p, "left <path>");
        for needle in [
            "as it stands",
            "couldn't remove",
            "doesn't make the wrap-up incomplete",
            "leave the directory",
            "any command the line names",
            "to them",
        ] {
            assert!(sentence.contains(needle), "{name}: {needle}: {sentence}");
        }
    }
    // A skill that names `clm wrap-up` and no paragraph naming `left <path>` with it.
    let bare = vec![("build".to_string(), "Run `clm wrap-up`.".to_string())];
    assert_reports(&wrap_up_gaps(&bare), "build", "left <path>");
    let pruned = vec![("build".to_string(), "Run `git worktree prune`.".to_string())];
    assert_reports(&wrap_up_gaps(&pruned), "build", "git worktree prune");
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
    // req:plugin/contribution-skill@c27fbc23
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
            &["flagging", "CLAUDE.md", "AGENTS.md", "change.md"][..],
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
            "/clm:configure",
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
    // req:plugin/help-skill@bc92d819
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
    // help on a setting
    let gaps = help_skill_gaps(&body.replace("/clm:configure", "elided"));
    assert_reports(&gaps, "help", "/clm:configure");
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

/// What `configure-skill` asks of a `configure` skill's frontmatter and body.
fn configure_skill_gaps(fm: &serde_yaml::Value, body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        "configure",
        body,
        &[
            "clm config list",
            "clm guide settings",
            "clm.local.toml",
            "config.toml",
            "/clm:upgrade",
            "`upgraded`",
            "`declined_migrations`",
            "`skip_migrations`",
            "clm gate --base origin/main",
            "spec review",
            "clm approve",
            "clm land",
            "clm branch push",
            "clm pr open",
            "clm branch delete",
            "config/<YYYYMMDD>-<slug>",
            ".claude/worktrees/<YYYYMMDD>-<slug>",
            "git switch -c",
            "git switch --detach origin/main",
            "git worktree remove",
            "clm wrap-up",
            "left <path>",
            "session-worktrees",
            "small-change-no-claim",
            "environment variable",
            LAND_RULE,
            APPROVE_RULE,
        ],
        &["git push"],
    );
    let description = fm["description"].as_str().unwrap_or_default();
    for needle in ["settings", "change one"] {
        if !description.contains(needle) {
            gaps.push(format!("configure: description must mention {needle}"));
        }
    }
    gaps
}

#[test]
fn configure_skill() {
    // req:plugin/configure-skill@0cbec06f
    let (fm, body) = frontmatter(&plugin().join("skills/configure/SKILL.md"));
    assert_eq!(configure_skill_gaps(&fm, &body), Vec::<String>::new());
    // a skill that doesn't validate
    let gaps = configure_skill_gaps(&fm, &body.replace("clm config list", "elided"));
    assert_reports(&gaps, "configure", "clm config list");
    // each piece of the landing steps is needed
    for needle in [
        "clm branch delete",
        "git switch --detach origin/main",
        "spec review",
    ] {
        let gaps = configure_skill_gaps(&fm, &body.replace(needle, "elided"));
        assert_reports(&gaps, "configure", needle);
    }
    // The three keys that belong to `/clm:upgrade` are left to it.
    let sentence = sentence_with(&body, "`declined_migrations`");
    assert!(sentence.contains("/clm:upgrade"), "{sentence}");
    // A change to the clone's own or the person's own file commits nothing.
    let sentence = sentence_with(&body, "Commit nothing");
    assert!(
        sentence.contains("clm.local.toml") && sentence.contains("config.toml"),
        "{sentence}"
    );
}

/// What `verify-skill` asks of a `verify` skill's frontmatter and body.
fn verify_skill_gaps(fm: &serde_yaml::Value, body: &str) -> Vec<String> {
    let mut gaps = text_gaps(
        "verify",
        body,
        &[
            "clm verify --commit origin/main",
            "verify off",
            "verify pending",
            "verify passed",
            "verify failed",
            "no command time limit",
            "lands wait",
            "is green at that commit",
            "the log's path",
            "guess nothing",
            "the human's decision that it is real",
            "kind: bug",
            "flaky tests",
            "git switch -c",
            "git worktree add",
            "/clm:configure",
            "check lock",
            "clm setup",
            "[build] setup",
            "git switch --detach",
            "git worktree list",
            "tests-run-in-their-own-tree",
            "verify_failures",
            "git rev-parse <sha>",
            "`P1`",
            "Resolves: backlog/<slug>",
            "verify-after-land",
            "verify/<YYYYMMDD>-<short sha>",
            "clm gate --base origin/main",
            "clm branch push",
            "clm pr open",
            "clm land",
            "clm branch delete",
            "git worktree remove",
            "clm wrap-up",
            "left <path>",
            "session-worktrees",
            LAND_RULE,
        ],
        &["git push"],
    );
    let description = fm["description"].as_str().unwrap_or_default();
    for needle in ["origin/main", "triggers", "slow tests"] {
        if !description.contains(needle) {
            gaps.push(format!("verify: description must mention {needle}"));
        }
    }
    gaps
}

#[test]
fn verify_skill() {
    // req:plugin/verify-skill@26e884db
    let (fm, body) = frontmatter(&plugin().join("skills/verify/SKILL.md"));
    assert_eq!(verify_skill_gaps(&fm, &body), Vec::<String>::new());
    // a skill that doesn't verify main
    let gaps = verify_skill_gaps(
        &fm,
        &body.replace("clm verify --commit origin/main", "elided"),
    );
    assert_reports(&gaps, "verify", "clm verify --commit origin/main");
    // each outcome, the re-run and the landing steps are needed
    for needle in [
        "verify off",
        "verify pending",
        "verify failed",
        "clm setup",
        "git switch --detach",
        "verify_failures",
        "Resolves: backlog/<slug>",
        "clm branch delete",
    ] {
        let gaps = verify_skill_gaps(&fm, &body.replace(needle, "elided"));
        assert_reports(&gaps, "verify", needle);
    }
    // The run takes no command time limit, and the human is told lands wait.
    let sentence = sentence_with(&body, "clm verify --commit origin/main");
    assert!(
        sentence.contains("background") && sentence.contains("check lock"),
        "{sentence}"
    );
    // A re-run changes no record and decides nothing.
    let sentence = sentence_with(&body, "change no record");
    assert!(sentence.contains("decide nothing"), "{sentence}");
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

fn skill_text(name: &str) -> String {
    fs::read_to_string(plugin().join("skills").join(name).join("SKILL.md")).unwrap()
}

/// One gap per needle the text lacks.
fn lacks(skill: &str, body: &str, needles: &[&str]) -> Vec<String> {
    needles
        .iter()
        .filter(|n| !body.contains(*n))
        .map(|n| format!("{skill}: must name {n}"))
        .collect()
}

#[test]
fn skills_run_verify() {
    // req:plugin/skills-run-verify@b8bb7c23
    let run = "clm verify --due --event work-done";
    let build = skill_text("build");
    let triage = skill_text("triage");
    assert_eq!(lacks("build", &build, &[run]), Vec::<String>::new());
    let triage_needles = [
        run,
        "clm verify --import",
        "verify <sha> new",
        "verify_failures",
        "verify-after-land",
        "git rev-parse <sha>",
        "verify failed",
    ];
    assert_eq!(
        lacks("triage", &triage, &triage_needles),
        Vec::<String>::new()
    );
    // Verify comes before the last wrap-up, in both.
    for (name, body) in [("build", &build), ("triage", &triage)] {
        let at = body.find(run).unwrap();
        let wrap = body.rfind("`clm wrap-up`").unwrap();
        assert!(at < wrap, "{name}: verify must precede the last wrap-up");
    }
    assert!(triage.find("clm verify --import").unwrap() < triage.find("`clm intake`").unwrap());
    let gaps = lacks(
        "triage",
        &triage.replace("verify_failures", "x"),
        &triage_needles,
    );
    assert_eq!(gaps, vec!["triage: must name verify_failures".to_string()]);
}
