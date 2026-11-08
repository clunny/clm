---
title: "Parallel builds run out of disk in the middle of a land check, and nothing warns before they start"
kind: idea
components: [orchestrator.run, land, config]
---

On 2026-11-07 six parallel builds filled the disk: each build's worktree held its own build output (6 to 9 GB each for this repository), plus the clone's shared land build directory. Four lands then failed with "no space left on device" partway through the check, after their code was written and reviewed. Only `max_builds` limits this, and it counts builds, not disk. Nothing checks free space before a build starts or before a land's check runs, and a failed land reports the disk error only from deep in its log.

## Direction

- **Check before starting.** `clm run` and `clm land` compare free space on the build's filesystem with an estimate, and refuse or wait with a clear message instead of starting a check that will fail. The estimate can come from what earlier builds and lands of the same project used, which CLM can record, so it holds for any toolchain without CLM knowing its output sizes.
- **Name the cause.** A land check that fails with the system's out-of-space error says so in its refusal (`disk full`), not only in the log, so a build stops with that reason and its orchestrator can resume it once space is freed (see `a-stopped-build-is-always-restarted-from-scratch-even-when`).
- **Bound what builds keep.** A build's own build output can be large and is rebuildable. `clm run --finish`, or the land, can offer to clear it once the branch has landed, or a setting can cap it.

## Toolchains

Disk use per build differs by orders of magnitude: a Rust or C++ workspace can keep several GB per worktree, a JavaScript project's dependency install can too, while a Python or Go project may keep little (Go's build cache is shared). So the check must be measured, not assumed, and any clearing of build output must go through the project's own clean command or a declared output path, never a hard-coded `target/`.
