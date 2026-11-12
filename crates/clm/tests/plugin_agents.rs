//! The plugin's four read-only subagents under `plugin/agents/`.
// clm:reads plugin/agents/**

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
    let rest = text
        .strip_prefix(
            "---
",
        )
        .expect("frontmatter");
    let (yaml, body) = rest
        .split_once(
            "
---
",
        )
        .expect("frontmatter end");
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

#[test]
fn spec_auditor() {
    // req:plugin.agents/spec-auditor@312da3a0
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
    // req:plugin.agents/auditor-flags-unstated-fixes@084d9c14
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
    // req:plugin.agents/spec-auditor-keeps-checkout@ab433b2d
    let (_, body) = frontmatter(&plugin().join("agents/spec-auditor.md"));
    assert_eq!(auditor_checkout_gaps(&body), Vec::<String>::new());
    for read in ["`git diff`", "`git log`"] {
        assert!(body.contains(read), "{read}");
    }
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
            "#### Enforced by",
            "docs/decisions/",
            "#### Needs",
            "say so beside the proposal",
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
    // req:plugin.agents/spec-slimmer@d0110548
    let (fm, body) = frontmatter(&plugin().join("agents/spec-slimmer.md"));
    assert_eq!(slimmer_gaps(&fm, &body), Vec::<String>::new());
    // agent that could edit
    let mut editing = fm.clone();
    editing["tools"] = format!("{}, Edit", fm["tools"].as_str().unwrap()).into();
    let gaps = slimmer_gaps(&editing, &body);
    assert_eq!(gaps, ["spec-slimmer: may not have `Edit`"]);
    // agent without the context command
    let gaps = slimmer_gaps(&fm, &body.replace("clm context", "elided"));
    assert_eq!(gaps, ["spec-slimmer: must name `clm context <component>`"]);
    // agent that doesn't look for references
    for needle in ["#### Enforced by", "docs/decisions/", "#### Needs"] {
        let gaps = slimmer_gaps(&fm, &body.replace(needle, "elided"));
        assert_eq!(gaps, [format!("spec-slimmer: must name `{needle}`")]);
    }
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
    // req:plugin.agents/issue-verifier@28b4ce10
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
    // req:plugin.agents/architecture-auditor@e2b0c516
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
