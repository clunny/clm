---
title: "Two of config's tests run other components' commands and can't move into its crate"
kind: question
components: [config, clm, check, check.unreferenced]
origin: 20261115-commands-in-their-crates
blocks: [clm/routes-only]
choices:
  - "Both stay as root tests"
  - "Move each to its command's component"
  - "Reword both scenarios"
---

`config/commands-in-crate` moved 35 of the 38 tests `changes/20261115-commands-in-their-crates/moved-tests.txt` lists for `config` into `crates/clm-config/tests/` (`command.rs` and `this_repository.rs`). `this_repository_is_clean` also cites the root's `clm/clean-check-runs-own-clm`, so it stays the root's: config's build took its own marker off it and proves "with this repository as it stands, the detector SHALL exit 0" with `the_retired_terms_detector_passes_here`, which runs the detector's command itself. Two stay in `crates/clm/tests/cli.rs`, because what they observe is another component's command, which `clm-config` (`depends_on: []`) can neither call nor run as a binary, which Cargo gives only to `crates/clm`'s tests:

- `check_fails_on_a_retired_term_in_a_spec` (scenario "a retired term in a spec fails the check" of `config/retired-terms-detector`) runs `clm check` and asserts its `clm.toml: check/detectors: retired-terms failed (exit 1)` line, which `check` prints.
- `unreferenced_is_clean_with_the_allow_list` (scenario "clean with the list" of `config/unreferenced-detector`) runs `clm unreferenced` with this repository's allow-list; the scenario names "the `clm` binary under test".

Both requirements also say their tests "SHALL live in `crates/clm/tests/cli.rs`", which the root's Design overrides for `commands-in-crate`. What is needed: a spec change saying where each scenario is proven once `clm/routes-only` holds, such as keeping them as the root's tests of this repository's detectors, or rewording the scenarios so `config`'s crate can observe them.
