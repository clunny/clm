---
component: replica
base: { contract: "d50087ae078806df" }
---

## MODIFIED Contract
Crate `clm-replica`: a repository's derived state, kept up to date from the changes a source reports, and the views computed from it (`web-repo-replica`). It is host-neutral and runtime-neutral: it knows nothing of GitHub, Cloudflare, a browser or a filesystem, and builds for the host and for `wasm32-unknown-unknown`. A runtime (`web.worker.replica` on Cloudflare; later a local `clm serve`) gives it a `Source` and a `Store` and calls it.

- `Replica::load(store)` and `save(store)`: the state, to and from a `Store` (`get(key) -> Option<bytes>`, `put(key, bytes)`, `delete_all()`), in chunks of at most 1 MiB each, so a store with a per-value limit holds it.
- `apply(change)`: takes a change a source has already fetched and marks the views stale. The changes are:
  - `Tip { sha, files, removed }`: the default branch at a new commit, with the text of each spec or non-Markdown file that changed, and the paths removed. Each file comes with its blob SHA, size, mode and whether it is binary.
  - `Tree { sha, entries }`: the whole listing at a commit, to rebuild from, or to recover after a force-push.
  - `Ledger { branch, text }`: a ledger branch's `records.jsonl` at its tip, or its deletion.
  - `Ref { name, sha, time, change }`: a ref the views read, moved, created or deleted. `change` is a spec ref's change files or an `impl/*` head's code key, when the source fetched them.
  - `Pull { .. }`: a pull request opened, edited or closed, with what `live::pulls` reads today.
  - `Event { .. }`: a push or a branch creation, for the activity feed.
- `pending_fetch()`: what the replica still needs before its views are complete: tree paths whose text it doesn't hold, in the order to fetch them. The runtime fetches them in slices it can afford and applies each slice as a `Tip` change.
- `recompute(now) -> Recomputed`: computes every view from the state, at once, and returns the time they next go stale without a change (the next claim's expiry), or none.
- `view(name)`: the last computed view, with the commit it describes and the time it was computed, or `Preparing { done }` before the first recompute that has the spec files.
- `validate(write)`: checks a web write against the state, giving the refusal its route answers or the record's fields to append (`validate-writes`, `validate-answers`).

The views and their shapes are the ones `web.worker`'s routes answer: `snapshot`, `implementation` (all but the `mine` of each of its `proposals` and `answers`, which is per requester), `capabilities` and `architecture`. A view's shape changes only with that contract.

## ADDED Requirements
### Requirement: snapshot-backlog-text
Each item of the snapshot's `backlog` SHALL carry `body`, the text of the item's file at the tip after its frontmatter with leading and trailing blank lines removed; `priority`, `P1` to `P5`, `P5` when the item sets none; and `choices`, the item's choices (`model/backlog-choices`), empty when it has none.

#### Scenario: an item in full
- GIVEN a tip with `backlog/q.md` holding `kind: question`, `priority: P2`, `choices: [yes, no]` and the body `Should we?`, and `backlog/i.md`, an `idea` with no priority
- WHEN the views are recomputed
- THEN the snapshot's `q` carries `body` `Should we?`, `priority` `P2` and `choices` `yes` and `no`, and `i` carries `priority` `P5` and no choices

#### Serves
- capability/follow-from-browser

#### Needs
- model/backlog-choices

### Requirement: open-answers
The replica's implementation view SHALL carry `answers`: the open answers of its ledger fold (`ledger/answer-records`) whose item its tip's backlog holds, each with `id`, `writer`, `at`, `item` and `answer` and no `mine`, sorted by `at` then `id`.

#### Scenario: answered, then resolved
- GIVEN a fold holding open answers `a1` naming `q` and `a2` naming `gone`, and a tip whose backlog holds `q` and not `gone`
- WHEN the views are recomputed
- THEN `answers` lists `a1` only, with its fields and no `mine`

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- ledger/answer-records

### Requirement: validate-answers
`validate` SHALL refuse an answer naming an item its tip's backlog doesn't hold as `Unknown`, and as `Invalid` an answer to a `question` item that isn't one of its `choices` (so any answer to a question without choices), an answer to a `human-task` item other than `done`, and any answer to an item of another kind.

#### Scenario: what an item offers
- GIVEN a tip whose backlog holds question `q` with choices `yes` and `no`, question `r` with none, `human-task` `h` and `idea` `i`
- WHEN answers `yes` to `q`, `maybe` to `q`, `yes` to `r`, `done` to `h`, `yes` to `h`, `done` to `i` and `yes` to `nope` are validated
- THEN the first and fourth pass, `nope` is refused as `Unknown`, and the rest as `Invalid`

#### Serves
- capability/follow-from-browser
- architecture/per-writer-ledgers

#### Needs
- model/backlog-choices
