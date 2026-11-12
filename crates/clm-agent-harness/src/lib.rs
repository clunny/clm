//! Which agent harness a `clm` command runs under, what it can do, and how its
//! hook events and decisions are written (`agent-harness-adapters`).

/// The variable Codex sets in every command a session runs (read from Codex's
/// own binary, checked 2026-11-12).
pub const CODEX_MARKER: &str = "CODEX_THREAD_ID";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AgentHarness {
    ClaudeCode,
    Codex,
    None,
}

impl AgentHarness {
    pub fn name(self) -> &'static str {
        match self {
            AgentHarness::ClaudeCode => "claude-code",
            AgentHarness::Codex => "codex",
            AgentHarness::None => "none",
        }
    }

    pub fn from_name(name: &str) -> Option<AgentHarness> {
        [
            AgentHarness::ClaudeCode,
            AgentHarness::Codex,
            AgentHarness::None,
        ]
        .into_iter()
        .find(|h| h.name() == name)
    }
}

/// The agent harness the environment names; Codex wins when both markers are
/// set, being the innermost.
pub fn detect(env: impl Fn(&str) -> Option<String>) -> AgentHarness {
    let set = |k: &str| env(k).is_some_and(|v| !v.is_empty());
    if set(CODEX_MARKER) {
        AgentHarness::Codex
    } else if set("CLAUDECODE") {
        AgentHarness::ClaudeCode
    } else {
        AgentHarness::None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Abilities {
    pub confirm: bool,
    pub guard: bool,
    pub session_hooks: bool,
    pub subagents: bool,
    pub isolated_worktrees: bool,
    pub named_agents: bool,
    pub model_choice: bool,
    pub build_model: Option<&'static str>,
}

pub fn abilities(agent_harness: AgentHarness) -> Abilities {
    let none = Abilities {
        confirm: false,
        guard: false,
        session_hooks: false,
        subagents: false,
        isolated_worktrees: false,
        named_agents: false,
        model_choice: false,
        build_model: None,
    };
    match agent_harness {
        AgentHarness::ClaudeCode => Abilities {
            confirm: true,
            guard: true,
            session_hooks: true,
            subagents: true,
            isolated_worktrees: true,
            named_agents: true,
            model_choice: true,
            build_model: Some("sonnet"),
        },
        AgentHarness::Codex => Abilities {
            guard: true,
            session_hooks: true,
            subagents: true,
            ..none
        },
        AgentHarness::None => none,
    }
}

/// What `clm`'s hooks read from an event; each field is `None` when absent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HookEvent {
    pub kind: Option<String>,
    pub tool: Option<String>,
    pub command: Option<String>,
    pub cwd: Option<String>,
}

/// Both supported agent harnesses name these fields alike.
pub fn parse_event(
    _agent_harness: AgentHarness,
    json: &str,
) -> Result<HookEvent, serde_json::Error> {
    let v: serde_json::Value = serde_json::from_str(json)?;
    let s = |path: &[&str]| {
        path.iter()
            .try_fold(&v, |v, k| v.get(k))
            .and_then(|x| x.as_str())
            .map(str::to_string)
    };
    Ok(HookEvent {
        kind: s(&["hook_event_name"]),
        tool: s(&["tool_name"]),
        command: s(&["tool_input", "command"]),
        cwd: s(&["cwd"]),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    Ask(String),
    Block(String),
}

/// What a hook prints and exits with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub fn render(agent_harness: AgentHarness, decision: &Decision) -> Rendered {
    match decision {
        Decision::Allow => Rendered {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        },
        Decision::Ask(reason) if abilities(agent_harness).confirm => {
            let out = serde_json::json!({"hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "ask",
                "permissionDecisionReason": reason,
            }});
            Rendered {
                stdout: out.to_string(),
                stderr: String::new(),
                exit_code: 0,
            }
        }
        Decision::Ask(reason) => block(&format!(
            "{reason}\nThis agent harness can't ask the human: run the command in the human's own terminal."
        )),
        Decision::Block(reason) => block(reason),
    }
}

fn block(reason: &str) -> Rendered {
    Rendered {
        stdout: String::new(),
        stderr: reason.to_string(),
        exit_code: 2,
    }
}
