---
component: clm
---

## MODIFIED Requirements

### Requirement: agent-harness-command <!-- base: 8dc1a8d6942468a9 -->
`clm agent-harness` SHALL print `agent harness <name>`, the name of the agent harness `agent-harness`'s `detect` finds in its environment, then one line `<ability> yes` or `<ability> no` for each of `confirm`, `guard`, `session-hooks`, `subagents`, `isolated-worktrees`, `named-agents` and `model-choice` from `agent-harness`'s `abilities`, in that order, then `build-model <alias>` when `model-choice` is yes, and exit 0. It needs no CLM repository, and `clm --help` SHALL list it.

#### Scenario: under Codex
- GIVEN Codex's marker set and `CLAUDECODE` unset
- WHEN `clm agent-harness` runs outside any repository
- THEN it prints `agent harness codex`, `confirm no`, `guard yes`, `subagents yes`, `isolated-worktrees no`, `named-agents no` and `model-choice no`, no `build-model` line, and exits 0

#### Scenario: under Claude Code
- GIVEN `CLAUDECODE=1`
- WHEN `clm agent-harness` runs
- THEN it prints `agent harness claude-code`, `confirm yes` and `build-model <model>`, `<model>` the build model `agent-harness/abilities` gives Claude Code

#### Needs
- agent-harness/detect
- agent-harness/abilities

#### Serves
- capability/work-from-any-agent-harness
