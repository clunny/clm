---
component: agent-harness
base: { design: "0080640283b86101" }
---

## MODIFIED Design

Crate `clm-agent-harness`, at `crates/clm-agent-harness/`, depending on `serde_json` and nothing of CLM's, so the guard, the hooks, `clm-approval` and the CLI can all use it. `env` is a lookup function (`Fn(&str) -> Option<String>`), as `config`'s is, so tests pass a map.

**Markers.** Claude Code sets `CLAUDECODE=1` in every command it runs. Codex's marker is the variable its own source sets in every command a session runs; the build reads it from Codex's source and documentation and names it in this Design and its tests. When both are set (a Codex session started from a Claude Code session's shell), `Codex` wins, being the innermost: an agent harness doesn't set another's marker, but a child inherits its parent's environment. A marker that is set to an empty string counts as unset.

**Abilities**, as each agent harness documents them (checked 2026-11-12):

| | `confirm` | `guard` | `session_hooks` | `subagents` | `isolated_worktrees` | `named_agents` | `model_choice` |
|---|---|---|---|---|---|---|---|
| `claude-code` | yes | yes | yes | yes | yes | yes | yes, `build_model` `opus` |
| `codex` | no, until Codex's pre-tool hook is shown to make the human confirm | yes | yes | yes | no | no, until Codex is shown to load a plugin's agents | no |
| `none` | no | no | no | no | no | no | no |

`confirm` stays `false` for an agent harness until its documentation shows a hook can hold a command for the human's yes, since a wrong `true` would let an agent approve (`approval-policy`); a wrong `false` costs only a trip to the terminal. `none` declares nothing, since nothing is known of what runs it.

**Events.** Claude Code's event is `{"hook_event_name", "tool_name", "tool_input": {"command"}, "cwd"}`; Codex's is as its hooks documentation gives it, read for the same fields. A field an event lacks is `None`, and an event that isn't JSON is an error, which each hook handles as its own requirement says.

**One decision type.** `Decision` (`Allow`, `Ask(reason)`, `Block(reason)`) is this crate's; `clm-guard` returns it, so the guard's rules and each agent harness's rendering never hold two copies.

**Rendering.** For `claude-code`: `Allow` is exit 0 with no output; `Ask(reason)` is the `ask` permission decision's JSON on stdout (`{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": <reason>}}`) and exit 0; `Block(reason)` is the reason on stderr and exit 2. For `codex`, as its hooks documentation gives a pre-tool hook's allow and deny. An agent harness whose `confirm` is false renders `Ask` as `Block`, its reason followed by a line saying the agent harness can't ask the human, so the command is to be run in the human's own terminal: an `ask` an agent harness can't show would otherwise let the command through or be dropped.

## MODIFIED Requirements

### Requirement: abilities <!-- base: a558ddae441557a1 -->
`clm-agent-harness`'s `abilities(agent_harness)` SHALL return, for each agent harness, the abilities this component's Design tabulates: for `ClaudeCode` every ability true and `build_model` `opus`; for `Codex` `guard`, `session_hooks` and `subagents` true and `confirm`, `isolated_worktrees`, `named_agents` and `model_choice` false; for `None` every ability false. No agent harness's `confirm` SHALL be true unless its Design row says so.

#### Scenario: Codex can't confirm
- GIVEN `Codex`
- WHEN `abilities` runs
- THEN `confirm` is false and `guard` is true

#### Scenario: Claude Code builds on opus
- GIVEN `ClaudeCode`
- WHEN `abilities` runs
- THEN `confirm` and `model_choice` are true and `build_model` is `opus`
