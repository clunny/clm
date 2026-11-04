---
component: clm
---

## ADDED Requirements

### Requirement: check-passes-requires
`clm check` SHALL pass each detector's `requires`, as the settings load it (`config/detector-requires-setting`), to `clm-check`'s detectors with the rest of the detector, so a detector whose `requires` is missing is skipped as `check/detector-requires` says.

#### Scenario: requires reaches the check
- GIVEN a clean repository with no `package.json` whose `clm.toml` has a detector `knip` (`["sh", "-c", "touch ran; exit 3"]`, `requires = "package.json"`)
- WHEN the `clm check` command runs
- THEN it exits 0, stderr contains `knip skipped: no package.json`, and no file `ran` exists

#### Serves
- capability/no-dead-weight

#### Needs
- config/detector-requires-setting
- check/detector-requires
