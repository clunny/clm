---
status: proposed
resolves: [the-written-exception-to-path-ownership-lets-a-build-re-cite-a]
human_burdens:
  - "The mutation pass spends this machine's time after lands, for as long as it is on: a `cargo-mutants` and StrykerJS run and an instrumented run of the whole suite, at least every 20 lands and when the queue drains."
  - "A build of `init` that changes a file `clm init` writes is refused at its land until a template and its migration share a component (`backlog/a-template-and-its-migration-live-in-different-components`), so such a change waits."
  - "A fix that spans several components is built one component at a time, one claim each, where one claim carried it before."
---

## Intent

The second spec branch of phase 3 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`. `20261114-proof-is-real` built three checks and left two of them off. This change switches them on in this repository and makes the human layer say so: `path-ownership` becomes a checked entry, the root names its `wiring`, `proven-by-tests` says what is now checked and what still is not, and this repository's settings name the mutation command, once that command works.

It removes no requirement, scenario or test. It ends two routes a build had here, listed under What it ends.

## Problem

- `ARCHITECTURE.md` says two things that stopped being true when `20261114-proof-is-real` was built: `path-ownership` is labelled a principle whose `#### Untested` says no land check refuses a file, and `proven-by-tests` says the check reads a marker wherever it sits.
- The ownership rule is off until the root sets `wiring`, and the mutation pass until `clm.toml` sets `[verify] mutate`.
- `.github/scripts/mutate.sh` judges nothing. Run with the real tools on 2026-11-14, on the lands `2ae3b3a0` and `98ca15ee`, every requirement came back `not-judged` and the script exited 0. Its nextest filter names a test target with `binary(=<package>::<target>)`, which matches no target; that form is a `binary_id`. StrykerJS is given `--vitest.configFile` on its command line, which it takes only from a configuration file; past that, StrykerJS 10.1.1 calls a function that TypeScript 7.0.2 doesn't have; and past that it runs no test. The script's tests use fakes of both tools, so `ci/mutate-script` read as current throughout.
- A tool that fails is recorded as `not-judged`, and a test that hasn't changed is never judged again, so a broken tool would quietly exempt every requirement built while it was broken.
- The tracefile the pass copies holds absolute paths under the verify tree for the Rust files. `clm health` strips only the root of the checkout it runs in, so in any other checkout it drops every Rust file without saying so.
- `check`'s and `check.coverage`'s Contracts say every component reaches coverage through `clm-check`'s re-exports. Five import `clm-coverage` directly: `check.affected`, `check.health`, `rename`, `gate` and `verify`.
- `path-ownership-by-prompt` records an accepted gap that this change closes.

## Approaches

**A template and its migration.** A build of `init` that changes a file `clm init` writes has also written `upgrade`'s migration and mended `upgrade`'s tests, in four of the last 300 build lands. The ownership rule has no clause for it.
- *Taken: no clause. The rule refuses it, and the two come to share a component in a change of their own.* The exception exists because one behaviour is split over two components. The refusal says so, and the requirement waits behind a question item, as the rule's message tells the build.
- *Not taken: the migration files in `wiring`.* It opens `upgrade`'s files to every build, makes a transitional exception permanent, and would not have let any of the four lands through, since each also edited `upgrade`'s tests.
- *Not taken: an exemption rule.* An exemption covers only a component's own files, so it would sit on `upgrade` and open its files to every build, which is `wiring` with an end date, for a new rule name that `check.health`, `check` and `init` match.

**A fix that spans components.** The fixtures mend of 2026-11-14 changed 17 files of ten components on `impl/check` under one `Resolves:` trailer; the rule would have refused it.
- *Taken: no clause.* Work is per component (`component-reconciliation`): each component's part of such an item is fixed under its own claim, and the last deletes the item.
- *Not taken: a clause for the files of the components a resolved item names.* A sixth clause on the rule's first day, for a route that already departs from one build per component.

**What `wiring` holds.** Replayed over the last 300 `impl/*` lands with the real `foreign_files`: with an empty list 48 would have been refused; the 27 that a list saves touched only `crates/clm/src/main.rs` (15 edits), `Cargo.lock` (14), `crates/clm/Cargo.toml` (4), `crates/clm/src/upgrade_gate.rs` (4), `clm.toml` (4) and `Cargo.toml` (2).
- *Taken: those files but `clm.toml`.* `clm.toml` is `config`'s: this repository's settings are requirements of `config` (`verify-here`, `health-lcov-here`), built by `config`.
- *Not taken: `crates/clm/src/**`,* the earlier draft. No other build touched the root's other four source files.

**Proving the mutation script with the real tools.** The land check has no place for a test that installs and runs two mutation tools (`land-checks-stay-fast`).
- *Taken: the script judges a fixture of its own with each tool before it judges the plan, and fails the pass when the verdicts are wrong.* It runs where the tools run, after the land, and catches a tool or a compiler that moves later.
- *Not taken: a test in the suite that runs the real tools.* It would add their run to every land that selects it.

**The Contracts.** *Taken: say what `check-coverage-split` decided on 2026-11-14.* A component that reads coverage names `check.coverage` and imports `clm-coverage`; the re-exports stay for the components that already use them. *Not taken: moving the twelve crates that use the re-exports,* twelve builds for no change in behaviour, which belongs with phase 4.

## Outcome

**The human layer** (`ARCHITECTURE.md`, edited directly).
- `path-ownership` is **Checked**, in force once a project's root names its `wiring`. `#### Enforced by` names `check/paths-disjoint`, `check/tests-cite-their-owner`, `model/wiring-globs`, `gate/own-files` and `land/build-holds-own-files`. `#### Untested` keeps what no program sees: that a wiring edit adds only the build's own entry, that a capability-test edit keeps what the capability states, and that a branch named outside `impl/` is not judged.
- `proven-by-tests`: `#### Enforced by` gains `check.coverage/test-spans`, `check.coverage/markers-in-tests`, `verify/mutation-pass` and `verify/files-unproven`. `#### Untested` now says: a marker in a language CLM doesn't read counts anywhere; whether the test ran is not read from a runner; the mutation pass judges only where a project sets `[verify] mutate`, only tests added or changed since it began, and no browser test; and CLM knows no eval proof.
- `exemptions-end` is unchanged: `wiring` is no exemption, and the land's rule has none.

**The ownership rule, on here** (the root). `specs/SPEC.md` sets `wiring` to `Cargo.toml`, `Cargo.lock`, `crates/clm/Cargo.toml`, `crates/clm/src/main.rs` and `crates/clm/src/upgrade_gate.rs`. It is in force for every `impl/*` branch whose merge base with `main` holds this change, from the land of this branch, with no build.

**The mutation pass, on here once it works** (`ci`, `config`).
- `ci/mutate-script`: the nextest filter names a test target by `binary_id`; StrykerJS takes its options from a configuration file the script writes; a tool that fails for any reason but its mutants' outcomes fails the script, so the pass records nothing and the requirements stay due; and before the plan the script judges a fixture of its own with each tool the plan needs.
- `ci/coverage-script`: every `SF:` path is relative to the repository root.
- `config/verify-here`: `[verify] mutate = ".github/scripts/mutate.sh"`. It needs `ci/mutate-script`, so `config`'s build, which makes the edit to `clm.toml`, comes after the script's.
- `config/health-lcov-here`: `[health] lcov = ".clm/verify-build/lcov.info"`, the copy the pass leaves in the place `clm verify` keeps from run to run. It needs `ci/coverage-script`.

**The Contracts** (`check`, `check.coverage`). Both say who reads coverage directly and who through `clm-check`.

**Decisions.** `path-ownership-by-prompt` is deleted, and the five decisions that cite it are corrected. `own-files-at-the-gate`, `mutation-after-land` and `template-change-ships-its-migration` each gain what this change settled.

**Backlog.** `the-written-exception-to-path-ownership-lets-a-build-re-cite-a` is resolved: the guide's list of what a build may change, rewritten by `20261114-proof-is-real`, already says a build may add, change or delete a test that cites it in another component's file, and the land now reads it the same way. `a-template-and-its-migration-live-in-different-components` records the direction settled here. `the-build-prompt-and-skills-still-tell-sessions-what-programs-now-refuse` gains the skill text that says one claim carries a fix across components.

## What it ends

Neither is a requirement, a scenario or a test. The human agreed to both in the session that proposed this change.
- **A build of `init` writing `upgrade`'s migration.** Refused from this land until the two share a component.
- **One claim carrying a fix across components.** Each component's files are fixed under its own claim.

## What a land pays

Nothing new. The ownership rule was built by `20261114-proof-is-real` and costs tens of milliseconds; the mutation pass runs after the land, outside the check lock.

## What an adopter's project sees

- `clm guide constraints` prints `path-ownership` as checked once the root names its wiring. A project that sets no `wiring` is judged as before.
- Nothing else: the root's `wiring`, the two scripts and the two settings are this repository's own. No file `clm init` writes, no settings key and no format changes, so no migration.

## What a build in flight meets

- An `impl/*` branch cut before this lands is judged by the root at its own merge base, which sets no `wiring`, so it lands as before. Once `clm land` or `clm rebase` merges this `main` into it, the rule applies to its whole diff.
- The mutation pass stays off until `config`'s build lands, which waits for `ci`'s.

## Open questions

- **Nothing in the land check runs the real mutation tools.** `ci/mutate-script`'s tests still use fakes; the fixture the script judges at each pass is what proves the tools, after the land. A build of `ci` that can't make StrykerJS judge the fixture under TypeScript 7 leaves the requirement unbuilt with a question item, and `config/verify-here` waits behind it.
- **A failed pass is one line on stderr.** `verify/mutation-pass` prints `mutation: <command> failed with status <n>; nothing recorded` and the verification still passes. An unattended loop shows it to nobody. Whether a failed pass should file an item is left for the first one seen.
- **`clm health` in a linked worktree** finds no tracefile, since `.clm/verify-build` is under the clone's main checkout, and prints `not found`.
- **Tests written before the pass** are still never judged (`backlog/tests-written-before-the-mutation-pass-were-never-judged`).

## Constraints

- `clm land` gains nothing.
- No requirement, scenario or test is removed.
- The freeze holds: this is the plan's own work.
