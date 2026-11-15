---
status: applied
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, its second change of four. `ci`, `plugin`, `plugin.agents` and `plugin.panel` have no crate, and their 144 tests sit in the root's top-level test files without running the binary. Each gets a directory of its own in the root package, as the capability suite has. The tests that move there, and the other tests that prove a component from another component's files, are recorded as backlog items, since moving a test changes nothing a requirement states.

It adds and removes no requirement, so it is archived in its own branch.

## Problem

`20261115-commands-in-their-crates` moves the tests of the 22 components that own a command. Measured on `main` at `4f96740b`, that leaves:

- 144 tests of the four components with no crate, which read workflows, scripts and plugin files;
- 64 tests in the root's files of `view`, `check.coverage` and `upgrade`, which own no command;
- 111 markers in 18 files outside the root that cite a component other than their owner: `check.coverage` in `check`'s crate (42), `land.approval` in `land`'s (25), the web family (40) and four single markers.

All of it rests on the clause "a file that cites the component" (`own-files-at-the-gate`), which can't go while any of it is left.

## Approaches

- **A directory per component in the root package, named in its `paths`.** Taken for the four components with no crate: they depend on no binary, the capability suite is the precedent, and a new test file there is its component's the moment it is written.
- **A test-only crate per component.** Not taken: four more crates and workspace entries for tests that need only a package to run in.
- **A requirement per component for the move alone.** Not taken: these components gain no behaviour, and the only test that could prove a move is a scan of source text (`dependency-work-is-a-requirement`). `commands-in-crate` states where its component's tests sit because the call it adds is what they move onto. A `deferred` item per group records the work here; the two that empty the root's files block `clm/routes-only`, and `20261115-builds-change-only-their-own-files` makes all three block the clause's removal.

## Outcome

- `ci`'s `paths` gain `crates/clm/tests/ci_suite/**`, `plugin`'s `crates/clm/tests/plugin_suite/**`, `plugin.agents`'s `crates/clm/tests/plugin_agents_suite/**` and `plugin.panel`'s `crates/clm/tests/plugin_panel_suite/**`. The root's Design names them, in `20261115-commands-in-their-crates`'s delta. No directory shares a name with a top-level test file, which Cargo refuses.
- `backlog/tests-of-components-with-no-crate-sit-in-the-root-s-test-files`: the move of those 144 tests. Due now.
- `backlog/some-tests-prove-a-component-from-another-component-s-files`: `view`, `check.coverage`, `upgrade`, `land.approval` and four single markers. Due now.
- `backlog/web-tests-prove-one-web-component-from-another-s-files`: the web family's 40 markers, which need a design first.

Each is fixed one component at a time under `clm claim <component> --fix`, by a session or `/clm:triage`, not by the build loop.

## What it removes

Nothing. Each item moves tests under their own names and deletes none that hasn't moved.

## Constraints

- No file, key or format an adopter holds changes.
- The new `paths` globs match no file yet, and no existing file changes owner.
