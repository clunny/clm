---
title: Agent harness adapters
status: unbuilt
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-agent-harness/**"]
successor: ""
serves: [capability/work-from-any-agent-harness, architecture/agent-harness-adapters]
---

## Contract
Which agent harness a `clm` command runs under, what that agent harness can do, and how its hook events and decisions are written (`agent-harness-adapters`). Crate `clm-agent-harness`:
- `AgentHarness`: `ClaudeCode`, `Codex` or `None`, the last meaning the command runs outside any agent CLM recognises (the human's terminal, CI, or an agent of the generic tier, which reads `AGENTS.md` and needs no adapter). Each has a name, `claude-code`, `codex` or `none`, and `AgentHarness::from_name` reads one back.
- `detect(env)`: the agent harness the environment names, from its markers; `None` when it names none.
- `abilities(agent_harness)`: an `Abilities` of booleans `confirm` (the agent harness can make the human confirm a shell command before it runs), `guard` (it runs a hook before each shell command that can stop it), `session_hooks` (it runs hooks at a session's start and at the end of each turn), `subagents` (a session can launch subagents), `isolated_worktrees` (a subagent can be given a worktree of its own, which the agent harness makes), `named_agents` (the agent harness loads the plugin's agents and a session can launch one by name) and `model_choice` (a session can name a subagent's model), with `build_model`, the model alias builds launch with when `model_choice` is true.
- `HookEvent`: what `clm`'s hooks read from an event (its kind, the tool and its shell command for a pre-tool event, and its `cwd`), parsed from an agent harness's event JSON by `parse_event(agent_harness, json)`.
- `Decision` rendering: `render(agent_harness, decision)` gives the stdout and the exit code by which an agent harness reads an `Allow`, an `Ask(reason)` or a `Block(reason)`.

`clm agent-harness` (root `clm`'s `agent-harness-command`) prints what `detect` and `abilities` say, for the skills to follow.

## Design
Crate `clm-agent-harness`, at `crates/clm-agent-harness/`, depending on `serde_json` and nothing of CLM's, so the guard, the hooks, `clm-approval` and the CLI can all use it. `env` is a lookup function (`Fn(&str) -> Option<String>`), as `config`'s is, so tests pass a map.

**Markers.** Claude Code sets `CLAUDECODE=1` in every command it runs. Codex's marker is the variable its own source sets in every command a session runs; the build reads it from Codex's source and documentation and names it in this Design and its tests. When both are set (a Codex session started from a Claude Code session's shell), `Codex` wins, being the innermost: an agent harness doesn't set another's marker, but a child inherits its parent's environment. A marker that is set to an empty string counts as unset.

**Abilities**, as each agent harness documents them (checked 2026-11-12):

| | `confirm` | `guard` | `session_hooks` | `subagents` | `isolated_worktrees` | `named_agents` | `model_choice` |
|---|---|---|---|---|---|---|---|
| `claude-code` | yes | yes | yes | yes | yes | yes | yes, `build_model` `sonnet` |
| `codex` | no, until Codex's pre-tool hook is shown to make the human confirm | yes | yes | yes | no | no, until Codex is shown to load a plugin's agents | no |
| `none` | no | no | no | no | no | no | no |

`confirm` stays `false` for an agent harness until its documentation shows a hook can hold a command for the human's yes, since a wrong `true` would let an agent approve (`approval-policy`); a wrong `false` costs only a trip to the terminal. `none` declares nothing, since nothing is known of what runs it.

**Events.** Claude Code's event is `{"hook_event_name", "tool_name", "tool_input": {"command"}, "cwd"}`; Codex's is as its hooks documentation gives it, read for the same fields. A field an event lacks is `None`, and an event that isn't JSON is an error, which each hook handles as its own requirement says.

**One decision type.** `Decision` (`Allow`, `Ask(reason)`, `Block(reason)`) is this crate's; `clm-guard` returns it, so the guard's rules and each agent harness's rendering never hold two copies.

**Rendering.** For `claude-code`: `Allow` is exit 0 with no output; `Ask(reason)` is the `ask` permission decision's JSON on stdout (`{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": <reason>}}`) and exit 0; `Block(reason)` is the reason on stderr and exit 2. For `codex`, as its hooks documentation gives a pre-tool hook's allow and deny. An agent harness whose `confirm` is false renders `Ask` as `Block`, its reason followed by a line saying the agent harness can't ask the human, so the command is to be run in the human's own terminal: an `ask` an agent harness can't show would otherwise let the command through or be dropped.

## Requirements

### Requirement: detect
`clm-agent-harness`'s `detect(env)` SHALL return `ClaudeCode` when `CLAUDECODE` is set to a non-empty value, `Codex` when Codex's marker is set to a non-empty value, `Codex` when both are, and `None` when neither is. `AgentHarness::from_name` SHALL read back each agent harness's name and return nothing for any other string.

#### Scenario: Claude Code
- GIVEN an environment with `CLAUDECODE=1`
- WHEN `detect` runs
- THEN it returns `ClaudeCode`

#### Scenario: Codex inside Claude Code
- GIVEN an environment with `CLAUDECODE=1` and Codex's marker set
- WHEN `detect` runs
- THEN it returns `Codex`

#### Scenario: the human's terminal
- GIVEN an environment with neither marker, or with `CLAUDECODE` set to the empty string
- WHEN `detect` runs
- THEN it returns `None`

### Requirement: abilities
`clm-agent-harness`'s `abilities(agent_harness)` SHALL return, for each agent harness, the abilities this component's Design tabulates: for `ClaudeCode` every ability true and `build_model` `sonnet`; for `Codex` `guard`, `session_hooks` and `subagents` true and `confirm`, `isolated_worktrees`, `named_agents` and `model_choice` false; for `None` every ability false. No agent harness's `confirm` SHALL be true unless its Design row says so.

#### Scenario: Codex can't confirm
- GIVEN `Codex`
- WHEN `abilities` runs
- THEN `confirm` is false and `guard` is true

#### Scenario: Claude Code builds on sonnet
- GIVEN `ClaudeCode`
- WHEN `abilities` runs
- THEN `confirm` and `model_choice` are true and `build_model` is `sonnet`

### Requirement: hook-events
`clm-agent-harness`'s `parse_event(agent_harness, json)` SHALL read each supported agent harness's hook event into a `HookEvent` holding its kind, its tool's name, a shell tool's command and its `cwd`, each `None` when the event lacks it, and SHALL return an error for text that isn't JSON. `render(agent_harness, decision)` SHALL give each agent harness's stdout and exit code for `Allow`, `Ask(reason)` and `Block(reason)` as the Design says, and for an agent harness whose `confirm` is false SHALL render `Ask(reason)` as a block whose reason is the given reason followed by a line saying the agent harness can't ask the human and naming the human's own terminal.

#### Scenario: Claude Code ask
- GIVEN `ClaudeCode` and `Ask("confirm clm approve")`
- WHEN `render` runs
- THEN stdout is the `ask` permission decision's JSON with that reason and the exit code is 0

#### Scenario: Codex ask becomes a block
- GIVEN `Codex` and `Ask("confirm clm approve")`
- WHEN `render` runs
- THEN it is Codex's block, its reason holding `confirm clm approve` and `own terminal`

#### Scenario: a Codex pre-tool event
- GIVEN a Codex pre-tool event for a shell command `git push origin main` in `/r`
- WHEN `parse_event` runs
- THEN the `HookEvent` holds that command and `cwd` `/r`

#### Scenario: not JSON
- GIVEN the text `nope`
- WHEN `parse_event` runs for either agent harness
- THEN it returns an error
