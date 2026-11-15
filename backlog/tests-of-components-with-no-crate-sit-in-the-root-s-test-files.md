---
title: "Tests of components with no crate sit in the root's top-level test files"
kind: deferred
components: [ci, plugin, plugin.agents, plugin.panel]
origin: 20261115-tests-in-their-own-files
blocks: [clm/routes-only]
---

144 tests in `crates/clm/tests/` prove `ci` (121, in `ci_check.rs`, `release_workflow.rs` and `community_files.rs`), `plugin` (18, in `plugin.rs`), `plugin.agents` (4, in `plugin_agents.rs`) and `plugin.panel` (1, in `plugin.rs`). None runs the `clm` binary: they read workflows, scripts and plugin files. The files are the root's, so each build of those components edits a file it doesn't own, which only the clause "a file that cites the component" lets through (`own-files-at-the-gate`).

Each component now names a directory of its own in the root package: `crates/clm/tests/ci_suite/`, `crates/clm/tests/plugin_suite/`, `crates/clm/tests/plugin_agents_suite/` and `crates/clm/tests/plugin_panel_suite/` (`commands-live-in-their-crates`). No directory shares a name with a top-level test file, which Cargo would refuse as two targets of one name. The work, one component at a time, each under its own `clm claim <component> --fix`, the last deleting this item:

- move every test `changes/20261115-commands-in-their-crates/moved-tests.txt` (under `changes/archive/` once that change is complete) lists for the component into its directory, as a test target with a `main.rs` as the capability suite has, each under its own name with what it asserts unchanged, and the helpers it uses with it;
- delete the test from the root's file, and the file when nothing is left in it;
- delete no test that hasn't moved, and say in the pull request how many moved against how many the list holds.

`plugin.panel`'s one test may move into the panel's own suite under `plugin-panel/` instead, when it can be written there with what it asserts kept.

Due now. It changes nothing a requirement states.
