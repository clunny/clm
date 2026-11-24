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

/// The YAML frontmatter of a markdown file.
fn frontmatter(path: &Path) -> serde_yaml::Value {
    let text = fs::read_to_string(path).unwrap();
    let rest = text
        .strip_prefix(
            "---
",
        )
        .expect("frontmatter");
    let (yaml, _) = rest
        .split_once(
            "
---
",
        )
        .expect("frontmatter end");
    serde_yaml::from_str(yaml).unwrap()
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

/// What an agent's frontmatter lacks: its `name`, a non-empty `description`, and tools that
/// leave out `Edit` and `Write`. One line per gap, naming the agent.
fn agent_gaps(agent: &str, fm: &serde_yaml::Value) -> Vec<String> {
    let mut gaps = Vec::new();
    if fm["name"] != agent {
        gaps.push(format!("{agent}: name must be `{agent}`"));
    }
    if !fm["description"].as_str().is_some_and(|d| !d.is_empty()) {
        gaps.push(format!("{agent}: needs a description"));
    }
    let tools = agent_tools(fm);
    gaps.extend(
        ["Edit", "Write"]
            .into_iter()
            .filter(|tool| tools.iter().any(|t| t == tool))
            .map(|tool| format!("{agent}: may not have `{tool}`")),
    );
    gaps
}

/// Check `agent`'s file as shipped, then with each editing tool added to its tools.
fn check_read_only_agent(agent: &str) {
    let fm = frontmatter(&plugin().join(format!("agents/{agent}.md")));
    assert_eq!(agent_gaps(agent, &fm), Vec::<String>::new());
    for tool in ["Edit", "Write"] {
        let mut editing = fm.clone();
        editing["tools"] = format!("{}, {tool}", fm["tools"].as_str().unwrap()).into();
        assert_eq!(
            agent_gaps(agent, &editing),
            [format!("{agent}: may not have `{tool}`")]
        );
    }
}

#[test]
fn spec_auditor() {
    // req:plugin.agents/spec-auditor@3a5b1a67
    check_read_only_agent("spec-auditor");
}

#[test]
fn spec_slimmer() {
    // req:plugin.agents/spec-slimmer@eb082984
    check_read_only_agent("spec-slimmer");
}

#[test]
fn architecture_auditor() {
    // req:plugin.agents/architecture-auditor@3eef0a5d
    check_read_only_agent("architecture-auditor");
}

/// What `issue-verifier` asks of the verifier's tools: exactly `Read`, `Glob` and `Grep`.
fn verifier_tool_gaps(fm: &serde_yaml::Value) -> Vec<String> {
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
    gaps
}

#[test]
fn issue_verifier() {
    // req:plugin.agents/issue-verifier@fd670af4
    let fm = frontmatter(&plugin().join("agents/issue-verifier.md"));
    assert_eq!(fm["name"], "issue-verifier");
    assert!(fm["description"].as_str().is_some_and(|d| !d.is_empty()));
    assert_eq!(verifier_tool_gaps(&fm), Vec::<String>::new());
    // verifier that could run commands
    let mut running = fm.clone();
    running["tools"] = format!("{}, Bash", fm["tools"].as_str().unwrap()).into();
    assert_eq!(
        verifier_tool_gaps(&running),
        ["issue-verifier: may not have `Bash`"]
    );
}

#[test]
fn launch_models() {
    // req:plugin.agents/launch-models@5c6cd1bd
    for agent in ["spec-auditor", "architecture-auditor", "spec-slimmer"] {
        let fm = frontmatter(&plugin().join(format!("agents/{agent}.md")));
        assert_eq!(
            fm["model"], "sonnet",
            "{agent}: frontmatter must set model: sonnet"
        );
    }
}
