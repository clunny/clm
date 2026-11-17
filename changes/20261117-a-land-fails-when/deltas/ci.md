---
component: ci
---

## MODIFIED Requirements

### Requirement: phase-times <!-- base: e8375a0b123b5f0d -->
`.github/scripts/ci-check.sh` SHALL time each phase it runs: the tool installs, as one phase `tools`, then `cargo fmt`, `cargo clippy`, the wasm builds, the Rust tests, and each npm suite, capability package and `claude plugin validate` it runs. After each it SHALL print `ci-check: <phase> <n>s` to stderr, `<n>` the whole seconds it took. On exit it SHALL keep its exit status and print to stderr: when it passes, `ci-check: phases`, then one line `<n>s <phase>` per phase, in order, then `<n>s total`; when a phase fails, only `ci-check: failed in <phase> after <n>s, <n>s total`. It SHALL print nothing of these to stdout. Every `<n>` SHALL be a count of whole seconds, never negative: a time the shell's clock gives as less than 0, because the clock stepped back during the run, SHALL print as `0s`.

#### Scenario: a passing run
- GIVEN the script with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments and pass, and an empty selection file
- WHEN it runs
- THEN it exits 0, stderr holds `ci-check: phases`, a line ending in ` tools`, one ending in ` fmt`, one ending in ` clippy`, and a last line matching `<n>s total`, and stdout holds no line starting `ci-check:`

#### Scenario: a failing phase
- GIVEN the same fakes, but the `cargo` fake exits 3 on `clippy`
- WHEN it runs
- THEN it exits 3, stderr's last line is `ci-check: failed in clippy after <n>s, <n>s total`, stderr holds no `ci-check: phases`, and no line names a wasm-build or test phase

#### Scenario: the clock steps back
- GIVEN the same fakes, but `cargo fmt` sets the script's own shell clock (`SECONDS`) back by 100 seconds while it runs, as an exported bash function named `cargo` that takes precedence over the fake on `PATH`
- WHEN it runs
- THEN it exits 0, stderr holds `ci-check: fmt 0s`, and every `<n>` it prints, in each phase's line, the summary and `<n>s total`, is a count of digits with no `-`
