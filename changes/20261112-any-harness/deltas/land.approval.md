---
component: land.approval
base: { frontmatter: "211fc8fdb093b4c8" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [ledger, gate, config, vcs, agent-harness]
```

## ADDED Requirements

### Requirement: approve-refuses-unconfirming-agent-harness
`clm approve` SHALL, before it fetches or appends anything, ask `agent-harness`'s `detect` which agent harness its environment names, and when that agent harness's `abilities` hold `confirm` false and the agent harness isn't `None`, SHALL exit 1, appending nothing, with a message naming the agent harness and `approval-policy` and saying the human approves by running the same `clm approve` command in their own terminal, outside any agent. Under an agent harness whose `confirm` is true, or `None`, it SHALL go on as `approve` says. It SHALL read the markers from the environment lookup `approve` is given, not from the process's environment. The tests, capability tests included, that run `clm approve` SHALL remove both agent harnesses' markers from its environment unless a scenario sets them, so the suite passes from inside any agent harness; this requirement's build makes that change to them.

#### Scenario: under Codex
- GIVEN Codex's marker set in `clm approve spec/x`'s environment and a branch with a diff
- WHEN it runs
- THEN it exits 1, the writer's ledger is unchanged, and the message names `codex`, `approval-policy` and `own terminal`

#### Scenario: in the human's terminal
- GIVEN neither agent harness's marker set
- WHEN `clm approve spec/x` runs on a branch with a diff
- THEN it appends the `approved` record as `approve` says

#### Scenario: under Claude Code
- GIVEN `CLAUDECODE=1` set
- WHEN `clm approve spec/x` runs on a branch with a diff
- THEN it appends the `approved` record, the guard having asked the human first

#### Needs
- agent-harness/detect
- agent-harness/abilities

#### Serves
- capability/approve-intent
- capability/work-from-any-agent-harness
- architecture/approval-policy
- architecture/agent-harness-adapters
