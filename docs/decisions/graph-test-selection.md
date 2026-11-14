---
title: "A land runs the tests the spec graph says its change can break"
date: 2026-10-27
status: accepted
---

`clm land` runs the project's whole check command on every merged tree whose code it hasn't checked, one land at a time per clone (`land-target-lock`). Projects built with CLM will have slow integration and end-to-end suites, and a full run per land caps how many branches a clone lands in a day. The spec graph already says what a change can break, so CLM reads the selection from it rather than from a build tool or a coverage map (`reuse-first`).

**The selection** (`clm affected`, node `check.affected`):
- A diff maps to nodes through each node's `paths`; the *affected* nodes are those plus every node that depends on one of them, directly or indirectly (`depends_on`).
- A file belongs to the nodes its `req:` markers cite, so an integration test citing requirements of several nodes runs when any of them is affected. A file with no marker belongs to the node whose `paths` hold it, as `node-size` counts it (`tests-count-where-cited`): helpers and smoke tests run with their node.
- A changed file always runs, so an edited test is run by the land that edits it.
- Changes under the doc-only paths `code_key` leaves out change no code, so they select only the files whose `clm:reads` markers name them (`doc-readers-marked`).
- A changed file no node's `paths` hold, or one a glob in `clm.toml`'s `[affected] all` matches, selects everything. The glob list is for files a node owns whose reach is wider than its dependants, such as a lockfile, the toolchain pin or a CLI's dispatch.

The selection is as sound as `paths` and `depends_on`: a test that exercises code of a node it neither cites nor lives in, through a dependency the graph doesn't declare, can be skipped. `check`'s rules already fail an undeclared dependency, and the full suite still runs wherever the check runs outside a land: in every build session before it lands, by hand, and in the release's `prepare` job (`release-by-dispatch`).

**CLM selects, the project runs.** `clm affected` prints nodes and files; it knows no test runner. `clm land` runs it on the merged tree against the `main` it merged into and hands the output to `[land] check` as a file named by `CLM_SELECTION`. The check command maps files to its runner's syntax; one that ignores the variable runs everything, so adopting the selection is opt-in and nothing changes for an existing project until its check command reads it. When `clm affected` fails, `clm land` warns and runs the check without a selection: a wrong selection can only cost time, never a skipped test.

Rejected:
- **The mapping from files to runner arguments in `clm.toml`**, a template per glob, as the backlog item suggested. Every runner selects differently (cargo by package and target, npm by suite, pytest by path), and the check command is already the project's own script; a template language in `clm.toml` would reinvent part of it.
- **Running the full suite on a schedule.** Nothing runs on a schedule since `land-locally`: no workflow but the release is left, and a scheduled one would be the second. The full runs above already happen several times a day.
- **Selecting by the crate graph (`cargo`'s dependency tree) or by coverage.** Language-specific, and a second graph beside the spec graph, which CLM checks already.
- **Recording in the ledger which tests a `checked` record ran.** A `checked` record names a code key, and two merged trees with one code key hold the same code, so the same tests pass on both; the record stays as it is.

## The rule in full

`ARCHITECTURE.md`'s entry `land-checks-stay-fast` states this in short; the rule in full:

A project's `[land] check` SHALL run the tests `CLM_SELECTION` names, or every test when it is unset, empty or names `all`, and those tests SHALL be fast, deterministic and able to run beside other runs (`tests-run-concurrently`); every file a test reads SHALL be one the spec graph connects it to: a file of the component that owns it or of one it depends on (`depends_on`), or one a `req:` or `clm:reads` marker in it names. A test that is slow, fails or passes from run to run on the same tree, or needs a resource no other run may share belongs in `[verify]`, after the land (`verify-after-land`). CLM builds a land's verdict from checks of its parts: a land whose branch's head was checked whole runs only the tests the branch and what `main` gained since can both affect (`land/overlap-recheck`), so the tests' honesty about what they read is what that verdict rests on.

**Scenario: an unrelated move of main**
- GIVEN a project whose build checked its branch whole, and `main` moved since by a land on a component the branch's components neither own nor depend on
- WHEN the branch lands
- THEN its land runs no test, and a test that reads a file its markers don't name is the project's defect, which `[verify]` shows after the land, or, in a project without `[verify]`, the next build's full check

**Guarantee**
A test that the spec graph doesn't connect to a change gives the same result with and without it, so CLM can land a branch on a `main` that moved without re-running the tests the two can't both affect, and slow or exclusive tests never hold a land.

**Untested**
Whether a test is deterministic, and whether every file it reads is one the graph connects it to, are things CLM can't see before the fact: a gap shows as a failure after the land, which `[verify]` finds, bisects to a land and reports with how that land re-checked, or, in a project that sets no `[verify]`, as the next build's full `clm land --check-only` failing on a `main` it didn't change (`graph-test-selection`'s safeguard), without presuming whether the graph, the code or the test was wrong. A check that runs more than its selection is never wrong, only slower, and the check-time report says where its time went.
