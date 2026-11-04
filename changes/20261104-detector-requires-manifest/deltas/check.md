---
component: check
---

## ADDED Requirements

### Requirement: detector-requires
`clm check` SHALL skip a detector whose `requires` names a path that doesn't exist under the repository root, writing `<name> skipped: no <path>` to its stderr and reporting no `check/detectors` violation for it; a detector whose `requires` exists, or that has none, SHALL run as `detectors` says. A detector command whose first word is a relative path holding a `/` SHALL be found from the repository root, so `["node_modules/.bin/knip"]` runs the root's installed tool, and when nothing is there it is reported as `detectors` reports a command that can't be started (`could not run`, with its hint).

#### Scenario: required file missing
- GIVEN a clean repository with no `package.json` whose `clm.toml` has a detector `knip` (`["sh", "-c", "touch ran; exit 3"]`, `requires = "package.json"`)
- WHEN `clm check` runs
- THEN it exits 0, prints no `check/detectors` line, stderr contains `knip skipped: no package.json`, and no file `ran` exists

#### Scenario: required file present
- GIVEN the same repository holding `package.json`
- WHEN `clm check` runs
- THEN stdout has the line `clm.toml: check/detectors: knip failed (exit 3)`

#### Scenario: relative tool not installed
- GIVEN a clean repository holding `package.json` and no `node_modules/`, whose `clm.toml` has a detector `knip` (`["node_modules/.bin/knip"]`, `requires = "package.json"`, hint `npm ci`)
- WHEN `clm check` runs from a subdirectory
- THEN it exits 1 with a line starting `clm.toml: check/detectors: knip could not run:` and ending `; install with: npm ci`

#### Scenario: relative tool installed
- GIVEN that repository with an executable `node_modules/.bin/knip` that exits 0
- WHEN `clm check` runs from that subdirectory
- THEN no `check/detectors` line is printed

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- config/detector-requires-setting
