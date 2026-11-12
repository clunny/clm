---
component: clm
base: { frontmatter: "7d3db585322aa546" }
---

## MODIFIED Frontmatter
```yaml
title: CLM
status: active
owners: ["@clunny"]
depends_on: [model, check, check.affected, check.unreferenced, check.health, gate, apply, context, vcs, orchestrator, orchestrator.run, orchestrator.checkpoint, orchestrator.triage, orchestrator.runner, config, init, view, view.repo, plugin.hooks, plugin.hooks.guard, ledger, land, host, sync, effort, rename, upgrade, verify, agent-harness, machine]
paths: ["Cargo.toml", "Cargo.lock", "crates/clm/Cargo.toml", "crates/clm/build.rs", "crates/clm/src/**", "crates/clm/tests/*.rs", "rust-toolchain.toml", ".claude/settings.json", ".gitattributes", "scripts/dev-install.sh", "scripts/retired-terms.sh", "scripts/retired-terms.txt", "README.md", "CONTRIBUTING.md", "LICENSE"]
outside: []
serves: [capability/install-and-release, architecture/platform]
```

## ADDED Requirements

### Requirement: machine-command
`clm` SHALL dispatch `clm machine [--format text|json]` to `clm-machine`, for the CLM repository found from the current directory (`root-from-cwd`), passing `config`'s `max_builds` and the claim expiry `clm next` uses, and `clm --help` SHALL list it.

#### Scenario: dispatch
- GIVEN a repository `clm init` has just set up, with `max_builds` 2 in `clm.local.toml`
- WHEN `clm machine --format json` runs in a subdirectory of it
- THEN it exits 0 and its `disk.free_places` is 2

#### Needs
- machine/machine-report

#### Serves
- capability/watch-machine-load
