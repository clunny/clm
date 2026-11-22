---
status: applied
resolves: [a-build-that-narrows-a-public-call-clm-toml-allows-cannot-land-on-its-own]
---

## Intent

A build that makes a public item private can land on its own again: `gate/own-files`' `foreign_files` leaves out an edit of the root's `clm.toml` that only deletes lines and, read as TOML, only drops `--allow` pairs from detectors' commands, so the narrowing build takes out the entry its own work makes stale. `clm land`'s claim test reads `foreign_files`, so it follows with no change of its own.

## Problem

An `--allow` entry in `clm.toml`, `config`'s file, keeps a public item that nothing outside its library uses (`unreferenced-allow-then-narrow`). When a later requirement makes the item private, the entry skips no finding, and `check.unreferenced/allow-list` reports it, so `clm check` fails. The narrowing build may not delete it, since `gate/own-files` and `land/build-holds-own-files` count `clm.toml` as `config`'s; nor can `config` delete it first, since the detector then reports the still-public item. `unreferenced-allow-then-narrow` let the narrowing build delete the entry in the same diff, as an exception to `path-ownership` like the root's dispatch files, and `config`'s Design still says so; when the dispatch files became the root's `wiring`, the exception was not carried into the gate's rule. The design is wrong, not the code: the gate and the Design contradict each other.

The expand end the item also records (a build adding a public call another component will adopt) needs nothing: the call is proven from its crate's own tracked tests.

## Approaches

- **Taken: the gate admits a dropped allowance** (`a-build-drops-its-allowance`). One more case in `foreign_files`' list of files a build may change: the CLM root's `clm.toml`, modified, its text at the head the merge base's with whole lines taken out, and both reading as TOML and equal once `"--allow"` elements with the element after each are taken out of the merge base's `[[detectors]]` commands. Dropping an allowance can only make a detector stricter, and the check of the merged tree fails if the entry was still needed, so the edit excuses nothing and needs no decision outside the build. That answers the objection that sank the first design the item records.
- **Rejected, as the item records: a mark in the item's own file.** A build could excuse its own item.
- **Rejected, as the item records: the Contract as the allowance, hiding a changing component's items.** No limit while a requirement stays blocked.
- **Rejected: `clm.toml` in the root's `wiring`.** Every build could change any key, and add allowances for itself.
- **Rejected: a stale entry that doesn't fail.** Nothing would make anyone take it out.

The decision records all five.

## Outcome

- `gate/own-files` (modified): the new case in `foreign_files`, and two scenarios, "an allowance the build no longer needs" and "an edit to clm.toml that isn't only a dropped allowance". The component a code branch is read as follows from `foreign_files`, so a branch that narrows `a` and drops `a`'s entry is `a`'s, and one that only drops an entry is `config`'s.
- `gate`'s Design: three kinds of file are a build's beside its own.
- `config`'s Design: the allow-list paragraph says the gate admits the deletion, and no longer that each narrowing item names `config` for it.
- `docs/guide/how-it-works.md`: an exception "An allowance it makes stale" under "What a build may change". `docs/decisions/unreferenced-allow-then-narrow.md` gains a dated line pointing to the new decision.

No `check.unreferenced` requirement changes: the detector's stale-entry finding stays as it is, and is what makes the dropped entry safe.

## Constraints

- `path-ownership`: the constraint already leaves what else a build may change to the guide; the guide gains the case.
- `land/build-holds-own-files` and the gate's `gate/own-files` block line keep their wording ("its own files, the wiring and a capability test it serves"); the new case is named in the guide and the decision rather than in every refusal.
- `ARCHITECTURE.md`'s `path-ownership` `#### Untested` sentence ("The land judges which files a build changes, not the edit") now names the one edit judged by a program, the architecture audit's finding.
- Edge cases known now: a pair written over two lines is admitted only when both lines go; a `clm.toml` that doesn't read as TOML on either side is never admitted; a comment line may be deleted but never reworded.

## Open questions

- **A light branch.** `gate/light` lists its own files and doesn't take the new case, so a light branch that narrows an item and drops its entry is `mixed`. Left: a light branch is admitted only for being small, and a narrowing is a code branch's work; the narrowing build lands as `code`.
- **`--entry` pairs.** Dropping one also only makes the detector stricter, but nothing needs it today (`yagni`), so it is not admitted.
