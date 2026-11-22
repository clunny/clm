---
status: done
resolves:
  - a-live-change-and-an-archived-one-with-the-same-id-load-as-the-archived-one-alone
---

## Intent

A live change and an archived one that share an ID are reported by `clm check`, naming both `change.md` files, and the graph keeps the live one, so a change still to be built or archived never disappears behind an archived copy without a word.

## Problem

`clm_model::load_files` (`crates/clm-model/src/lib.rs`) loads live changes, then archived ones, into one map keyed by ID. When `changes/x/` and `changes/archive/x/` both hold a `change.md` that loads, the archived copy overwrites the live one: the graph holds one change `x`, marked archived, and no violation names the pair. `check.coverage`'s progress skips archived changes, so the live `x` is never counted, `clm rebase` never archives it, and nothing reports that it exists. No requirement states what happens to such a pair: `parse-changes` says only that both directories are loaded, and `unique-requirement-ids` covers requirement slugs, not changes. The fix restores no written rule, so it is a requirement (`code-fix-needs-a-written-rule`).

## Approaches

- **Taken: the model reports the pair and keeps the live change** (`model/unique-change-ids`). The map is the model's, and only loading sees both copies before one replaces the other, so the rule sits where the fault is; `clm check` already prints every loading violation, so it reports the pair with no rule of its own. The live change is kept because it is the one with work left (a status that may be pending, deltas still to build, an archive still to make); the archived one is a record whose deltas `specs/` already holds. Since the violation fails `clm check` and so every land, which copy the graph keeps matters only to what commands show until a person removes one, and showing the live one keeps it counted.
- **Not taken: a `check` rule over duplicate change IDs.** By the time `check` reads the graph, one copy is gone, so `check` would have to read `changes/` again on its own, a second loader of the same directories (`reuse-first`).
- **Not taken: keep the archived change, or neither.** Keeping the archived one is what hides the live one today; keeping neither hides both, and with them any backlog rule that reads the archived change's deltas.

## Outcome

`model` gains `unique-change-ids`: a live and an archived change that both load and share an ID are reported at the live `change.md`, naming the archived one, and the graph keeps the live change. Its build changes `load_files` (or `change::load_changes`) alone. The Contract already says a record that breaks a rule stays unless its requirement says it is not loaded, so it needs no edit. A pair where either copy fails to load is already reported by `parse-changes` and makes no second report. Leaving the archived copy out also leaves out its `resolves`, its `deleted-tests.txt` and any `check/archive-done` report on it; that is accepted, since the pair already fails `clm check` until a person removes one copy.

`clm rebase` changes with no code of its own: `clm-sync` already prefers the live copy when it picks the changes a branch adds, and `clm-regenerate` keeps the violations under the live change's directory, so regenerating such a change now stops on `model/unique-change-ids` instead of applying the archived copy's deltas, which agrees with the pair failing every land.

One piece of work stays out: the comment in `clm-apply`'s `archive` (`crates/clm-apply/src/lib.rs`) that says a change both live and archived loads as the archived one alone becomes false once this is built (`deferred-work-resolves-in-code`, `backlog/clm-apply-s-archive-comment-says-a-live-and-archived-pair-loads-as-the-archived-one`). Its check on disk stays, since an archive directory holding no change makes no pair.

## Constraints

- No other component's code changes: `clm check` prints the model's loading violations as they are, and `sync/archive-complete`'s scenario "archive's place taken" puts a file, not a change, at `changes/archive/x`, so it makes no pair and keeps its outcome.
- No edge cases or workarounds are known.
