---
status: applied
resolves: [an-i-o-failure-while-clm-apply-writes-leaves-the-files-already, clm-apply-and-clm-archive-fall-short-of-two-requirements]
---

## Intent

`clm apply` and `clm rename` leave the tree as they found it when a write fails part way, `clm apply` names every fault of a delta in one run, and `clm archive` names an existing archive entry as the reason it refuses, never "no unarchived change". The three gaps were found in wave 1 of phase 5 (`backlog/an-i-o-failure-while-clm-apply-writes-leaves-the-files-already`, `backlog/clm-apply-and-clm-archive-fall-short-of-two-requirements`). No change or code fix since 2026-11-12 touches them.

## Problem

- **A failed write leaves a half-applied tree.** `apply` validates, then performs its `FileOp`s one by one (`crates/clm-apply/src/lib.rs`, `apply`); when one fails it returns `apply/atomic` `write failed` and the files written before stay written. `atomic` says a change with a violation leaves every file unchanged, but no requirement says what a failed write leaves, and no test reaches the path.
- **`clm rename`'s undo misses the apply's files.** `rename` remembers what each of its own writes replaced and restores them on a failure (`Writes` in `crates/clm-rename/src/build.rs`), but the files `clm_apply::apply` wrote on its behalf are in no such record: a failure after the apply (rewriting a test marker, rebasing another change's delta, git) leaves `specs/` applied, `change.md` `applied` and the answered questions deleted, around a restored `deltas/`.
- **One fault per run.** `validate` stops at a delta's first frontmatter line at fault (`edit_frontmatter` returns the first) and then skips that delta's sections (`return None` before `edit_body`), where the Contract says it returns every reason and `atomic` says it prints all of them. A session fixing the delta learns its faults one `clm apply` at a time.
- **The wrong refusal.** `archive` looks the ID up in the loaded graph; with both `changes/x/` and an archived `x` the archived one is the one loaded, so `clm archive x` says ``no unarchived change `x` ``, where `archive-command` reserves that message for an ID that names no change outside the archive. Found by reading the code, not a run.

No requirement states the first, second or fourth behaviour as the fix restores it, so each is a spec change (`code-fix-needs-a-written-rule`); the third is folded in with them as a scenario, since the change modifies `atomic` anyway and the scenario pins what "all of them" means for one delta.

## Approaches

- **Taken: undo through a journal.** Before the first op on a path, remember its bytes or its absence; on a failure, put each path back in reverse. It is the shape `rename` already uses for its own writes, so it moves into `clm-apply` as `Journal`, `apply` uses it inside, and `rename` passes its own through `apply_into` and undoes everything once (`reuse-first`): one record of what a run changed, not two that miss each other.
- **Rejected: accept it and state what's left.** Tempting under `yagni`, since a checkout recovers with `git restore`. But `clm apply` runs unattended in builds, `clm rebase` and `clm land`'s regeneration, where a half-applied tree is read as the next state and the fault surfaces far from its cause; and the undo is a few dozen lines `rename` already has.
- **Rejected: write each file to a temporary name and rename it into place.** Each file becomes atomic, the set doesn't: a failure on the fifth still leaves four renamed. It also needs the journal for removals, so it adds work without removing any.
- **Rejected: plan in `clm-apply`, perform in `rename`.** Exposing the ops before they run would let `rename` journal them itself, but leaves `apply` with a second, private undo; one `Journal` type both use is smaller.

## Outcome

- `apply/failed-write` (added): a failed write or removal puts every file back, prints `write failed` naming the file, exits 1; a file that can't be put back is named `not restored`.
- `apply/journal` (added): `Journal` remembers each path once, before its first op, and `undo()` restores in reverse, recreating directories for bytes it writes back and removing directories it created that end empty; `apply_into` performs through the caller's journal.
- `apply/atomic` (modified): a delta whose frontmatter is at fault still has its sections judged; scenario *bad frontmatter and a missing section*.
- `apply/frontmatter` (modified): each key and line at fault is a violation of its own.
- `apply/archive-command` (modified): with `changes/<id>/` holding a change and `changes/archive/<id>/` existing, the refusal names the archive entry; scenario *the archive holds the same change*.
- `apply`'s Contract names `apply_into`, `Journal` and the failed-write exit; its Design says why the frontmatter no longer stops a delta's checks, how the journal undoes, and why `archive` reads the disk.
- `rename/undo-on-failure` (added, needs `apply/journal`): a failure after writing began and before the archive puts back every file, the apply's among them, through one journal.

## Constraints

- No requirement is removed and no scenario renamed or dropped; every sentence holding `SHALL` stays.
- `apply(root, id)` keeps its signature: its other callers (`clm-apply`'s command, the tests of `land`, `sync` and `sync.regenerate`, and `rename` until its build) build unchanged. `rename`'s move from its `Writes` to the journal is `rename`'s own build, after `apply/journal`.

## Edge cases and workarounds

- A path whose bytes can't be read before its op stops the apply there, as a failed write, since it could not be put back.
- The file whose write was refused before it was opened already holds its old bytes: undo leaves it alone, so it is never named `not restored`.
- `clm archive` checks the archive entry before the status, since a change both live and archived loads only as the archived one.
- Putting back can fail too; such files are named, never left silent. Nothing retries.
- The model loads a change both live and archived under one ID once, as the archived one. That a duplicate ID loads silently is the model's to judge; this change only keeps `archive` from being misled by it.

## Open questions

- `rename/undo-on-failure` carries no `#### Serves` block, so it traces to `rename`'s `serves: [architecture/plain-terms]`, which is about names, not about leaving the tree as found (architecture audit, `traced-rules`). Left as is: `rename/refusals` and `rename/finish` trace the same way on `main`, and no capability or constraint names a command's integrity on a failed write; a wider fix of rename's traces is a separate change.
- The failed-write tests make a write fail with a read-only file, which a run as root doesn't refuse; the tests of `clm-checkpoint` already rely on the same (builder check).
