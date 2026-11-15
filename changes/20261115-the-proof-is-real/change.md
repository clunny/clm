---
status: applied
resolves: [the-proof-is-real-upgrade-note-says-a-foreign-change-needs-review]
---

## Intent

The upgrade note `20261114-proof-is-real`, not yet in a release, tells an adopter the wrong thing about `wiring`: that a branch changing files its component doesn't own "needs review". `clm land` refuses such a build branch, and no review lets it through (`land/build-holds-own-files`). The note also doesn't tell a project whose tests share files to give them to their components with `tests` globs before it names `wiring`. Fix the note before the release that first ships it, after which `upgrade/shipped-migrations-frozen` freezes it.

## Problem

`crates/clm-upgrade/src/migrations/20261114-proof-is-real/note.md` says `wiring` switches on a rule under which "a branch that changes files its component doesn't own needs review". What `land/build-holds-own-files` does is refuse an `impl/<component>` branch that changes such a file, before any check runs; an approval doesn't lift it. An adopter who reads the note would set `wiring` expecting a review prompt and get refused builds instead. A project whose tests sit in shared files, as this one's sat in the root's, is the one that meets it: once `own-files-at-the-gate` loses the clause "a file that cites the component", a build can change such a test only when a `tests` glob gives the file to its component. `upgrade/migration-markers-in-tests` asks only that the note name `wiring`, so no requirement as written states the correct wording, and the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

- **Reword the requirement so it states what the note says of the ownership rule, and test the wording.** Taken: the note is unreleased, so it may still change, and its test then holds it to the rule.
- **Fix the note on a code branch alone.** Not taken: no requirement states the wording, so a test of it would cite nothing that says it.
- **Accept the note.** Not taken: it gives an adopter the wrong expectation of a rule that refuses lands.

## Outcome

- **`upgrade/migration-markers-in-tests`** says the note tells an adopter that `clm land` refuses a build's branch changing a file the rule doesn't count as its own, which no review lets through, worded so it holds before and after `gate/own-files` drops its clause (the architecture audit's finding under `needs-alone-order`), and that a project whose tests share files gives them to their components with `tests` globs before naming `wiring`; it never says such a branch needs review. Its scenario "the note" checks for `clm land` refuses and `tests` glob, and for the absence of `needs review`. `upgrade`'s build rewrites the note and its test.
- **Its `#### Needs` drops `gate/own-files`.** The note describes `wiring` and the refusal as `clm land` does them now, which was built at `gate/own-files`' earlier version; its current version only drops the clause, which the note's wording leaves open, and is blocked by four backlog items and `clm/routes-only`. Keeping the need would hold this fix behind them, past the release that ships the note and freezes it (`upgrade/shipped-migrations-frozen`). The note's advice on `tests` globs holds with the clause or without it.

## Constraints

- **Before the next release.** The migration is not in `v0.26.0` and not in `crates/clm-upgrade/migrations.lock`; once a release ships it, the lock freezes the note.
- **No edge cases or workarounds** are known.
