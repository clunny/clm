use clm_agent_harness::{AgentHarness, Decision, parse_event, render};

// req:agent-harness/hook-events@cab71a98
#[test]
fn claude_code_ask_is_permission_json() {
    let r = render(
        AgentHarness::ClaudeCode,
        &Decision::Ask("confirm clm approve".into()),
    );
    assert_eq!(r.exit_code, 0);
    let v: serde_json::Value = serde_json::from_str(&r.stdout).unwrap();
    let o = &v["hookSpecificOutput"];
    assert_eq!(o["hookEventName"], "PreToolUse");
    assert_eq!(o["permissionDecision"], "ask");
    assert_eq!(o["permissionDecisionReason"], "confirm clm approve");
}

// req:agent-harness/hook-events@cab71a98
#[test]
fn allow_and_block() {
    for h in [AgentHarness::ClaudeCode, AgentHarness::Codex] {
        let a = render(h, &Decision::Allow);
        assert_eq!(
            (a.stdout.as_str(), a.stderr.as_str(), a.exit_code),
            ("", "", 0)
        );
        let b = render(h, &Decision::Block("no".into()));
        assert_eq!(b.exit_code, 2);
        assert!(b.stderr.contains("no"));
        assert_eq!(b.stdout, "");
    }
}

// req:agent-harness/hook-events@cab71a98
#[test]
fn codex_ask_becomes_block() {
    let r = render(
        AgentHarness::Codex,
        &Decision::Ask("confirm clm approve".into()),
    );
    assert_eq!(r.exit_code, 2);
    assert!(r.stderr.contains("confirm clm approve"));
    assert!(r.stderr.contains("own terminal"));
    assert!(r.stderr.contains("can't ask the human"));
    let n = render(AgentHarness::None, &Decision::Ask("x".into()));
    assert_eq!(n.exit_code, 2);
    assert!(n.stderr.contains("own terminal"));
}

// req:agent-harness/hook-events@cab71a98
#[test]
fn codex_pre_tool_event() {
    let j = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"git push origin main"},"cwd":"/r"}"#;
    for h in [AgentHarness::Codex, AgentHarness::ClaudeCode] {
        let e = parse_event(h, j).unwrap();
        assert_eq!(e.kind.as_deref(), Some("PreToolUse"));
        assert_eq!(e.tool.as_deref(), Some("Bash"));
        assert_eq!(e.command.as_deref(), Some("git push origin main"));
        assert_eq!(e.cwd.as_deref(), Some("/r"));
    }
}

// req:agent-harness/hook-events@cab71a98
#[test]
fn missing_fields_are_none_and_non_json_errors() {
    let e = parse_event(AgentHarness::Codex, r#"{"hook_event_name":"SessionStart"}"#).unwrap();
    assert_eq!((e.tool, e.command, e.cwd), (None, None, None));
    for h in [AgentHarness::Codex, AgentHarness::ClaudeCode] {
        assert!(parse_event(h, "nope").is_err());
    }
}
