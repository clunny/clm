---
status: applied
resolves: [land-s-contract-still-lists-run-check-after-runner-names-its]
---

## Intent

`land`'s Contract still offers `run_check(command, dir, selection, env, log)`, a library call `land/runner-names-its-caller` removed and its build deleted. This change drops that bullet, so the Contract lists only the runner `clm-land` offers, `run_check_as`. Resolves `backlog/land-s-contract-still-lists-run-check-after-runner-names-its`.

## Problem

Change `20261122-the-check-runner-s` ran the runner's rename expand, then contract: `land`'s Contract listed both forms while both existed, with `run_check` "offered only until `runner-names-its-caller` removes it". No delta dropped the entry once the function was gone. The build of `land/runner-names-its-caller` removed `clm_land::run_check` (only `run_check_as` remains in `crates/clm-land/src/runner.rs`), and `clm rebase` archived the change, so the Contract now describes a call that doesn't exist, contradicting `land/runner-names-its-caller`, which states no such call is public. Checked against `origin/main`: no change since a week before the item was filed (2026-11-22) drops the bullet, and no code fix can, since it is spec text.

## Approaches

- **Drop the bullet (chosen).** One line of the Contract goes; the `run_check_as` bullet already says everything a caller needs.
- **Leave it, as a record of history.** A Contract says what the component offers now; history is the archived change's.

## Outcome

`specs/land/SPEC.md`'s Contract loses the `run_check(...)` bullet and nothing else. No requirement, scenario or Design moves, so nothing is pending and nothing is built: the code already matches (`run_check` is gone, and `land/runner-names-its-caller`'s `compile_fail` test proves it). No capability test sees a difference. No term the glossary covers is added.

Edge cases and workarounds known now: none.

## What the requirements no longer state

Nothing: the bullet was Contract text, not a requirement, and it described a call no longer offered.

## Constraints

- `expand-contract`: this is the contract step's last trace in the spec.
- `code-fix-needs-a-written-rule`: no code changes.
