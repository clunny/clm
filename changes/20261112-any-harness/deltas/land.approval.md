---
component: land.approval
base: { frontmatter: "211fc8fdb093b4c8" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [ledger, gate, config, vcs, harness]
```

## ADDED Requirements

### Requirement: approve-refuses-unconfirming-harness
`clm approve` SHALL, before it fetches or appends anything, ask `harness`'s `detect` which harness its environment names, and when that harness's `abilities` hold `confirm` false and the harness isn't `None`, SHALL exit 1, appending nothing, with a message naming the harness and `approval-policy` and saying the human approves by running the same `clm approve` command in their own terminal, outside any agent. Under a harness whose `confirm` is true, or `None`, it SHALL go on as `approve` says.

#### Scenario: under Codex
- GIVEN Codex's marker set in `clm approve spec/x`'s environment and a branch with a diff
- WHEN it runs
- THEN it exits 1, the writer's ledger is unchanged, and the message names `codex`, `approval-policy` and `own terminal`

#### Scenario: in the human's terminal
- GIVEN neither harness's marker set
- WHEN `clm approve spec/x` runs on a branch with a diff
- THEN it appends the `approved` record as `approve` says

#### Scenario: under Claude Code
- GIVEN `CLAUDECODE=1` set
- WHEN `clm approve spec/x` runs on a branch with a diff
- THEN it appends the `approved` record, the guard having asked the human first

#### Serves
- capability/approve-intent
- capability/work-from-any-harness
- architecture/approval-policy
- architecture/harness-adapters
