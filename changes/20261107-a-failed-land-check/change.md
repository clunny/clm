---
status: applied
resolves: [a-failed-land-check-s-refusal-shows-a-tail-of-output-that-may]
---

## Intent

Make a failed `[land] check`'s refusal say where the failure is, not only repeat a fixed tail of the output. The check marks its steps in its output with GitHub Actions' group lines (`::group::<name>`, `::endgroup::`). When a step is open as a failing check ends, the refusal names that step and the lines of the kept log it spans, so a session reads the failing step's own message even when it lies more than 40 lines back. A check that marks no steps gets the refusal it gets today.

## Problem

`clm land` repeats the last 40 lines of a failing check and keeps the whole output at `.clm/land-logs/<branch>.log` (`land-check-log`). In a project whose check runs several steps (an install, unit tests, a browser end-to-end suite), the output after the failing step's own message can run past 40 lines, so the tail held no reason. The session reran the suite separately to tell a flake from a real failure (GitHub issue #262, `backlog/a-failed-land-check-s-refusal-shows-a-tail-of-output-that-may`). The log held the reason, but nothing told the session where in a log of thousands of lines to look.

`land-check-log` rejected parsing the output for the failing stage: the check is the project's own script, and its stages vary. That reasoning still holds for CLM guessing the stages. It doesn't hold when the script names them.

## Approaches

- *A longer tail.* Rejected: one step's output can outrun any fixed count, and a long tail fills the session's context with lines it may not need.
- *`[land] check` as a list of commands in `clm.toml`.* CLM would know which command exited non-zero. Rejected: it moves each project's check out of a script that already does it better (selection, installs, skips), and it widens `config`'s contract for one message.
- *Matching failure words (`error`, `FAILED`) in the output.* Rejected: the words vary by runner and language, and a match can point at a harmless line.
- *Step markers the script prints, in GitHub Actions' form.* Chosen (`check-step-markers`, `reuse-first`). The script knows its steps. CLM only counts lines and remembers the last marker, which is deterministic and the same in every language. A script that also runs in GitHub Actions gets folded logs from the same lines.

## Outcome

- `land`'s Design, **The check's output**: the runner numbers the output's lines and follows `::group::`/`::endgroup::` lines. It keeps only the open step's name and first line, so no memory beyond the tail. A failing land's refusal adds `; the step open when it ended is `<name>`, lines <first> to <last> of its output` before `; its output is at <path>`.
- `land/check-log` states the step in the refusal, with scenarios for a named failing step, a step that ended before the failure, and a marker split across writes.
- `land/shared-check-runner` returns the open step beside the tail. `--check-only` and `verify` print nothing new.
- The decision `check-step-markers` records the choice and amends `land-check-log` in what the refusal says.
- `docs/guide/ci-and-deployment.md` tells adopters to mark their check's steps.

## Constraints

- The refusal says "the step open when it ended", not "the step that failed". CLM sees only output and an exit status. Under `set -e` the two are the same. A script that goes on after a failure, or fails outside any step, can't be read that way, and then the refusal says only what CLM knows.
- No behaviour changes for a check that prints no markers, so every existing project's refusal stays as it is.

## Edge cases and workarounds

- A marker split across two reads of the pipe is matched once its line is whole.
- A last line with no trailing `\n` counts as a line.
- A `::group::` with an empty name is still a step, named by the empty string.
- With no log (it couldn't be created), the line numbers count the output copied to stderr.

Out of scope: this repository's `.github/scripts/ci-check.sh` and the `scripts/clm-check.sh` that `/clm:init` writes printing markers. Neither is needed for the item. They are recorded as `backlog/ci-check-marks-no-steps` and `backlog/init-check-starter-marks-no-steps`.

## Open questions

The builder check raised these about `land/shared-check-runner`, which `20261107-slow-tests-after-land` added and nothing has built yet. They predate this change, which only adds the step to what the runner returns, so they are left to that build to settle within the requirement as written:

- where the runner finds the check lock's path, since its inputs name no CLM root (a land checks in `.clm/land-tree` or a temporary worktree, so the run directory can't give it);
- whether its log path is optional, since `--check-only` keeps no log today;
- the shape of the environment it takes, and whether the selection file is separate from it;
- the refusal wording stays with each caller (`clm land` here, `verify` later), and the runner returns the facts the wording needs.
