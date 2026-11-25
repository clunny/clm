---
component: replica
base: { contract: "b9bd7ad1d6eb68d5" }
---

## MODIFIED Contract
Crate `clm-replica`: a repository's derived state, kept up to date from the changes a source reports, and the views computed from it (`web-repo-replica`). It is host-neutral and runtime-neutral: it knows nothing of GitHub, Cloudflare, a browser or a filesystem, and builds for the host and for `wasm32-unknown-unknown`. A runtime (`web.worker.replica` on Cloudflare; later a local `clm serve`) gives it a `Source` and a `Store` and calls it.

- `Replica::default()`: a new replica, holding nothing, which a runtime makes when it starts again from git.
- `Replica::load(store)` and `save(store)`: the state, to and from a `Store` (`get(key) -> Option<bytes>`, `put(key, bytes)`, `delete_all()`), in chunks of at most 1 MiB each, so a store with a per-value limit holds it; `load` gives a new replica from a store that holds none that reads (`stores-in-chunks`). `save` records the release that saved the state, and `load` of a state another release saved counts its markers again (`markers-of-this-release`).
- `apply(change)`: takes a change a source has already fetched and marks the views stale. The changes are:
  - `Tip { sha, files, removed }`: the default branch at a new commit, with each file that changed and that `wanted` accepts, and the paths removed. Each file is an `Entry { path, sha, size, mode }`, its blob SHA, size and git mode, made into a `File` by one of `File::text(entry, text)`, a file read as text; `File::binary(entry)`, a blob the source says is binary; `File::asset(entry, bytes)`, a binary blob with its bytes, of which the replica keeps only the hash; and `File::unfetched(entry)`, a file the source read nothing of (`files-a-source-gives`, `asset-hashes`). A source builds a `File` only with these, so a field `File` gains breaks no source.
  - `Tree { sha, entries }`: the whole listing at a commit, to rebuild from, or to recover after a force-push.
  - `Ledger { branch, text }`: a ledger branch's `records.jsonl` at its tip, or its deletion (`text: None`).
  - `Ref { name, sha, time, change }`: a ref the views read, moved, created or deleted (`sha: None`). `change` is a spec ref's change files or an `impl/*` head's code key, when the source fetched them.
  - `Pull(Pull)`: a pull request opened, edited or closed (`open` false), with what `live::pulls` reads.
  - `Event(Event)`: `Push` or `Create { branch, at, actor }`, a push or a branch creation, for the activity feed.
- `stale()`: whether a change has been applied since the last `recompute` that computed (`batched-recompute`).
- `keeps_text(path)` and `wanted(path)`: the replica's rule for the files whose text it keeps and the files it reads at all, which a source fetches by (`files-it-fetches`); `keeps_hash(path)`, the binary files whose hash it keeps for the snapshot's `assets` (`asset-hashes`). `REF_PREFIXES` and `reads_ref(name)`: the refs its views read, which a source lists and applies by (`refs-the-views-read`).
- `pending_fetch()`: what the replica still needs before its views are complete: the tree's files it hasn't been given at their listed blob SHA, with their text or marked binary, in the order to fetch them (`files-it-fetches`). The runtime fetches them in slices it can afford and applies each slice as a `Tip` change.
- `recompute(now) -> Recomputed`: computes every view from the state, at once, and returns the time they next go stale without a change (the first second a live claim reads lapsed), or none; while a regular file of the listing that `keeps_text` accepts is still to be fetched (given with its text or marked binary) it computes nothing (`batched-recompute`).
- `view(name)`: the last computed view, with the commit it describes and the time it was computed, or `Preparing { done }` before the first recompute that has the spec files; nothing for a name that isn't a view's.
- `validate(write)`: checks a web write against the state, giving the refusal its route answers or the record's fields to append (`validate-writes`, `validate-answers`).

The views and their shapes are the ones `web.worker`'s routes answer: `snapshot`, `implementation` (all but the `mine` of each of its `proposals` and `answers`, which is per requester), `capabilities` and `architecture`. A view's shape changes only with that contract.

## MODIFIED Requirements

### Requirement: batched-recompute <!-- base: d88f848bb084e2a5 -->
`apply` SHALL never recompute, and `view` SHALL answer the last computed views however many changes have been applied since, until `recompute` runs. `stale()` SHALL be true from any `apply` until a `recompute` that computes, and false after one. Before any `Tree` change, and while a regular file of the listing that `keeps_text` accepts has still to be fetched (given with its text or marked binary), `recompute` SHALL compute nothing, leaving every view and `stale()` as they were, and return none. `view` SHALL answer nothing for a name other than `snapshot`, `implementation`, `capabilities` and `architecture`. `recompute` SHALL return the time the implementation view next changes without a change: of the live `impl/<component>` and `propose/<item>` claims, the earliest time one lapses, the first second at which `orchestrator`'s rule reads it lapsed (its tip older than the 4-hour expiry), one second past 4 hours after its tip's time; or none when no claim is live.

#### Needs
- web.worker.replica/alarm-batches
- web.worker.replica/times-as-instants

#### Scenario: three pushes, one recompute
- GIVEN a replica whose views describe `111`
- WHEN pushes to `222`, `333` and `444` are applied, then `view("snapshot")` is read, then `recompute` runs and `view("snapshot")` is read again
- THEN the first read describes `111` and the second `444`, and `stale()` is true before the recompute and false after it

#### Scenario: a claim lapses
- GIVEN a replica holding `impl/a` pushed at 10:00 with the 4-hour expiry, and no other claim
- WHEN `recompute` runs at 11:00, at 14:00, at 14:00:01 and at 15:00, and, on a replica whose only claim is `propose/x` pushed at 10:30, at 11:00
- THEN they return 14:00:01, 14:00:01, none and none, and 14:30:01

#### Scenario: spec files still to fetch
- GIVEN a new replica given only a `Tree` change at `111` listing spec files, and a ready replica at `111` given a `Tree` change at `222` that lists a spec file whose text it doesn't hold
- WHEN `recompute` runs on each
- THEN it returns none; the first's `stale()` is true and its `view("snapshot")` is `Preparing`, and the second's `view("snapshot")` still describes `111`

#### Scenario: an unknown view
- GIVEN a ready replica
- WHEN `view("nothing")` is read
- THEN it answers nothing
