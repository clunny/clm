---
status: applied
---

## Intent

Phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: make proof real. Three checks replace three instructions. A `req:` marker counts only inside a test. `clm land` refuses a build's change to a file that isn't its to change. And after a land, `clm verify` asks the project's mutation command whether the tests of each newly built requirement fail when its code is changed; a requirement whose tests notice nothing gets a bug item and reads as not built until a build mends the test.

It adds checks and removes no requirement. It replaces one scenario and ends one behaviour, listed under What it removes.

## Problem

- A requirement reads as built when a marker with the right version exists in any file outside Markdown. Nothing ties it to a test.
- `path-ownership` is kept by the build prompt alone. Of the last 429 builds, 29 changed a file of another component that none of its written exceptions covers.
- Nothing shows that a test would fail if the code were wrong. Builds are told to break each key rule by hand, and nothing records that they did. CLM's own capability test for `prove-by-tests` passes with an empty test function.
- The list of tests a removal deletes was made by a script kept outside the repository.

## Approaches

Each part's choice is in its decision; in short:

- **Which record makes a marker count** (`markers-count-in-tests`). Taken: none new. The marker must sit in a test, read from the file's text, and the land's existing rule already has every changed file in the selection of the check behind its verdict. Not taken: reading the test runner's results, which needs a format every project's check must write, a join per runner, and a ledger read wherever coverage is computed.
- **The ownership rule's transitional case** (`own-files-at-the-gate`). Taken: a file that cites the building component is its to change, the reading the gate already uses for code fixes. Not taken: an exemption per owner of such files, eleven here, each open to every build; a new exemption rule was cut from the change altogether.
- **What the mutation pass judges** (`mutation-after-land`). Taken: requirements whose citing tests were added or changed, against the changed code first. Not taken: every re-cited requirement against a sample of its component, which a measurement showed misjudges sound tests.
- **How a verdict reaches coverage.** Taken: a `bug` item naming the requirement under `unproven`, landed by verify, so coverage stays a function of the tree. Not taken: coverage reading the ledger, and a new state word.

## Outcome

**The marker rule** (`check.coverage`, `check`).
- `check.coverage/test-spans`: the library finds the tests and groups of tests in a Rust, TypeScript, JavaScript or Python file, and the markers outside them. Counting is unchanged by it.
- `check.coverage/markers-in-tests`: only markers inside a test count in those languages. In any other language every marker counts as before.
- `check.coverage/test-lists` and `check/coverage-lists`: `clm coverage` lists the markers outside tests as `uncounted`, and `--obsolete-tests` and `--deleted-tests <base>` print the lists phase 2 made by script.
- `check/dangling-markers`, `check.affected/affected-files`, `rename/marker-rewrite` and `check.health/churn-signal` read every marker, outside tests too: such a marker still can't name what doesn't exist, still selects its file, is still renamed and still counts as churn.
- On `main` at `9f2746dd`, 16 of 4,432 markers stop counting, all file-header comments in TypeScript test files, and five requirements become pending: `plugin.panel/build-rows`, `web.client-live/live-updates`, `web.client-live/push-renewal`, `web.views/backlog-item-page` and `web.views/home-waiting-on-you`. Their builds move the markers.
- 118 tests in ten components build fixtures whose marker sits in a bare comment. `backlog/test-fixtures-hold-markers-outside-any-test` blocks `markers-in-tests` until one code branch has mended them.

**The ownership rule** (`gate`, `model`, the root).
- `gate/own-files` finds the files, `land/build-holds-own-files` refuses them at the land and in `clm land --check-only`, and `model/wiring-globs` is the root's key. The gate itself reads no branch name, as before.
- The rule is off until the root sets `wiring`. This repository sets it in a later branch: first a build of `init` that writes one of `upgrade`'s migrations needs a way through (`backlog/a-template-and-its-migration-live-in-different-components`).

**The mutation pass** (`verify`, `config`, `ledger`, `model`, `check.coverage`, `ci`).
- `config/verify-setting` gains `mutate`; `verify/mutation-pass` and `verify/files-unproven`; `ledger/mutated-record`; `model/parse-backlog` gains `unproven`; `check.coverage/coverage-report` reads it; `orchestrator.triage/triage` leaves such an item to its component's build.
- `ci/mutate-script` is this repository's command, with `cargo-mutants` and StrykerJS, and runs the coverage script at the end of each pass.
- Nothing runs it until `clm.toml` sets `[verify] mutate`, which a later spec branch does.

**Adopters** (`upgrade/migration-markers-in-tests`). One advised note. A project's markers outside tests stop counting, which makes requirements pending and never fails a check; the note points at the `uncounted` list before the next build. The ownership rule and the mutation pass are off until the project sets `wiring` and `[verify] mutate`.

## What a land pays

The marker rule adds a scan of the marker-bearing files to `clm check`, tens of milliseconds. The land loads the merge base's specs for every build branch, as the gate does today for a code fix, tens of milliseconds on a gate that takes 0.14 seconds. The mutation pass runs after the land and outside the check lock.

## Left for a later spec branch

It needs review, and can't be written until this change's builds are installed:
- `ARCHITECTURE.md`: `path-ownership` becomes checked; `exemptions-end` is reworded to cover the land's rule; the blocks of `proven-by-tests` are rewritten.
- The root's `wiring`, once a template and its migration share a component or the rule has a way to keep that exception.
- `clm.toml`: `[verify] mutate` and the lcov path the pass writes.
- `path-ownership-by-prompt` is deleted.

## What it removes

- **A scenario, replaced.** `check.coverage/marker-spans` loses "the same markers as markers_in" (`marker_spans` and `markers_in` find the same markers in the same order), which stops being true. "every marker of the file" replaces it: `marker_spans` finds every marker, the ones `markers_in` returns and the ones outside tests.
- **A behaviour, ended.** A marker outside a test, in a language CLM reads, no longer makes a requirement read as built. That is the point of the change, and the human agreed to it in the session that proposed it.

No requirement, test or other scenario goes.

## Open questions

From the architecture audit, left as they are:
- `ARCHITECTURE.md` is untouched, so two of its `#### Untested` texts are untrue from the day the builds are installed until the later branch: `proven-by-tests` says the check reads a marker wherever it sits, and `path-ownership`, still labelled a principle, says no land check refuses a file. The later branch needs the built keys, so the edits can't come first.
- `path-ownership-by-prompt` stays accepted until the rule is switched on here. It says the gap is accepted, which is true of this repository until then.
- The limit on a wiring edit, "only to add or change the entry its own component needs", is checked by no program. This repository's wiring will include all of `crates/clm/src/**`.
- The clause "a file that cites the component" opens the whole file to that component's build, not only its tests.

## Constraints

- `clm land` gains only checks that take milliseconds.
- No project that was green before upgrading turns red: the marker rule makes requirements pending, and the other two parts are off until the project switches them on.
- `crates/clm/tests/capability/right_model_per_task.rs` is not part of this change: its three fixed sentences go on a code branch of its own.
