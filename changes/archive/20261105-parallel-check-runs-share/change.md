---
status: done
resolves: [parallel-check-runs-share-host-resources]
---

# Check runs on one clone don't fail each other

## Intent

Builds and drafts that CLM runs together on one clone must not fail each other's checks through host resources their tests share, such as a fixed local port. Every run of the project's `[land] check`, by `clm land` or `clm land --check-only`, takes a per-clone lock on `.clm/check.lock` while the command runs, so no two checks of a clone overlap (`check-runs-one-at-a-time`).

## Problem

- `clm land` serialises its checks on `.clm/land.lock` only when the check compiles into the shared `.clm/land` (`land-target-lock`); a land whose caller set `CARGO_TARGET_DIR` takes no lock.
- `clm land --check-only`, which every build and every draft with a hand edit runs before landing, takes no lock at all, so it overlaps other check-only runs and lands.
- A project whose tests bind a fixed local port (notespeed's browser tests) then fails in whichever check starts second, for a reason unrelated to its branch: the build retries until the port is free or spends its stop budget and leaves its work unlanded.

## Approaches

- **One check at a time per clone (chosen).** A lock of its own, held around the check command alone, by every land and every check-only run. Covers every shared resource, not just ports, with no help from the project. Costs parallelism of the check command only; lands are already serialised.
- **Widen `land.lock` to check-only runs.** Rejected: that lock spans the gate, the push and the cap and decides whether a land uses the land tree; a check-only run would wait on all of it, and lands on a caller's target would still overlap.
- **A port or slot variable per run.** Rejected: works only for projects that read it, and only for ports.
- **A warning in the method.** Rejected: leaves the failure in place for every adopter.

## Outcome

- `land/check-lock` (added): every `[land] check` run holds `.clm/check.lock` in the main worktree's CLM root, says once on stderr when it waits, and a land takes it after `land.lock`.
- `land/land-tree` (modified): `.clm/check.lock` joins what a land keeps under `.clm/`.
- `land`'s Contract names the check lock and the wait; its Design describes the lock under **Places** and **`--check-only`**, with the exact waiting line, exit 2 for a lock that can't be taken, and the land lock (not the check lock) as the one `clm affected` runs under.
- The retry-budget point of the item needs no rule: with no overlapping checks, another run's check can't fail a build's.
- `orchestrator.run`, `orchestrator.triage` and `plugin` get no delta: their prompts already run the check as `clm land --check-only`, tell the session to wait for a background run's completion notice however long it takes, and forbid worktree changes while it runs, a window that now includes the wait (see Open questions for the foreground case).

## Constraints

- Lock order: `land.lock` before `check.lock`, and `--check-only` takes only `check.lock`, so no cycle.
- `clm check`, `clm affected` and the gate stay outside the lock.

## Edge cases and workarounds

- Tests a session runs by hand (`cargo test`, a part of the check) take no lock and can still collide with a running check. Accepted: the prompts already say such parts are never reported as the check, and the check that counts is serialised.
- A check run by a human or another tool outside `clm` isn't covered.

## Open questions

- Serialising check-only runs lengthens a batch of parallel builds by their checks' total time. If that shows as a bottleneck, a `[land]` setting to opt out for projects whose tests share nothing can follow (`yagni` for now).
- `orchestrator.run`'s prompt allows `clm land --check-only` in the foreground or the background (architecture audit). A foreground run now also waits behind other checks of the clone and could meet the tool's own time limit. Left: a full check already often runs longer than that limit, and Claude Code moves an overrunning command to the background rather than killing it, where the prompt's waiting rule applies; if builds show otherwise, the prompt should say to run it in the background.
- A long `--check-only` run makes a land holding `.clm/land.lock` wait, and every other land behind it (builder check). Accepted as the cost the decision states.
