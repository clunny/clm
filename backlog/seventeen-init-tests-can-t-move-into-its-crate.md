---
title: "Seventeen of init's tests can't move into its crate"
kind: question
components: [init, clm, check, apply, view.repo, orchestrator.run]
origin: 20261115-commands-in-their-crates
blocks: [clm/routes-only]
choices:
  - "Root tests; root re-points doc_readers"
  - "Move each to its command's component"
  - "Reword the scenarios"
---

`init/commands-in-crate` moved 83 of the 100 tests `changes/20261115-commands-in-their-crates/moved-tests.txt` lists for `init` into `crates/clm-init/tests/` (`init.rs` 49 of 60, `survey.rs` 34 of 34). `survey.rs`'s `from_a_subdirectory` also cites the root's `clm/root-from-cwd`, so the root keeps it; init's build took its own marker off it and proves its own copy in the crate. Seventeen stay where they are:

**Eleven observe another component's command** on what `clm init` wrote, which `clm-init` (`depends_on: [model, config, vcs, upgrade, host]`) can neither call (`check` depends on `init`, so a dev-dependency there would be a cycle besides an undeclared import) nor run as a binary, which Cargo gives only to `crates/clm`'s tests. They stay in `crates/clm/tests/init.rs`:

- `clm check` passes on the result: `empty_repository`, `init_writes_capabilities`, `template_says_architectural_constraint`, `templates_say_new_words`, `architecture_template_proof` (`init/init-files`, `init/template-says-architectural-constraint`, `init/templates-say-new-words`, `init/architecture-template-proof`); `init_seeds_the_allow_rules`, `init_keeps_the_settings_it_finds` (`init/init-seeds-permissions`); `empty_repository_every_language`, `dependencies_not_installed` (`init/init-without-manifest`).
- `clm hashes` and `clm pending` find no entry in the layers it wrote: `init_writes_capabilities`, `template_says_architectural_constraint` (`apply`, `view.repo`).
- `clm setup` runs the `[build] setup` it wrote: `typescript_project_sets_build_setup`, `setup_installs_when_package_json_exists` (`init/init-build-setup`, `orchestrator.run`).

**Six guide tests** (`crates/clm/tests/guide.rs`, `init/guide-command`, `init/guide-table-from-files`, `init/guide-constraints-page`) would move with no other change: `clm-init`'s call proves them (`crates/clm-init/tests/command.rs` runs `clm guide`'s call). But the root's `crates/clm/tests/doc_readers.rs` (`the_guides_readers`, `the_spec_formats_readers`) asserts that `crates/clm/tests/guide.rs` holds the `clm:reads` markers of `docs/guide/**` and `docs/spec-format.md`, and an `impl/init` build may not change that file (`land/build-holds-own-files`). Once the root's test names `crates/clm-init/tests/guide.rs` instead, or either path, `init`'s next build moves them.

What is needed: for the eleven, a spec change saying where each scenario is proven once `clm/routes-only` holds, as `backlog/two-config-tests-run-other-components-commands.md` was answered (a scenario only a `clm` process can show is the root's); for the six, the root's edit to `doc_readers.rs`.

## The human's answer (2026-11-15)

The first choice. The eleven tests of another component's command on what `clm init` wrote are the root's: the batched spec change that follows the build run moves their scenarios into requirements of the root, or names the capability test that already shows one (`commands-live-in-their-crates`). For the six guide tests, a small code branch makes the root's `doc_readers.rs` name `crates/clm-init/tests/guide.rs`; `init`'s next build then moves them.
