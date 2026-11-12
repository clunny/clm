---
component: clm
base: { frontmatter: "ebca29b2c812fa99" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [model, check, check.affected, check.unreferenced, check.health, gate, apply, context, vcs, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, orchestrator.runner, config, init, view, view.repo, plugin.hooks, plugin.hooks.guard, ledger, land, host, sync, effort, rename, upgrade, verify, harness]
```

## ADDED Requirements

### Requirement: harness-command
`clm harness` SHALL print `harness <name>`, the name of the harness `harness`'s `detect` finds in its environment, then one line `<ability> yes` or `<ability> no` for each of `confirm`, `guard`, `session-hooks`, `subagents`, `isolated-worktrees` and `model-choice` from `harness`'s `abilities`, in that order, then `build-model <alias>` when `model-choice` is yes, and exit 0. It needs no CLM repository, and `clm --help` SHALL list it.

#### Scenario: under Codex
- GIVEN Codex's marker set and `CLAUDECODE` unset
- WHEN `clm harness` runs outside any repository
- THEN it prints `harness codex`, `confirm no`, `guard yes`, `subagents yes`, `isolated-worktrees no` and `model-choice no`, no `build-model` line, and exits 0

#### Scenario: under Claude Code
- GIVEN `CLAUDECODE=1`
- WHEN `clm harness` runs
- THEN it prints `harness claude-code`, `confirm yes` and `build-model sonnet`

#### Needs
- harness/detect
- harness/abilities

#### Serves
- capability/work-from-any-harness
