---
title: Agent harness adapters
status: active
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-agent-harness/**"]
successor: ""
serves: [capability/work-from-any-agent-harness, architecture/agent-harness-adapters]
---

## Contract
Which agent harness a `clm` command runs under, what that agent harness can do, and how its hook events and decisions are written (`agent-harness-adapters`). Crate `clm-agent-harness`:
- `AgentHarness`: `ClaudeCode`, `Codex` or `None`, the last meaning the command runs outside any agent CLM recognises (the human's terminal, CI, or an agent of the generic tier, which reads `AGENTS.md` and needs no adapter). `AgentHarness::name()` gives `claude-code`, `codex` or `none`, and `AgentHarness::from_name` reads one back.
- `detect(env)`: the agent harness the environment names, from its markers; `None` when it names none. `CODEX_MARKER` is the variable `CODEX_THREAD_ID`, by which Codex marks the commands a session runs.
- `abilities(agent_harness)`: an `Abilities` of booleans `confirm` (the agent harness can make the human confirm a shell command before it runs), `guard` (it runs a hook before each shell command that can stop it), `session_hooks` (it runs hooks at a session's start and at the end of each turn), `subagents` (a session can launch subagents), `isolated_worktrees` (a subagent can be given a worktree of its own, which the agent harness makes), `named_agents` (the agent harness loads the plugin's agents and a session can launch one by name) and `model_choice` (a session can name a subagent's model), with `build_model`, the model alias builds launch with when `model_choice` is true.
- `HookEvent`: what `clm`'s hooks read from an event (the tool, its shell command for a pre-tool event, and its `cwd`), parsed from an agent harness's event JSON by `parse_event(agent_harness, json)`.
- `Decision` rendering: `render(agent_harness, decision)` gives a `Rendered`, the stdout, the stderr and the exit code by which an agent harness reads an `Allow`, an `Ask(reason)` or a `Block(reason)`.

`clm agent-harness` (`clm/agent-harness-command`) prints what `detect` and `abilities` say, for the skills to follow.

## Design
Crate `clm-agent-harness`, at `crates/clm-agent-harness/`, depending on `serde_json` and nothing of CLM's, so the guard, the hooks, `clm-approval` and the CLI can all use it. `env` is a lookup function (`Fn(&str) -> Option<String>`), as `config`'s is, so tests pass a map.

**Markers.** Claude Code sets `CLAUDECODE=1` in every command it runs. Codex sets `CODEX_THREAD_ID` in the environment of every command a session runs, since its release `rust-v0.95.0`. Codex is preferred when both are set because an agent harness doesn't set another's marker, but a child inherits its parent's environment: a Codex session started from a Claude Code session's shell is the innermost.

**Abilities.** The values are the requirement's (`abilities`), set from what each agent harness documents and erring toward `false`: Codex can name a subagent's model, which its `model_choice` doesn't say yet (`backlog/codex-can-name-a-subagent-s-model-and-abilities-says-it-can-t`). `confirm` stays `false` for an agent harness until its documentation shows a hook can hold a command for the human's yes, since a wrong `true` would let an agent approve (`approval-policy`); a wrong `false` costs only a trip to the terminal. Codex's pre-tool hook can only allow or deny: it reads an `ask` decision as an error and runs the command, so its `confirm` is false. Its plugin manifest holds no agents, so its `named_agents` is false. `none` declares nothing, since nothing is known of what runs it.

**Events and decisions**, checked against Codex's source and its hooks documentation on 2026-11-19. Both agent harnesses write a pre-tool event with the same keys and name their shell tool `Bash`, and both read a hook that exits 0 with no output as an allow and one that exits 2 with a reason on stderr as a block, so `parse_event` and `render` don't tell them apart there. They differ in `ask` alone, which `render` takes from `abilities`: an `ask` an agent harness can't show would otherwise let the command through or be dropped. What Codex makes of a block with no reason is in the backlog item named above.

**One decision type.** `Decision` (`Allow`, `Ask(reason)`, `Block(reason)`) is this crate's; `clm-guard` returns it, so the guard's rules and each agent harness's rendering never hold two copies.

## Requirements

### Requirement: detect
`clm-agent-harness`'s `detect(env)` SHALL return `ClaudeCode` when `CLAUDECODE` is set to a non-empty value, `Codex` when `CODEX_THREAD_ID`, Codex's marker, is set to a non-empty value, `Codex` when both are, and `None` when neither is. An agent harness's name SHALL be `claude-code`, `codex` or `none`, and `AgentHarness::from_name` SHALL read back each name and return nothing for any other string.

#### Scenario: Claude Code
- GIVEN an environment with `CLAUDECODE=1`
- WHEN `detect` runs
- THEN it returns `ClaudeCode`

#### Scenario: Codex
- GIVEN an environment with only `CODEX_THREAD_ID` set
- WHEN `detect` runs
- THEN it returns `Codex`

#### Scenario: Codex inside Claude Code
- GIVEN an environment with `CLAUDECODE=1` and `CODEX_THREAD_ID` set
- WHEN `detect` runs
- THEN it returns `Codex`

#### Scenario: the human's terminal
- GIVEN an environment with neither marker, or with `CLAUDECODE` or `CODEX_THREAD_ID` set to the empty string
- WHEN `detect` runs
- THEN it returns `None`

#### Scenario: names read back
- GIVEN the strings `claude-code`, `codex`, `none`, `claude` and the empty string
- WHEN `AgentHarness::from_name` reads each
- THEN it returns the agent harness of that name for the first three, whose `name()` is that string, and nothing for the other two

### Requirement: abilities
`clm-agent-harness`'s `abilities(agent_harness)` SHALL return, for `ClaudeCode`, every ability true and `build_model` `opus`; for `Codex`, `guard`, `session_hooks` and `subagents` true and `confirm`, `isolated_worktrees`, `named_agents` and `model_choice` false; and for `None`, every ability false.

#### Scenario: Codex can't confirm
- GIVEN `Codex`
- WHEN `abilities` runs
- THEN `confirm` is false and `guard` is true

#### Scenario: Claude Code builds on opus
- GIVEN `ClaudeCode`
- WHEN `abilities` runs
- THEN `confirm` and `model_choice` are true and `build_model` is `opus`

#### Scenario: no agent harness declares nothing
- GIVEN `None`
- WHEN `abilities` runs
- THEN every ability is false

### Requirement: hook-events
`clm-agent-harness`'s `parse_event(agent_harness, json)` SHALL read each supported agent harness's hook event into a `HookEvent` holding its tool from the key `tool_name`, a shell tool's command from `tool_input.command` and its `cwd` from `cwd`, each `None` when the event lacks it or holds something other than a string there, and SHALL return an error for text that isn't JSON. `render(agent_harness, decision)` SHALL give, for each supported agent harness, `Allow` as empty stdout, empty stderr and exit code 0, and `Block(reason)` as the reason on stderr, empty stdout and exit code 2, an empty reason giving empty stderr with that exit code; for an agent harness whose `confirm` is true, `Ask(reason)` as the JSON `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": <reason>}}` on stdout, empty stderr and exit code 0; and for an agent harness whose `confirm` is false, `Ask(reason)` as the block whose reason is the given reason, a line break, and a line saying the agent harness can't ask the human and naming the human's own terminal.

#### Scenario: Claude Code ask
- GIVEN `ClaudeCode` and `Ask("confirm clm approve")`
- WHEN `render` runs
- THEN stdout is the `ask` permission decision's JSON with that reason and the exit code is 0

#### Scenario: Codex ask becomes a block
- GIVEN `Codex` and `Ask("confirm clm approve")`
- WHEN `render` runs
- THEN it is Codex's block, its reason holding `confirm clm approve` and `own terminal`

#### Scenario: the reason comes first
- GIVEN `Codex` or `None`, and `Ask("confirm clm approve")`
- WHEN `render` runs
- THEN stderr starts with `confirm clm approve` and a line break, stdout is empty and the exit code is 2

#### Scenario: allow
- GIVEN `ClaudeCode` or `Codex`, and `Allow`
- WHEN `render` runs
- THEN stdout and stderr are empty and the exit code is 0

#### Scenario: block
- GIVEN `ClaudeCode` or `Codex`, and `Block("no")`
- WHEN `render` runs
- THEN stderr is `no`, stdout is empty and the exit code is 2

#### Scenario: a Codex pre-tool event
- GIVEN a Codex pre-tool event for a shell command `git push origin main` in `/r`
- WHEN `parse_event` runs
- THEN the `HookEvent` holds that command and `cwd` `/r`

#### Scenario: an event lacking fields
- GIVEN the event `{"hook_event_name":"SessionStart"}`
- WHEN `parse_event` runs for `Codex`
- THEN the `HookEvent`'s tool, command and `cwd` are each `None`

#### Scenario: not JSON
- GIVEN the text `nope`
- WHEN `parse_event` runs for either agent harness
- THEN it returns an error
