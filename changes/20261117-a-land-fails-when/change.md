---
status: applied
resolves: [a-land-fails-when-ci-check-prints-a-negative-phase-time]
---

## Intent

A phase time that `ci-check.sh` prints is never negative. When the machine's clock steps back during a land's check, the script prints `0s` for a phase or total the clock would make negative, instead of a `-2s` that fails `ci_check::a_passing_run_prints_its_phase_times` on a branch that changed nothing near it.

## Problem

A land of an `impl/*` branch failed in its `tests` step on `ci_check::a_passing_run_prints_its_phase_times`, which the branch didn't touch: the script it runs printed `ci-check: tools -2s`. The same land passed when run again at once. The script times each phase by bash's `SECONDS`, which bash 5 counts from the wall clock (`gettimeofday`), so a step back of the clock during a phase makes `SECONDS - phase_start` negative. This machine's wall clock does step back: `3edf25be` fixed the ledger and push-retry tests for the same cause. `ci/phase-times` says `<n>` is the whole seconds a phase took, but doesn't say what the script prints when its clock disagrees, and the test, rightly, expects a count of digits.

## Approaches

- **Taken: clamp each printed time at 0.** A phase's time, the failed phase's time and the total each print as `0s` when the shell's clock gives less than 0. One comparison per printed number, on the bash 3.2 the script runs on macOS too. A step back is a genuine edge case, the printed times are advisory (`land-checks-stay-fast` reads them to see where a land's time goes), and a phase whose clock stepped back has no true time left to print, so `0s` loses nothing a reader could use. The new scenario proves it: an exported bash function named `cargo` runs in the script's own shell, ahead of the fake on `PATH`, and sets `SECONDS` back while `cargo fmt` runs, as a clock step does.
- **Not taken: a clock that never steps back.** `/proc/uptime` is monotonic on Linux but macOS has none, and bash 3.2 has no other; a test can't step it, so no test could tell it from `SECONDS`.
- **Not taken: loosening the test to accept a `-`.** It would let the script print a time that isn't one, and the land log would still show `-2s`.

## Outcome

- `ci/phase-times` (modified): every `<n>` is a count of whole seconds, never negative; a time the clock makes negative prints `0s`. A new scenario, `the clock steps back`, proves it.
- No capability test sees a difference: none reads the check script's phase times.

## Edge cases and workarounds

- A step back of the clock prints `0s` for the phase it happened in and for the total when the total would be negative; the times of the other phases are as before.

## Open questions

- The scenario `the clock steps back` fakes the step with an exported bash function (`BASH_FUNC_cargo%=...`). Apple's patched bash 3.2 imports exported functions under another name, so that one test may not step the clock on a Mac with the system bash. Lands and verifies run on Linux, so this is left as it is.
