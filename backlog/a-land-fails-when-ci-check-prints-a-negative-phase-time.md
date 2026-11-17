---
title: "A land fails when ci-check prints a negative phase time"
kind: bug
components: [ci]
---

Step: `clm land` on an `impl/*` branch, running `.github/scripts/ci-check.sh` on the merged tree.

What happened: the land failed in the `tests` step on a test the branch did not touch, `ci_check::a_passing_run_prints_its_phase_times` in `crates/clm/tests/ci_suite/`. It panicked on the line `ci-check: tools -2s`: the script printed a negative phase time, and the test expects a count of seconds. The same land, run again at once with nothing changed, passed.

Expected: a phase time is never negative, or the test accepts whatever the script can print. A land fails only on what the branch changed.

Rule involved: `land-checks-stay-fast`, and `ci`'s requirement on the phase times the check script prints. The run was on a loaded machine with other sessions landing; the phase times are taken from the clock in whole seconds.
