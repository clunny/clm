---
status: applied
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, its third change of four: one route to coverage. Twelve crates still reach `check.coverage`'s library through `clm-check`'s re-exports while five import `clm-coverage`, and both Contracts state the two routes. This change makes the direction one: every reader depends on `check.coverage` and imports its crate. It is the expand step, and `20261115-check-passes-on-no-coverage`, left proposed beside it, is the contract step that removes the re-exports.

It adds and removes no requirement, so it is archived in its own branch.

## Problem

Measured on `main` at `4f96740b`: `clm`, `clm-affected`, `clm-checkpoint`, `clm-effort`, `clm-health`, `clm-land`, `clm-orchestrator`, `clm-replica`, `clm-run`, `clm-sync`, `clm-view` and `clm-view-repo` name a re-exported item as `clm_check::...`; `clm-affected`, `clm-gate`, `clm-health`, `clm-rename` and `clm-verify` import `clm-coverage`. A build of the twelve reads `check`'s Contract for calls the child's Contract describes, and three requirements exist partly to pass functions through (`check/path-ownership`, `check/coverage-lists`, `check.coverage/state-names`).

## Approaches

- **Expand now, contract later** (`contract-step-deferred`). Taken. The ten components that lack it gain `check.coverage` in `depends_on`, the Contracts say where the imports go, and a `deferred` item holds the twelve moves, which change nothing a requirement states. The removal of the re-exports is a second change, left proposed, with a `deferred` item naming what it waits on.
- **A requirement per crate for its import.** Not taken: an import path is not behaviour, and the plan's phase 5 removes requirements of that kind.
- **One code branch that changes all thirteen crates.** Not taken: a build changes its own component's files (`path-ownership`).

## Outcome

- `clm`, `orchestrator.checkpoint`, `effort`, `land`, `orchestrator`, `replica`, `orchestrator.run`, `sync`, `view` and `view.repo` depend on `check.coverage`.
- The Contracts of `check` and `check.coverage` state one direction and say what remains.
- `backlog/twelve-crates-reach-coverage-through-clm-check-s-re-exports`: the moves. Due now.

## What it removes

Nothing.

## Constraints

No file, key or format an adopter holds changes.
