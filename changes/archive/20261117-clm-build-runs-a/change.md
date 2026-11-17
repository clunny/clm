---
status: done
resolves: [clm-build-runs-a-deleted-clm-path]
---

## Intent

`clm build` keeps working after the dev install replaces the `clm` it was started from. Each `clm` subprocess the build loop starts runs the `clm` now at the path the loop was started from, the path without Linux's ` (deleted)` suffix, as `clm land` already does for its copy, so a long `clm build wait` or step no longer fails every subcommand with `No such file or directory` once `main` moves.

## Problem

`clm build` hands `clm_loop::Options.clm` the path `std::env::current_exe()` gives (`crates/clm/src/main.rs`), and `crates/clm-loop/src/cmd.rs` spawns every `clm` subcommand from it. On Linux that path is read from `/proc/self/exe`, whose text gains ` (deleted)` once the file the process started from is renamed over or removed. The dev install rebuilds and renames `clm` whenever `origin/main` moves (`dogfood-from-main`), so a `clm build wait` that outlives a land, or a step whose `clm run` or `clm land` subprocess lands something, spawns the rest of its subcommands from a path that names no file, and each fails. No requirement of `orchestrator.loop` states which `clm` the loop runs, so the fix is a requirement this change adds (`code-fix-needs-a-written-rule`).

No change or code fix since 2026-11-10 fixes it: `20261117-clm-verify-can-t` gave `clm land` the same resolution (`land/land-binary-copy`), privately in `crates/clm-land/src/exe.rs`, and nothing in `crates/clm-loop` resolves the path.

## Approaches

- **Taken: the loop resolves the path itself, at each spawn.** The design is right: the loop runs its own `clm` through the commands' Contracts, which keeps every subcommand on the loop's own build. Only the path goes wrong once the file is replaced, and the file now at the cut path is the one `dogfood-from-main` wants run, `main`'s newer `clm`. Resolving at each spawn, not once at start, is what the bug needs, since the replacement happens while the process runs. The running image at `/proc/self/exe` covers a file deleted with no replacement, as `land-binary-copy` does.
- **Not taken: sharing `clm-land`'s private `source` function.** `clm-loop` drives `clm land` as a subprocess and has no library dependency on `clm-land`; taking one for a dozen lines would pull the whole land crate into the loop. Moving the function to a crate both use would need a home that fits (`clm-git` is git plumbing, `clm-orchestrator` the work queue), a requirement there and a requirement moving `land` onto it: three components' builds for one Linux quirk. Two copies of a dozen lines is the smaller cost (`yagni`); `reuse-first` asks for an existing tool or library first, and none does this.
- **Not taken: `crates/clm` resolving the path once before handing it to the loop.** It would still go stale when the file is replaced during the process's life, which is the reported case.
- **Not taken: running `clm` from `PATH` or `CLM_BIN`.** It could run a different build from the loop's own, and the loop's Design runs its own executable on purpose.

## Outcome

- `orchestrator.loop/runs-the-clm-now-at-its-path` (new): every `clm` subprocess the loop starts resolves `Options.clm` afresh; a path ending in ` (deleted)` that names no file runs the file at the path without the suffix, or `/proc/self/exe` when there is none.
- The code is in `crates/clm-loop/` alone; `crates/clm/src/main.rs` keeps passing `current_exe`.

## Edge cases and workarounds

- A `clm` really installed under a name ending in ` (deleted)` names a file, so it runs as given; only a path that names no file is cut.
- Off Linux the path never gains the suffix, so the resolution changes nothing there.

## Open questions

- The root's Design has crate code read the process's executable path through the call's arguments. The fallback runs the fixed path `/proc/self/exe`, which `clm-loop` names itself rather than taking from `crates/clm`. The change leaves it so, as `clm-land` already does for `land/land-binary-copy`: it is a constant Linux path, not a reading of the process's state, and passing it in would edit the root's code for nothing a test could tell apart.
