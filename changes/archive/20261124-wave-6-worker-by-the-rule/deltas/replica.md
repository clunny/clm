---
component: replica
base: { contract: "994e48a181174524" }
---

## MODIFIED Contract
Crate `clm-replica`: a repository's derived state, kept up to date from the changes a source reports, and the views computed from it (`web-repo-replica`). It is host-neutral and runtime-neutral: it knows nothing of GitHub, Cloudflare, a browser or a filesystem, and builds for the host and for `wasm32-unknown-unknown`. A runtime (`web.worker.replica` on Cloudflare; later a local `clm serve`) gives it a `Source` and a `Store` and calls it.

- `Replica::default()`: a new replica, holding nothing, which a runtime makes when it starts again from git.
- `Replica::load(store)` and `save(store)`: the state, to and from a `Store` (`get(key) -> Option<bytes>`, `put(key, bytes)`, `delete_all()`), in chunks of at most 1 MiB each, so a store with a per-value limit holds it; `load` gives a new replica from a store that holds none that reads (`stores-in-chunks`).
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

## ADDED Requirements

### Requirement: asset-hashes
`keeps_hash(path)` SHALL be true exactly for a path under `specs/` with an `assets` directory in it, as the model reads a component's assets. A `File` made by `File::asset(entry, bytes)` for such a path SHALL be held as binary with the hash the model gives those bytes as an asset, keeping no bytes, and the snapshot SHALL list it in its component's `assets` with that hash; one made by `File::binary(entry)` for such a path SHALL be held as binary and left out of `assets`, and `pending_fetch` SHALL list neither. The model SHALL be given the path of every binary file `keeps_hash` accepts that the replica holds, so the snapshot's `violations` hold what `model/node-assets` gives for it, as `clm check` prints. A store written before hashes were held SHALL load as it did.

#### Scenario: a binary asset
- GIVEN the paths `specs/b/assets/logo.png`, `specs/b/x/assets/y.bin`, `changes/c/assets/b/logo.png`, `src/assets/z.bin` and `specs/b/SPEC.md`; a replica given `specs/b/assets/logo.png` by `File::asset` with 3,000 bytes holding a NUL; a second replica given it by `File::binary`; a replica given `specs/assets/x.png`, an asset of the root component, by `File::asset`; and a store a replica saved before this requirement was built
- WHEN `keeps_hash` is asked of each path, the snapshot of each replica is read, the first is saved and loaded again, and the old store is loaded
- THEN `keeps_hash` is true for the first two only; the first snapshot's `b` lists `logo.png` in `assets` with the hash `clm` gives that file, and so does the loaded one, whose saved state holds none of the 3,000 bytes; the second lists no `logo.png`; neither replica's `pending_fetch` lists it; the root asset's replica's `violations` hold the `model/node-assets` line `clm check` prints for `specs/assets/x.png` on a checkout; and the old store loads the replica it held

#### Serves
- capability/follow-from-browser
- capability/see-project-state

### Requirement: refs-the-views-read
`REF_PREFIXES` SHALL be `impl/`, `spec/`, `autosync/spec/` and `propose/`, and `reads_ref(name)` SHALL be true exactly for one of them followed by a non-empty name with no `/`. A `Ref` change naming a ref `reads_ref` refuses SHALL be held by no view.

#### Scenario: nested and empty names
- GIVEN the names `impl/a`, `spec/x`, `autosync/spec/x`, `propose/p`, `impl/archive/a`, `impl/`, `autosync/x` and `main`, and a replica whose tip holds components `a`, `archive` and `x` and a backlog item `p`
- WHEN `reads_ref` is asked of each, and the replica, given a `Ref` change for each, recomputes
- THEN `reads_ref` is true for the first four only; the implementation view's flight lists `a` claimed and neither `archive` nor `x` claimed, and its `taken` holds `p`'s claim and nothing for `impl/`, `autosync/x` or `main`

#### Serves
- capability/follow-from-browser
- architecture/reuse-first

## MODIFIED Requirements

### Requirement: snapshot-matches-clm <!-- base: 3e19d262066eb412 -->
Once its marker index is complete, the replica's snapshot of a commit SHALL report the same components, component statuses, requirement coverage, changes and backlog items as `clm status`, `clm coverage` and `clm backlog` run on a checkout of that commit, each component's `status` being the word `clm status` writes on that component's row; a `spec` model equal to what `clm view spec --format json` prints there, with the sizes `snapshot-size` gives; each component's assets in `assets` with the hashes a checkout's give, text assets and binary assets given with their bytes (`asset-hashes`) alike; `decisions`, each decision's `name`, `title`, `date` and `status` lowercased; and `violations`, the violations the model gives in loading the commit's files, each as `clm check` prints one.

#### Scenario: fixture repo
- GIVEN a fixture repo with an unbuilt and an active component, a test citing one requirement at its current version and another at an older one, a Markdown file citing a third, an applied change, a backlog item, a text asset `specs/b/assets/note.txt`, a binary asset `specs/b/assets/logo.png` given with its bytes, a binary file outside the spec directories, and `docs/decisions/d.md` with title `D`, date `2026-10-01` and status `accepted`
- WHEN a replica is built from it, and the `clm` commands are run on a checkout of it, and again with a backlog item whose frontmatter doesn't parse added
- THEN both report the same components, statuses (the unbuilt component's `status` the word on its `clm status` row), coverage, changes and backlog, the snapshot's `spec` is the model `clm view spec --format json` prints, `note.txt`'s and `logo.png`'s hashes are the checkout's, `decisions` is `d` with `D`, `2026-10-01` and `accepted`, and `violations` is empty, and then holds the line `clm check` prints for that item

#### Serves
- capability/follow-from-browser
- capability/see-project-state
- architecture/no-silent-drift
