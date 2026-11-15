use clm_agent_harness::{AgentHarness, abilities};

// req:agent-harness/abilities@447faacc
#[test]
fn codex_cannot_confirm_but_guards() {
    let a = abilities(AgentHarness::Codex);
    assert!(!a.confirm);
    assert!(a.guard && a.session_hooks && a.subagents);
    assert!(!a.isolated_worktrees && !a.named_agents && !a.model_choice);
    assert_eq!(a.build_model, None);
}

// req:agent-harness/abilities@447faacc
#[test]
fn claude_code_builds_on_opus() {
    let a = abilities(AgentHarness::ClaudeCode);
    assert!(a.confirm && a.guard && a.session_hooks && a.subagents);
    assert!(a.isolated_worktrees && a.named_agents && a.model_choice);
    assert_eq!(a.build_model, Some("opus"));
}

// req:agent-harness/abilities@447faacc
#[test]
fn none_declares_nothing() {
    let a = abilities(AgentHarness::None);
    assert!(!a.confirm && !a.guard && !a.session_hooks && !a.subagents);
    assert!(!a.isolated_worktrees && !a.named_agents && !a.model_choice);
    assert_eq!(a.build_model, None);
}
