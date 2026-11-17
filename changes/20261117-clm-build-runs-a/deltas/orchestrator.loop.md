---
component: orchestrator.loop
base: {}
---

## ADDED Requirements

### Requirement: runs-the-clm-now-at-its-path
Each time `clm-loop` starts a `clm` subprocess, it SHALL resolve the `clm` it was given (`Options.clm`, the path `std::env::current_exe` gave `clm build`) afresh: when that path ends in ` (deleted)` and names no file, as on Linux once the file the process started from has been replaced or deleted (the dev install renames a new `clm` over the running one whenever `main` moves, `dogfood-from-main`, while `clm build wait` and a step's subprocesses run for minutes), it SHALL run the file at the path without the suffix when one exists, and otherwise the running image at `/proc/self/exe`; any other path it SHALL run as given. So every subprocess a long `clm build` process starts after the replacement runs the `clm` now at the path, as `clm land` copies it (`land/land-binary-copy`), and none fails with `No such file or directory`. The resolution SHALL be a pure function of the path and of which files exist, so a test proves it without replacing a running binary.

#### Scenario: the clm it started from was replaced
- GIVEN a fixture repository, a fake `clm` at `<dir>/clm`, and `Options.clm` set to `<dir>/clm (deleted)`, a path that names no file
- WHEN `clm build step` runs through the library with those options
- THEN the fake `clm` at `<dir>/clm` receives the step's subcommands, and the step succeeds

#### Scenario: the clm it started from was deleted
- GIVEN a path `<dir>/clm (deleted)` where neither it nor `<dir>/clm` names a file, on Linux
- WHEN the loop resolves the `clm` to run
- THEN it runs `/proc/self/exe`

#### Scenario: a path that names a file
- GIVEN a path naming an existing file, whether or not it ends in ` (deleted)`
- WHEN the loop resolves the `clm` to run
- THEN it runs that path unchanged
