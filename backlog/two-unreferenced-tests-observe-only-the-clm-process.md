---
title: "Two of check.unreferenced's tests observe only the clm process and can't move into its crate"
kind: question
components: [check.unreferenced, clm, check]
origin: 20261115-commands-in-their-crates
blocks: [clm/routes-only]
choices:
  - "Both stay as root tests of routing"
  - "Detector stays; subdirectory to clm"
  - "Rewrite both for the crate's call"
---

`check.unreferenced/commands-in-crate` moved 11 of the 13 tests `changes/20261115-commands-in-their-crates/moved-tests.txt` lists for `check.unreferenced` into `crates/clm-unreferenced/tests/command.rs`. Two stay in `crates/clm/tests/unreferenced.rs`, because what they observe exists only in a `clm` process, so replacing the helper that ran the binary by a call of the crate would change what they assert (the change's "What it removes" says such a test stays and its build files a `question` item that blocks `clm/routes-only`):

- `scenario_named_as_a_detector` (scenario "named as a detector" of `check.unreferenced/report`) runs `clm check` with the `clm` binary named as a detector in `clm.toml`. `clm check` is `check`'s command and the detector is a child process; `clm-unreferenced` depends on `vcs` alone, so its tests can neither call `clm-check` nor run a `clm` binary, which Cargo gives only to `crates/clm`'s tests.
- `scenario_from_a_subdirectory_of_a_project_without_clm` (scenario "from a subdirectory of a project without CLM" of `check.unreferenced/report`) runs `clm unreferenced` from `crates/a/src`. The root it scans is the one `crates/clm` finds from the current directory (`survey_root`, the root's `init-finds-its-repository`), and the call takes that root as given, so the crate's call can't observe it. The root's Design lists "the directory it ran in" among what a moved test sets on its own process, but here the directory decides only the root `crates/clm` finds, not anything the crate reads: a moved test would pass the subdirectory as the root and assert paths the crate never gives from it.

What is needed: say where each of these two scenarios is proven once `clm/routes-only` holds, and whether `check.unreferenced/report` keeps them or they move to the root's or `check`'s requirements, a spec change.
