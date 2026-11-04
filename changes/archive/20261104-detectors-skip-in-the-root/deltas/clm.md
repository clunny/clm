---
component: clm
---

## REMOVED Requirements

### Requirement: check-passes-requires <!-- base: e839a9352a02221d -->

## ADDED Requirements

### Requirement: detector-requires-skip
The `clm check` command SHALL leave out of the detectors it passes to `clm-check` each detector whose `requires`, as the settings load it (`config/detector-requires-setting`), names a path that doesn't exist under the repository root, writing `<name> skipped: no <path>` to its stderr and reporting no `check/detectors` violation for it; a detector whose `requires` exists, or that has none, SHALL be passed and run as `check/detectors` says. `clm-check`'s `Detector` doesn't change: the skip is the root's, which owns the call (`architecture/path-ownership`).

#### Scenario: required file missing
- GIVEN a clean repository with no `package.json` whose `clm.toml` has a detector `knip` (`["sh", "-c", "touch ran; exit 3"]`, `requires = "package.json"`)
- WHEN the `clm check` command runs
- THEN it exits 0, prints no `check/detectors` line, stderr contains `knip skipped: no package.json`, and no file `ran` exists

#### Scenario: required file present
- GIVEN the same repository holding `package.json`
- WHEN the `clm check` command runs
- THEN stdout has the line `clm.toml: check/detectors: knip failed (exit 3)`

#### Serves
- capability/adopt-clm

#### Needs
- config/detector-requires-setting
