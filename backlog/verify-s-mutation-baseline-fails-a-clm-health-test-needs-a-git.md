---
title: "Verify's mutation baseline fails: a clm-health test needs a git repository"
kind: bug
components: [ci]
---

Step: `clm verify --due` (run by the build loop) runs the mutation step, .github/scripts/mutate.sh, which runs cargo mutants for requirement check.health/health-report.
What happened: cargo mutants' unmutated baseline failed, so no mutants were tested and verify recorded nothing. The one failing test is clm-health's `scenario_a_clean_repository` (crates/clm-health/tests/command.rs), which exits 2 because `clm health` runs `git ls-files` and the mutation copy of the tree is not a git repository. The full test suite passed in the same verify run, in the real checkout.
Expected: tests of a requirement that the mutation step targets pass in cargo mutants' copied tree, or the mutation step gives them a git repository, so verify can record mutation results.
Context: the test moved into clm-health with check.health/commands-in-crate earlier today; before that it ran the binary from the root crate.
