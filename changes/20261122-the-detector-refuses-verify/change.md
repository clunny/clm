---
status: applied
resolves: [the-detector-refuses-verify-s-move-off-run-check-before-land-can]
---

## Intent

`clm unreferenced` stops reporting a public item that a `clm:remove-by <change-id>` marker holds: the old path of an expand-contract, which nothing is meant to use once its users have moved and which `check/remove-by-markers` already keeps only while its removal is planned. So `verify`'s move off `clm_land::run_check` can land before `land` removes it, and so can the last move of any later expand-contract. `verify/verify-run` waits for the detector's build through a `#### Needs`, instead of the item's `blocks`. Resolves `backlog/the-detector-refuses-verify-s-move-off-run-check-before-land-can`.

## Problem

`20261122-the-check-runner-s` renames the check runner by `expand-contract`: `land` offers `run_check_as` beside `run_check` (marked `clm:remove-by 20261122-the-check-runner-s`) and moves its own tests to the new form; `verify` moves off `run_check` (`verify/verify-run`); then `land/runner-names-its-caller` removes it. `verify` is `run_check`'s last user, so its move leaves `clm_land::run_check` used by nothing outside `clm-land`, and `clm check`'s `unreferenced-public-items` detector fails the land. `verify`'s build can't add an `--allow` entry (`gate/own-files` admits only dropping one, `a-build-drops-its-allowance`), nor remove `run_check` (`path-ownership`), and `land`'s removal needs `verify`'s move first. Neither step can land.

The design is wrong, not the build: `expand-contract` and `rename-adds-then-removes` make an old path with no user a planned state between the last move and the removal, and the detector, which knows nothing of that plan, treats it as dead code. Every expand-contract whose provider's own tests move off the old path, or whose old path never had one, meets the same wall at its last move. It is checked against `20261122-a-build-that-narrows` (on `main`): that change lets a build drop an `--allow` entry its own narrowing makes stale, the contract end of an allowance; it gives no build a way to add one, which is what this move would need, so it doesn't fix this.

## Approaches

- **Taken: the detector skips an item a `clm:remove-by` marker holds** (`unreferenced-skips-marked-old-paths`). The marker already declares the item is going and names the change that removes it, and `check/remove-by-markers` fails it once that change isn't live, so the skip lasts exactly as long as the plan and needs no list of its own. One requirement of `check.unreferenced`; nothing else changes.
- **Rejected: the change adds the `--allow` entry with the item that ends it.** The entry skips no finding while `verify` still calls `run_check`, so it is a stale-entry finding from this change's land until `verify`'s build, and `clm check` fails at once; a spec branch editing `clm.toml` is also `mixed`.
- **Rejected: one change, one build, moves `verify` and removes `run_check` together.** One build in two components' files breaks `path-ownership` (`expand-contract` rejected it already).
- **Rejected: `clm-land`'s own tests keep calling `run_check`.** A use from the crate's own `tests/` counts, so it would work for this case, but only through a test that exists to quiet the detector, which `runner-names-its-caller`'s build must then delete under an approved `deleted-tests.txt`, and every later expand-contract would have to remember the trick.
- **Rejected: the detector reads whether the named change is live.** `check.unreferenced` depends on `vcs` alone and runs in projects without CLM's specs; `check/remove-by-markers` already judges liveness.

## Outcome

- `check.unreferenced/removal-marked` (added): an item a marker holds is not listed, in Rust, TypeScript and Python; a marker holds an item from the comments and attributes directly before its declaration, or before its `impl` block; the marker's form is `check.coverage`'s; marked items are skipped before the allow-list, so an `--allow` that skips only them is stale. Three scenarios. Its build makes `check.unreferenced` pending; no other test of the component changes.
- `check.unreferenced`'s Contract gains one sentence, its Design a paragraph **Old paths marked for removal**.
- `verify/verify-run` (modified): its `#### Needs` gains `check.unreferenced/removal-marked`, so `verify`'s build, which moves off `run_check` and removes the comment in `crates/clm-verify/src/run.rs` naming the item, waits for the detector's build; nothing else in it changes. `land/runner-names-its-caller` already needs `verify/verify-run`, so the order is detector, `verify`, `land`.
- The item loses its `blocks: [verify/verify-run]` on this branch: the `#### Needs` states the wait from now on, and an item in `resolves` that blocks a requirement the change modifies would hold the change from completing, so the item would never be archived (`apply-deletes-answered-questions`). It stays in the backlog until the change is archived.
- `docs/decisions/unreferenced-skips-marked-old-paths.md` records the choice; `docs/decisions/expand-contract.md` gains a dated line under **Markers**, and `docs/guide/how-it-works.md`'s expand-contract paragraph a sentence.

## Constraints

- `no-dead-code`: the skip covers only code a live change has promised to remove, and `check/remove-by-markers` makes that promise checked; a marker outliving its change fails `clm check`.
- `path-ownership`, `expand-contract`, `rename-adds-then-removes`: unchanged; the steps now land as those decisions intend.
- `reuse-first`: the marker and its liveness check already exist; the detector only reads the marker's form, which it parses itself rather than taking a dependency on `check.coverage` for one private pattern.
- Edge cases known now: in a project without CLM's specs a marker is checked by nothing, and skips its item as long as it stays; a marker in a comment inside the item's body or after it holds nothing; a marker on an `impl` block holds every item of the block, as an entry attribute there does.

## Open questions

None.
