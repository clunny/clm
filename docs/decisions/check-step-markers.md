---
title: "A land check marks its steps with GitHub Actions' group lines, and a failing land names the step open when it ended"
date: 2026-11-07
status: accepted
---

`clm land` follows the steps `[land] check` marks in its own output, in GitHub Actions' workflow-command form: a line starting `::group::` starts a step named by the rest of the line, and a line `::endgroup::` ends it. When the check fails, the refusal still repeats the last 40 lines and names the log, and when a step was open as the check ended it also names that step and the lines of the log it spans. A check that marks no steps gets the refusal without a step. This refines `land-check-log` in what the refusal says and nothing else: the pipe, the log's place and handling, and the 40-line tail are as it decided. Beside GitHub Actions' form the check prints one line of CLM's own, `::clm-selected::<path>`, for the files that made it run a step (`check-selector-lines`).

**Why.** A project whose check runs several steps (an install, the unit tests, a browser end-to-end suite) can end its output with more than 40 lines after the failing step's own message, so the tail held no reason: the session that hit it reran the suite separately to tell a flake from a real failure (GitHub issue #262). `land-check-log` rejected parsing the output for the failing stage because the check is the project's own script and its stages vary. A marker turns that around: the script, which knows its steps, names them, and CLM only counts lines and remembers the last marker, which is deterministic and the same for every language. The refusal then tells the session which step was running and where in the kept log to read, however long that step's output is.

**Why GitHub Actions' form** (`reuse-first`). It is the most widely used existing convention for naming steps in a script's output, a script run in GitHub Actions already folds its log by it, and a script that prints it costs nothing anywhere else. A form of CLM's own would ask every project to learn a new line for one tool.

**Why "the step open when it ended", not "the step that failed".** CLM sees only the output and the exit status. Under `set -e` the step open when the check exits is the one whose command failed; a script that goes on after a failure, or fails outside any step, can't be read that way, so the refusal says what CLM knows, and names no step when none was open.

**Alternatives.**
- *A longer tail.* Any fixed count can be outrun by one step's output, and a long tail fills the session's context with lines it may not need.
- *`[land] check` as a list of commands.* CLM would know each step's exit status, but every project's check moves into `clm.toml`, which a script already does better (selection, installs, skips), and `clm.toml`'s contract widens for one message.
- *Matching failure lines (`error`, `FAILED`).* Words vary by runner and language, and a match can point at a harmless line.
