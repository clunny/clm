---
status: applied
---

## Intent

The line `clm affected` prints when its settings can't be loaded names the requirement that states that exit. `20261118-affected-says-settings-fail` put the exit in `check.affected/affected-base` and left the line labelled `check.affected/affected-all`, recording it as an open question because a change of label costs two builds. The human said on 2026-11-18 that cost is no reason to leave a wrong label: a violation carries the ID of the requirement it enforces (`component-words`), as `clm size` labels the same failure `check/node-size`, which states it.

The builder check then found the same gap one command over. `clm check` labels its settings failure `check/detectors`, and `check_reports_settings_errors_and_still_runs_rules` in `crates/clm-check/tests/cli.rs` cites that requirement, but the requirement never said what `clm check` does when the settings can't be loaded: only `check`'s Design did. `check/detectors` now states it, with a scenario written from that test, so the label names a rule that states what the line reports and the test proves what it cites.

## Approaches

Label the line `check.affected/affected-base` (taken): the requirement that states the exit. Move the clause to `affected-all` so the old label is right: that requirement states what a glob selects, and the exits of 1 are all in `affected-base`. Leave it: the line then names a rule that doesn't state what it reports.

## Outcome

- `clm/routes-only`'s scenario for the settings a routed command is passed names `check.affected/affected-base`. The root's build changes the rule it passes for `clm affected`'s settings failure in `crates/clm/src/main.rs` and the two lines of `crates/clm/tests/routes.rs` that look for the label.
- `check.affected`'s Design says which rule the line carries. Its build, already pending for `affected-base`, words the stand-in loader in `crates/clm-affected/tests/command.rs` the same way.

- `check/detectors` gains the clause and the scenario `settings that can't be loaded`. `check`'s build re-cites its tests; the code already does it.

## Constraints

The label is in a line printed to standard error on a failure; no file, key or format an adopter holds changes.
