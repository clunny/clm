---
component: check
---

## MODIFIED Requirements

### Requirement: detectors <!-- base: 8bf99b6d0cdd02e7 -->
`clm check` SHALL run each detector in the repository's `clm.toml` from the repository root, copying its stdout and stderr to its own stderr as they arrive, and report a violation `clm.toml: check/detectors: <name> failed (exit <code>)` for each that exits non-zero. A detector still running when its `timeout` has passed SHALL be stopped, together with the processes it started, and reported as `clm.toml: check/detectors: <name> timed out after <n>s`, and `clm check` SHALL go on to the next detector. A detector whose command can't be started SHALL be reported as `clm.toml: check/detectors: <name> could not run: <error>`, followed by `; install with: <hint>` when it has a hint. `run` SHALL NOT run detectors. `clm check --no-detectors` SHALL run every other rule as `clm check` does, but run no detector and report no `check/detectors` violation, so it needs neither the detectors' tools nor loadable settings. When its caller can't load the settings, `clm check` SHALL report the violation the caller gives for the failure, run no detector, and still report every other rule's violations.

#### Scenario: failing detector
- GIVEN a `clm.toml` with detectors `ok` (`["true"]`) and `bad` (`["sh", "-c", "echo unused-dep; exit 3"]`)
- WHEN `clm check` runs
- THEN stderr contains `unused-dep`, stdout has the line `clm.toml: check/detectors: bad failed (exit 3)` and nothing for `ok`, and the exit code is 1

#### Scenario: hanging detector
- GIVEN a `clm.toml` with a detector `hang` (`["sh", "-c", "echo started; sleep 30 & echo $! > child.pid; wait"]`, `timeout = 1`) followed by a detector `bad` (`["sh", "-c", "exit 3"]`)
- WHEN `clm check` runs
- THEN it exits 1 well before 30 seconds, stderr contains `started`, stdout has the lines `clm.toml: check/detectors: hang timed out after 1s` and `clm.toml: check/detectors: bad failed (exit 3)`, and the process whose ID `child.pid` holds is no longer running

#### Scenario: tool not installed
- GIVEN a detector `machete` with command `["no-such-tool-xyz"]` and hint `cargo install cargo-machete`
- WHEN `clm check` runs
- THEN it exits 1 with a line starting `clm.toml: check/detectors: machete could not run:` and ending `; install with: cargo install cargo-machete`

#### Scenario: run skips detectors
- GIVEN the same failing `clm.toml`
- WHEN `clm_check::run` is called on the repository
- THEN it returns no `check/detectors` violation

#### Scenario: no detectors
- GIVEN a repository whose `clm.toml` names no detectors, or that has no `clm.toml`
- WHEN `clm check` runs
- THEN no `check/detectors` violation is reported

#### Scenario: detectors skipped on request
- GIVEN a clean repository whose `clm.toml` has a detector `bad` (`["sh", "-c", "touch ran; exit 3"]`)
- WHEN `clm check --no-detectors` runs
- THEN it exits 0, prints no `check/detectors` line, and no file `ran` exists

#### Scenario: settings that can't be loaded
- GIVEN a repository with a component spec that doesn't parse and a `clm.toml` whose detector `bad` exits 3, and a caller whose loading of the settings fails with a violation
- WHEN `clm-check`'s call for `clm check` runs
- THEN it exits 1 and prints that violation and the spec's, one line each, and no `bad failed` line

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- config/detector-timeout-setting
