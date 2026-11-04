---
title: "The check script validates the panel with Claude Code when it can, and skips when Claude Code is absent"
date: 2026-10-28
status: accepted
---

Claude Code refused `clm-panel`'s hooks module (a type contract that imported another file, a parameter that shadowed the registrar) and nothing noticed: the panel's vitest suite strips types and never loads the module as the engine does. `claude plugin validate plugin-panel` reads the plugin as the engine will and reports what it would refuse, so `ci-check.sh` runs it beside the panel's tests (`ci/panel-validate`).

It needs `claude` on `PATH`. Lands run on the machine of a person or session using Claude Code, so it is there for nearly every land. When it isn't (a person without Claude Code, or a hosted runner added later, since no workflow runs the script now), the script prints that it skipped the step and goes on. A validator nobody can install shouldn't block a land, and the next land from a Claude Code session runs it.

A `tsc` type-check of the panel is left out: `claude plugin validate` doesn't type-check, and the `'claude-code'` API types exist only inside an installed Claude Code, at a path that varies by version and process, with no package to install them from. It is recorded as `backlog/panel-type-check`.

Alternatives considered:
- Failing when `claude` is missing. Stricter, but it would block every land by someone without Claude Code over a check they can't run.
- Installing Claude Code in the script. It would download and run an agent on every land, and a land that installs software is what `unattended-commands` avoids.
- A Rust or Node check of our own for the two refusals seen. It would copy a part of Claude Code's validator and drift from it (`reuse-first`).
