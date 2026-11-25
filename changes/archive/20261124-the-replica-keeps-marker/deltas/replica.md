---
component: replica
base: { contract: "90738ec9a8bef19d", design: "0eaaf5d7e9a7d377" }
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
- `recompute(now) -> Recomputed`: computes every view from the state, at once, and returns the time they next go stale without a change (the next claim's expiry), or none; while a regular file of the listing that `keeps_text` accepts is still to be fetched (given with its text or marked binary) it computes nothing (`batched-recompute`).
- `view(name)`: the last computed view, with the commit it describes and the time it was computed, or `Preparing { done }` before the first recompute that has the spec files; nothing for a name that isn't a view's.
- `validate(write)`: checks a web write against the state, giving the refusal its route answers or the record's fields to append (`validate-writes`, `validate-answers`).

The views and their shapes are the ones `web.worker`'s routes answer: `snapshot`, `implementation` (all but the `mine` of each of its `proposals` and `answers`, which is per requester), `capabilities` and `architecture`. A view's shape changes only with that contract.

## MODIFIED Design
The state is plain data: maps from path to text and blob SHA, the marker index, ledger sources by branch, refs, pull requests and at most 100 events. It is serialised with `serde_json` and split into chunks by `save`. The spec files are those `keeps_text` accepts: the files the model reads, `clm.toml` and every `.gitattributes`. Every file under `changes/` is kept, the archive's deltas with its `change.md`, since the coverage's moves and the capabilities report's change counts read archived deltas.

**Recompute.** It runs the code the Worker ran on request before this component:
- the snapshot: `clm_model::load_files`, then `clm_check::report` with the index's markers, then `clm_view`'s `spec` model with sizes from `clm_check::size_of` over the listing;
- the implementation's shared part: `flight_view`, `blockers_view`, `taken_from_refs`, the activity feed, the pages with their `checked` and the open proposals;
- the capabilities report: `clm_effort::report` with the flags' text, no history;
- the architecture report: `clm_effort::architecture` with the current requirements from the index once it is complete.

Computation is whole-state: nothing is recomputed per component until a repository shows it needs that (`yagni`).

**Marker index.** A `Tip` change rescans only the files it carries, with `clm_coverage::markers_in`. A file is binary when the source says so, or when its text has a NUL byte in its first 8000 bytes, as `clm size` decides. Which markers count is the scanner's rule, and a release may change it (`20261124-a-marker-above-a` did), so the saved state names the release that counted its markers: `clm-replica`'s own crate version, which is the workspace's, and the Worker is deployed only from a release (`ci/web-deploy`). A different release, or none, on `load` means every held marker may be wrong. The replica keeps no text of the files outside the spec files, so it can't rescan them in place: it forgets them, and the runtime fetches them again through `pending_fetch` in the slices it already fetches a first build's files in, while the views stay up with coverage indexing. A per-scanner version, bumped by hand when the rule changes, would rescan less often but rests on someone remembering; a rescan costs one pass over the repository's non-Markdown blobs per release and asks the runtime for nothing new.

**Writes.** `validate` answers what the Worker's routes checked before: a decision's entry, its version against the entry's version now, and whether the tip is signed; a proposal's slug against the backlog, `resolved` and the open proposals. Since the replica holds the ledgers and the tip, the check needs no request. The runtime first brings the replica up to the tip, so "now" means the tip, as `decide` requires.

## MODIFIED Requirements

### Requirement: stores-in-chunks <!-- base: 548edbd9f85ec3af -->
`save` SHALL write no value over 1 MiB and leave in the store only what it wrote, and `load` from what `save` wrote SHALL give the same state, save for the markers another release counted (`markers-of-this-release`). `load` from a store that holds no replica, whose chunk count names a chunk it lacks, or whose chunks don't read as a saved replica, SHALL give a new replica, the same as `Replica::default()`.

#### Scenario: a large repository
- GIVEN a replica whose serialised state is 5 MiB
- WHEN it is saved to a fake store, then loaded from it, and then a replica of a few KiB is saved to the same store
- THEN every value the store holds is at most 1 MiB, the loaded replica's views equal the original's, and after the second save the store holds only that save's values

#### Scenario: an empty or unreadable store
- GIVEN an empty store, a store whose count is 2 holding only chunk 0, and a store whose count is 1 and whose chunk 0 holds `{`
- WHEN a replica is loaded from each
- THEN each gives `Replica::default()`

### Requirement: asset-hashes <!-- base: f1d76b7178ece696 -->
`keeps_hash(path)` SHALL be true exactly for a path under `specs/` with an `assets` directory in it, as the model reads a component's assets. A `File` made by `File::asset(entry, bytes)` for such a path SHALL be held as binary with the hash the model gives those bytes as an asset, keeping no bytes, and the snapshot SHALL list it in its component's `assets` with that hash; one made by `File::binary(entry)` for such a path SHALL be held as binary and left out of `assets`, and `pending_fetch` SHALL list neither. The model SHALL be given the path of every binary file `keeps_hash` accepts that the replica holds, so the snapshot's `violations` hold what `model/node-assets` gives for it, as `clm check` prints. A store written before hashes were held SHALL load as it did, save for the markers `markers-of-this-release` counts again, since such a store names no release.

#### Scenario: a binary asset
- GIVEN the paths `specs/b/assets/logo.png`, `specs/b/x/assets/y.bin`, `changes/c/assets/b/logo.png`, `src/assets/z.bin` and `specs/b/SPEC.md`; a replica given `specs/b/assets/logo.png` by `File::asset` with 3,000 bytes holding a NUL; a second replica given it by `File::binary`; a replica given `specs/assets/x.png`, an asset of the root component, by `File::asset`; and a store a replica saved before this requirement was built
- WHEN `keeps_hash` is asked of each path, the snapshot of each replica is read, the first is saved and loaded again, and the old store is loaded
- THEN `keeps_hash` is true for the first two only; the first snapshot's `b` lists `logo.png` in `assets` with the hash `clm` gives that file, and so does the loaded one, whose saved state holds none of the 3,000 bytes; the second lists no `logo.png`; neither replica's `pending_fetch` lists it; the root asset's replica's `violations` hold the `model/node-assets` line `clm check` prints for `specs/assets/x.png` on a checkout; and the old store loads the replica it held, its scannable files that aren't spec files to fetch again (`markers-of-this-release`)

#### Serves
- capability/follow-from-browser
- capability/see-project-state

## ADDED Requirements

### Requirement: markers-of-this-release
`save` SHALL record, with the state, the version of the `clm-replica` crate that saved it. `load` from a store whose state another version saved, or names none, a store saved before this requirement was built included, SHALL give the replica it held with each held file's markers counted by this release's `clm_coverage::markers_in`: a file whose text it keeps scanned again from that text when `File::text` would scan it (`files-a-source-gives`: not Markdown, not held as binary), and a file `wanted` accepts that is neither held as binary nor kept as text no longer held, so `pending_fetch` lists it again and, once a `recompute` computes, the snapshot reports coverage as indexing until it is given again; its tree, spec files' text, binary files, ledgers, refs, pull requests, events and last views SHALL stay as saved, and `stale()` SHALL be true. `load` from a store this version saved SHALL give the state as saved, as `stores-in-chunks` states.

#### Scenario: another release saved it
- GIVEN a ready replica, its index complete, holding `specs/b/SPEC.md`, `src/a.rs` with a test citing `b/x` at its current version, `specs/b/assets/check.ts` with a test citing `b/y`, `src/b.bin` held as binary, and `README.md` listed; saved to a store whose recorded version is then changed to `0.0.0`, and saved to a second store whose state is then written with no version
- WHEN a replica is loaded from each store, `pending_fetch` and `stale()` are read, `recompute` runs and the snapshot is read; then `src/a.rs` is given again with its text, and `recompute` runs
- THEN each loaded replica's `pending_fetch` lists `src/a.rs` only, `stale()` is true before the first recompute, and `view("snapshot")` before it describes the saved commit; after it the snapshot reports coverage as indexing, and the marker index holds `check.ts`'s marker citing `b/y` and none from `src/a.rs`; after `src/a.rs` is given again, its views equal those of the replica that was saved

#### Scenario: this release saved it
- GIVEN the same ready replica, saved to a store by this release
- WHEN a replica is loaded from it
- THEN it equals the replica that was saved, `pending_fetch` is empty and `stale()` is false

#### Serves
- capability/follow-from-browser
- architecture/no-silent-drift
