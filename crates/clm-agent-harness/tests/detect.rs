use clm_agent_harness::{AgentHarness, CODEX_MARKER, detect};
use std::collections::HashMap;

fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
    let m: HashMap<String, String> = pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    move |k| m.get(k).cloned()
}

// req:agent-harness/detect@5df05d5b
#[test]
fn claude_code_marker() {
    assert_eq!(
        detect(env(&[("CLAUDECODE", "1")])),
        AgentHarness::ClaudeCode
    );
}

// req:agent-harness/detect@5df05d5b
#[test]
fn codex_inside_claude_code_is_codex() {
    assert_eq!(
        detect(env(&[("CLAUDECODE", "1"), (CODEX_MARKER, "t")])),
        AgentHarness::Codex
    );
    assert_eq!(detect(env(&[(CODEX_MARKER, "t")])), AgentHarness::Codex);
}

// req:agent-harness/detect@5df05d5b
#[test]
fn codex_marker_is_the_variable_codex_sets() {
    assert_eq!(CODEX_MARKER, "CODEX_THREAD_ID");
    assert_eq!(
        detect(env(&[("CODEX_THREAD_ID", "t")])),
        AgentHarness::Codex
    );
}

// req:agent-harness/detect@5df05d5b
#[test]
fn humans_terminal_is_none() {
    assert_eq!(detect(env(&[])), AgentHarness::None);
    assert_eq!(detect(env(&[("CLAUDECODE", "")])), AgentHarness::None);
    assert_eq!(detect(env(&[(CODEX_MARKER, "")])), AgentHarness::None);
}

// req:agent-harness/detect@5df05d5b
#[test]
fn names_round_trip() {
    for h in [
        AgentHarness::ClaudeCode,
        AgentHarness::Codex,
        AgentHarness::None,
    ] {
        assert_eq!(AgentHarness::from_name(h.name()), Some(h));
    }
    assert_eq!(AgentHarness::ClaudeCode.name(), "claude-code");
    assert_eq!(AgentHarness::Codex.name(), "codex");
    assert_eq!(AgentHarness::None.name(), "none");
    assert_eq!(AgentHarness::from_name("claude"), None);
    assert_eq!(AgentHarness::from_name(""), None);
}
