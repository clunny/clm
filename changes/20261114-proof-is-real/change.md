---
status: applied
frontmatter_keys: [wiring]
---

## Intent

Phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: make proof real. Three checks replace three instructions. A `req:` marker counts only inside a test. The gate, and so `clm land`, refuses a build's change to a file that isn't its to change. And after a land, `clm verify` asks the project's mutation command whether the tests of each newly built requirement fail when its code is changed; a requirement whose tests notice nothing gets a bug item and reads as not built until a build mends the test.

It adds checks and removes no requirement, scenario or behaviour.

## Problem

- A requirement reads as built when a marker with the right version exists in any file outside Markdown. Nothing ties it to a test.
- `path-ownership` is kept by the build prompt alone. Of the last 429 builds, 29 changed a file of another component that none of its written exceptions covers.
- Nothing shows that a test would fail if the code were wrong. Builds are told to break each key rule by hand, and nothing records that they did. CLM's own capability test for `prove-by-tests` passes with an empty test function.
- The list of tests a removal deletes was made by a script kept outside the repository.

## Approaches

Each part's choice is in its decision; in short:

- **Which record makes a marker count** (`markers-count-in-tests`). Taken: none new. The marker must sit in a test, read from the file's text, and the land's existing rule already has every changed file in the selection of the check behind its verdict. Not taken: reading the test runner's results, which needs a format every project's check must write, a join per runner, and a ledger read wherever coverage is computed.
- **The ownership rule's transitional case** (`own-files-at-the-gate`). Taken: a file that cites the building component is its to change, the reading the gate already uses for code fixes. Not taken: an exemption per owner of such files, eleven here, each open to every build.
- **What the mutation pass judges** (`mutation-after-land`). Taken: requirements whose citing tests were added or changed, against the changed code first. Not taken: every re-cited requirement against a sample of its component, which a measurement showed misjudges sound tests.
- **How a verdict reaches coverage.** Taken: a `bug` item naming the requirement under `unproven`, landed by verify, so coverage stays a function of the tree. Not taken: coverage reading the ledger, and a new state word.

## Outcome

**The marker rule** (`check.coverage`, `check`).
- `check.coverage/test-spans`: the library finds the tests and groups of tests in a Rust, TypeScript, JavaScript or Python file, and the markers outside them. Counting is unchanged by it.
- `check.coverage/markers-in-tests`: only markers inside a test count in those languages; `clm coverage` lists the others as `uncounted`. In any other language every marker counts as before.
- `check.coverage/test-lists`: `clm coverage --obsolete-tests` and `--deleted-tests <base>`, the lists phase 2 made by script.
- `check/dangling-markers` judges the markers outside tests too.
- On `main` at `9f2746dd`, 16 of 4,432 markers stop counting, all file-header comments in TypeScript test files, and five requirements become pending: `plugin.panel/build-rows`, `web.client-live/live-updates`, `web.client-live/push-renewal`, `web.views/backlog-item-page` and `web.views/home-waiting-on-you`. Their builds move the markers.
- 118 tests in ten components build fixtures whose marker sits in a bare comment. `backlog/test-fixtures-hold-markers-outside-any-test` blocks `markers-in-tests` until one code branch has mended them.

**The ownership rule** (`gate`, `model`, the root).
- `gate/own-files`, `model/wiring-globs`, and `own-files` as a rule an exemption may name.
- The root sets `wiring` in this change, declared under `frontmatter_keys`, so the rule is in force here from the day the gate's build is installed.

**The mutation pass** (`verify`, `config`, `ledger`, `model`, `check.coverage`, `ci`).
- `config/verify-setting` gains `mutate`; `verify/mutation-pass` and `verify/files-unproven`; `ledger/mutated-record`; `model/parse-backlog` gains `unproven`; `check.coverage/coverage-report` reads it.
- `ci/mutate-script` is this repository's command, with `cargo-mutants` and StrykerJS, and runs the coverage script at the end of each pass.
- Nothing runs it until `clm.toml` sets `[verify] mutate`, which a later spec branch does.

**Adopters** (`upgrade/migration-markers-in-tests`). One advised note. A project's markers outside tests stop counting, which makes requirements pending and never fails a check; the note points at the `uncounted` list before the next build. The ownership rule and the mutation pass are off until the project sets `wiring` and `[verify] mutate`.

## What a land pays

The marker rule adds a scan of the marker-bearing files to `clm check`, tens of milliseconds. The gate loads the merge base's specs for every build branch, as it does today for a code fix, tens of milliseconds on 0.14 seconds. The mutation pass runs after the land and outside the check lock.

## Left for a later spec branch

It needs review, and can't be written until this change's builds are installed:
- `ARCHITECTURE.md`: `path-ownership` becomes checked; the blocks of `proven-by-tests` and `exemptions-end` are rewritten.
- `clm.toml`: `[verify] mutate` and the lcov path the pass writes.
- `upgrade`'s `own-files` exemption for the migrations `init`'s builds write.
- `path-ownership-by-prompt` is deleted, and the guide's page on what a build may change is rewritten.

## Constraints

- `clm land` gains only checks that take milliseconds.
- No project that was green before upgrading turns red: the marker rule makes requirements pending, and the other two parts are off until the project switches them on.
- `crates/clm/tests/capability/right_model_per_task.rs` is not part of this change: its three fixed sentences go on a code branch of its own.
