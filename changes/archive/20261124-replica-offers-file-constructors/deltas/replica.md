---
component: replica
base: { contract: "e3608a2be0d2f314" }
---

## MODIFIED Contract
Crate `clm-replica`: a repository's derived state, kept up to date from the changes a source reports, and the views computed from it (`web-repo-replica`). It is host-neutral and runtime-neutral: it knows nothing of GitHub, Cloudflare, a browser or a filesystem, and builds for the host and for `wasm32-unknown-unknown`. A runtime (`web.worker.replica` on Cloudflare; later a local `clm serve`) gives it a `Source` and a `Store` and calls it.

- `Replica::default()`: a new replica, holding nothing, which a runtime makes when it starts again from git.
- `Replica::load(store)` and `save(store)`: the state, to and from a `Store` (`get(key) -> Option<bytes>`, `put(key, bytes)`, `delete_all()`), in chunks of at most 1 MiB each, so a store with a per-value limit holds it; `load` gives a new replica from a store that holds none that reads (`stores-in-chunks`).
- `apply(change)`: takes a change a source has already fetched and marks the views stale. The changes are:
  - `Tip { sha, files, removed }`: the default branch at a new commit, with each file that changed and that `wanted` accepts, and the paths removed. Each file is an `Entry { path, sha, size, mode }`, its blob SHA, size and git mode, made into a `File` by one of `File::text(entry, text)`, a file read as text; `File::binary(entry)`, a blob the source says is binary; and `File::unfetched(entry)`, a file the source read nothing of (`files-a-source-gives`). A source builds a `File` only with these, so a field `File` gains breaks no source.
  - `Tree { sha, entries }`: the whole listing at a commit, to rebuild from, or to recover after a force-push.
  - `Ledger { branch, text }`: a ledger branch's `records.jsonl` at its tip, or its deletion (`text: None`).
  - `Ref { name, sha, time, change }`: a ref the views read, moved, created or deleted (`sha: None`). `change` is a spec ref's change files or an `impl/*` head's code key, when the source fetched them.
  - `Pull(Pull)`: a pull request opened, edited or closed (`open` false), with what `live::pulls` reads.
  - `Event(Event)`: `Push` or `Create { branch, at, actor }`, a push or a branch creation, for the activity feed.
- `stale()`: whether a change has been applied since the last `recompute` that computed (`batched-recompute`).
- `keeps_text(path)` and `wanted(path)`: the replica's rule for the files whose text it keeps and the files it reads at all, which a source fetches by (`files-it-fetches`).
- `pending_fetch()`: what the replica still needs before its views are complete: the tree's files it hasn't been given at their listed blob SHA, with their text or marked binary, in the order to fetch them (`files-it-fetches`). The runtime fetches them in slices it can afford and applies each slice as a `Tip` change.
- `recompute(now) -> Recomputed`: computes every view from the state, at once, and returns the time they next go stale without a change (the next claim's expiry), or none; while a regular file of the listing that `keeps_text` accepts is still to be fetched (given with its text or marked binary) it computes nothing (`batched-recompute`).
- `view(name)`: the last computed view, with the commit it describes and the time it was computed, or `Preparing { done }` before the first recompute that has the spec files; nothing for a name that isn't a view's.
- `validate(write)`: checks a web write against the state, giving the refusal its route answers or the record's fields to append (`validate-writes`, `validate-answers`).

The views and their shapes are the ones `web.worker`'s routes answer: `snapshot`, `implementation` (all but the `mine` of each of its `proposals` and `answers`, which is per requester), `capabilities` and `architecture`. A view's shape changes only with that contract.

## ADDED Requirements

### Requirement: files-a-source-gives
A `File` made by `File::text(entry, text)` SHALL be held at `entry`'s path, blob SHA, size and mode as read: scanned for markers when it holds markers that count, its text kept when `keeps_text` accepts its path, and held as binary, scanned for none, when its text has a NUL byte in its first 8000 bytes. One made by `File::binary(entry)` SHALL be held as binary, with no text and no markers. One made by `File::unfetched(entry)` SHALL be held in the tree only, so `pending_fetch` still lists it. Each SHALL leave `pending_fetch` and the views as a file of the same fields given today does.

#### Scenario: each kind of file
- GIVEN a replica given a `Tree` change listing `specs/a/SPEC.md`, `src/a.rs`, `src/b.bin`, `src/nul.rs` and `src/c.rs`
- WHEN a `Tip` change carries `File::text` of `specs/a/SPEC.md` and of `src/a.rs` citing a requirement, `File::binary` of `src/b.bin`, `File::text` of `src/nul.rs` whose text holds a NUL byte at offset 10 and a marker after it, and `File::unfetched` of `src/c.rs`, and `pending_fetch` is read
- THEN `pending_fetch` lists `src/c.rs` only; the marker index holds `src/a.rs`'s marker and none from `src/nul.rs`; and the spec file's text is held

#### Serves
- capability/follow-from-browser
- architecture/reuse-first
