use clm_agent_harness::{AgentHarness, CODEX_MARKER, detect};
use std::collections::HashMap;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let m: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |k| m.get(k).cloned()
}

// req:agent-harness/detect@a89f980b
#[test]
fn claude_code_marker() {
    assert_eq!(
        detect(env(&[("CLAUDECODE", "1")])),
        AgentHarness::ClaudeCode
    );
    assert_eq!(
        detect(env(&[("CLAUDECODE", "yes")])),
        AgentHarness::ClaudeCode
    );
}

// req:agent-harness/detect@a89f980b
#[test]
fn codex_inside_claude_code_is_codex() {
    assert_eq!(
        detect(env(&[("CLAUDECODE", "1"), (CODEX_MARKER, "t")])),
        AgentHarness::Codex
    );
    assert_eq!(detect(env(&[(CODEX_MARKER, "t")])), AgentHarness::Codex);
}

// req:agent-harness/detect@a89f980b
#[test]
fn codex_marker_is_the_variable_codex_sets() {
    assert_eq!(CODEX_MARKER, "CODEX_THREAD_ID");
    assert_eq!(
        detect(env(&[("CODEX_THREAD_ID", "t")])),
        AgentHarness::Codex
    );
}

// req:agent-harness/detect@a89f980b
#[test]
fn humans_terminal_is_none() {
    assert_eq!(detect(env(&[])), AgentHarness::None);
    assert_eq!(detect(env(&[("CLAUDECODE", "")])), AgentHarness::None);
    assert_eq!(detect(env(&[(CODEX_MARKER, "")])), AgentHarness::None);
}

// req:agent-harness/detect@a89f980b
#[test]
fn names_round_trip() {
    for (name, h) in [
        ("claude-code", AgentHarness::ClaudeCode),
        ("codex", AgentHarness::Codex),
        ("none", AgentHarness::None),
    ] {
        assert_eq!(AgentHarness::from_name(name), Some(h));
        assert_eq!(h.name(), name);
    }
    assert_eq!(AgentHarness::from_name("claude"), None);
    assert_eq!(AgentHarness::from_name(""), None);
}
