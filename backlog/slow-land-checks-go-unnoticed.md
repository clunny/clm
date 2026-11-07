---
title: "A land check that slows down goes unnoticed, and nothing says which tests to move after the land"
kind: idea
components: [check.health, ledger, land, config]
origin: 20261107-slow-tests-after-land
---

## Intent

`20261107-slow-tests-after-land` lets a project move slow tests out of `[land] check` into `[verify]`, but nothing tells it when to. `[land] check` is one opaque command to CLM: it knows neither how long a land's check takes over time nor which tests take the time. Have `clm health` advise (`health-advises`, never an automatic move) when the land check is slowing lands, and name the tests worth moving after the land.

Raised by the human on 2026-11-07.

## Approaches

- **Whole-check timing.** The ledger's `checked` record also holds how long `[land] check` ran; `clm health` flags a check whose recent runs pass a threshold. Says that lands are slow, not which tests to move.
- **Per-test timing from a report the project names.** As `[health] lcov` names a coverage file, a setting names a JUnit XML report the check writes (most runners can); `clm health` lists the slowest tests as candidates for `[verify]`. Language-agnostic, specific.
- **Both** (leaning): whole-check timing as the trigger, the report for the advice.

## Open questions

- Which report format(s) to read, and where the setting lives.
- What threshold counts as slow: an absolute duration, a share of the check, or growth over recent lands?
- Should a build or `/clm:triage` see the advice, or only `clm health`?
