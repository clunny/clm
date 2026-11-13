---
component: clm
base: { frontmatter: "0b39fcec1f075233" }
---

## MODIFIED Frontmatter
```yaml
depends_on: [model, check, check.affected, check.unreferenced, check.health, gate, apply, context, vcs, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, orchestrator.runner, orchestrator.loop, config, init, view, view.repo, plugin.hooks, plugin.hooks.guard, ledger, land, host, sync, effort, rename, upgrade, verify, agent-harness, machine]
```

`crates/clm` dispatches `clm build` to `clm-loop` (the Design), so it imports that crate.
