---
status: applied
resolves: [the-replica-keeps-marker-counts-a-scanner-change-has-made-stale]
---

## Intent

The web app's replica counts each file's markers once, when it fetches the file, and keeps the count. When a release changes which markers count, as `20261124-a-marker-above-a` did for a marker on a `describe` block's own lines, a replica the Worker already holds keeps the old counts until each file changes, so the web app's coverage disagrees with `clm coverage` on the same commit (`replica/snapshot-matches-clm`). This change makes the replica record the release that saved it and, when a different release loads it, count its markers again: from the text it keeps, and by fetching the other files again.

## Problem

`Held.markers` (`crates/clm-replica/src/lib.rs`) is filled by `clm_coverage::markers_in` in `State::put` and never again. The replica keeps the text of the spec files only, so it can't rescan the source files it holds. Nothing in the saved state says which scanner counted, so a new release can't tell its own counts from an older one's.

## Approaches

1. **Drop the store on a new release** (as wave 6's `web.worker.replica/fetches-binary-assets` drops a store whose object record names no version). Simple, but every repo then answers 503 `preparing` after each release until its first build ends, and refetches its spec files and ledgers too.
2. **A scanner version in `check.coverage`, bumped by hand when the rule changes.** Rescans only when needed, but rests on someone remembering to bump it: nothing checks it, and a code-only fix to the scanner wouldn't bump it.
3. **The release that saved the state, and a rescan in the replica (chosen).** `save` records `clm-replica`'s crate version, which is the workspace's and changes with every release, the only builds the Worker is deployed from (`ci/web-deploy`). On `load` from another version, or none, the replica rescans the text it keeps and forgets the other scannable files, so `pending_fetch` lists them again. The views stay up meanwhile, with coverage indexing after the next recompute, and the runtime fetches them as it already fetches any pending file: in slices, on its alarm. It costs one pass over the repository's non-Markdown blobs per release, and nothing new of the runtime.

## Outcome

- `replica` gains `markers-of-this-release`; its Contract's `load`/`save` line and its Design's "Marker index" say what the saved version means and why.
- `web.worker.replica` changes nothing: its alarm already fetches `pending_fetch` whenever it is non-empty (`alarm-batches`, `first-build`), and a read of an object with no alarm set already arms one. The backlog item names it, but nothing of it changes.

## Edge cases and workarounds

- A store saved before this requirement is built names no version, so the first release that has it rescans every store once. That is the case the item reports (stores counted before `20261124-a-marker-above-a`'s release).
- `asset-hashes` says a store written before hashes were held loads as it did. Such a store also names no release, so it loads with its source files forgotten; its tree, spec text, binary files and views stay as saved, which is what that scenario compares. Its build should draw the old store's fixture from spec files and assets, or compare what `markers-of-this-release` leaves as saved.
- Between a release and the object's next recompute, a read that isn't `fresh` answers the views the older release computed. A `fresh` read recomputes at once, since `load` leaves the replica stale.
- A rollback to an earlier release also differs from the saved version, and rescans.
