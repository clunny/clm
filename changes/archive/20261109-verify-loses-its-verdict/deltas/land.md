---
component: land
---

## MODIFIED Requirements
### Requirement: shared-check-runner <!-- base: 235e251982267964 -->
`clm-land` SHALL offer as a library call the runner that `clm land` and `clm land --check-only` run `[land] check` through: it takes the command, the directory to run it in, an optional selection file for `CLM_SELECTION`, the environment to set and unset, and the log file's path, runs the command through `sh -c` under the clone's check lock (`check-lock`), waiting for it with an ordinary ticket and printing its waiting line as `lock-queue` says when it waits, copies its output to stderr and the log as `land-check-log` says, and returns whether it passed and the command's exit code (none when a signal ended it), with the last 40 lines, whether the log holds the whole output, and the step open when its output ended, if any, with that step's name and first and last line numbers as `check-log` counts them, leaving the log for the caller to keep or remove. `clm land` and `--check-only` SHALL call it with the environment they set today, so their behaviour is unchanged apart from `check-log`'s step in a land's refusal. A write to stderr that fails (its waiting line, its copy of the command's output, its warning about the log) SHALL neither end the run nor change what it returns: the runner goes on reading the command's output until the pipe closes and returns its verdict as for any other run.

#### Scenario: a caller waits on the check lock
- GIVEN a command that writes `ok` and exits 0, and `.clm/check.lock` held by another process that releases it after 1 second
- WHEN the runner runs the command
- THEN it printed `waiting for another check` ending `: 1 ahead`, ran the command after the lock was released, wrote `ok` to the log, and returned a pass

#### Scenario: land unchanged
- GIVEN a `[land] check` that fails printing 50 numbered lines
- WHEN `clm land` runs
- THEN its refusal holds lines 11 to 50 and the log's path, as `land-check-log` says

#### Scenario: the open step returned
- GIVEN a command that prints `::group::a`, a line, `::group::b` and two lines, and exits 1
- WHEN the runner runs the command
- THEN it returns a failure with the step `b`, lines 3 to 5

#### Scenario: the exit code returned
- GIVEN a command that exits 127
- WHEN the runner runs the command
- THEN it returns a failure with exit code 127

#### Scenario: a failing stderr ends nothing
- GIVEN Linux, a command that prints 1000 numbered lines and exits 1, a log path whose directory can't be created, `.clm/check.lock` held by another process that releases it after 1 second, and the runner's stderr opened on `/dev/full`
- WHEN the runner runs the command
- THEN it returns a failure with exit code 1 and the last 40 of those lines, having run the command to its end

#### Serves
- capability/slow-tests-after-land
