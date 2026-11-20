---
title: "The mutation pass judges a requirement by the first mutants of its component's code, not the code the requirement states"
kind: bug
components: [ci, verify]
---

Found building `machine` for `backlog/unproven-machine-75853921` (wave 1, change `20261119-wave-1-by-the-rule`).

`clm verify` writes every code file of the requirement's component into the plan (`clm-verify` `write_plan`), and `.github/scripts/mutate.sh` (`judge_rust`), with no diff since the base, tries only the first `limit` (20) mutants, in cargo-mutants' order, on lines the citing tests run, and calls the requirement killed when any one is caught. For `machine` those first 20 are all in `crates/clm-machine/src/lib.rs`'s `State::word`, `State::from_word` and `judge`'s CPU and memory rules (`machine-report`'s), which `read` runs for every test that calls it. So `machine/build-measure` and `machine/disk-room` were filed unproven by mutants of code neither requirement states.

Re-running cargo-mutants over the whole crate with each requirement's citing tests: every mutant of `disk-room`'s room line is caught, and `build-measure`'s tests catch every mutant of `record.rs` and of `read`'s due rule but one (`at > now` to `at >= now`, now killed by a new test); the other survivors in `walk.rs` are equivalent. Their tests are sound, but the next pass will judge them by the same 20 CPU and memory mutants and file them again: nothing a build of `machine` can add to these tests changes that, short of asserting CPU and memory states in tests that cite `build-measure`, which would make the verdict pass without proving the requirement.

The window should hold the mutants of the code the requirement's tests are about, for example the mutants on lines run by the citing tests but not by every test of the package, or every tryable mutant ordered by how many of the citing tests run its line, rather than the first in file order.
