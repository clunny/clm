---
status: done
resolves: [config-design-lists-too-few-verify-keys]
---

## Intent

Bring `config`'s Design in line with the settings its requirements make this repository's `clm.toml` set. The paragraph on this repository's `clm.toml` gives `[verify]` only `check`, `mode`, `triggers` and `every`, while `config/verify-here` also requires `mutate = ".github/scripts/mutate.sh"`, `mutate_jobs = 6` and `mutate_minutes = 60`; and it gives `[health] lcov = "target/lcov.info"`, while `config/health-lcov-here` requires `lcov = ".clm/verify-build/lcov.info"`. The change rewrites those two clauses to say what the requirements say, and the file already sets (`backlog/config-design-lists-too-few-verify-keys`).

On the backlog, it deletes that item, and corrects the plan item `a-passing-check-proves-less-than-the-spec-claims`, which still said the mutation pass waited on `config/verify-here`: `impl/config` built it (`524d27ac`) and its change is archived.

## Problem

The Design restates what this repository's `clm.toml` sets so a reader of the component sees each value and why. Two requirements changed what the file sets (`20261114-proof-switched-on` moved `lcov`, `20261116-a-mutation-pass-that-stops` added the mutation keys) and neither change touched the Design paragraph, so the Design and the requirements disagree. The code review of `impl/config`'s `verify-here` build found the `[verify]` half; the `lcov` half is the same paragraph's same fault, found while drafting.

## Approaches

- **Correct the paragraph** (taken). Two clauses change, no requirement, scenario or code does: the file and its tests already hold what the requirements state, so nothing is pending.
- **Drop the list of values from the Design**, leaving it to the requirements. Not taken: the paragraph also carries the reasons (`[affected] all`, the `npm ci` list, the detectors' hints) that no requirement states, and cutting it is a larger edit than the defect asks.

## Outcome

`specs/config/SPEC.md`'s Design names `[verify]`'s seven keys and `[health]`'s `lcov` as `verify-here` and `health-lcov-here` state them. Nothing is built: no requirement is added, modified or removed, and the repository's `clm.toml` already sets these values.

## Constraints

- No edge cases or workarounds.
