---
title: Replica
status: active
owners: ["@clunny"]
depends_on: [model, check, config, orchestrator, view, ledger, effort]
paths: ["crates/clm-replica/**"]
successor: ""
serves: [capability/see-project-state, capability/follow-from-browser, capability/review-capability-cost]
---

## Contract
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

## Design
The state is plain data: maps from path to text and blob SHA, the marker index, ledger sources by branch, refs, pull requests and at most 100 events. It is serialised with `serde_json` and split into chunks by `save`. The spec files are those `web.worker.platform`'s `files::is_spec_path` and `is_size_path` select. The change archive is kept as `changes/archive/<id>/change.md` only, since nothing but `resolved` reads it.

**Recompute.** It runs the code the Worker ran on request before this component:
- the snapshot: `clm_model::load_files`, then `clm_check::report` with the index's markers, then `clm_view`'s `spec` model with sizes from `clm_check::size_of` over the listing;
- the implementation's shared part: `flight_view`, `blockers_view`, `taken_from_refs`, the activity feed, the pages with their `checked` and the open proposals;
- the capabilities report: `clm_effort::report` with the flags' text, no history;
- the architecture report: `clm_effort::architecture` with the current requirements from the index once it is complete.

The code moves here from `clm-web` (`no-dead-code`), keeping its tests, which this component's build re-points at the new requirement IDs. Computation is whole-state: nothing is recomputed per component until a repository shows it needs that (`yagni`).

**Marker index.** A `Tip` change rescans only the files it carries, with `clm_check::markers_in`. A file is binary when the source says so, or when its text has a NUL byte in its first 8000 bytes, as `clm size` decides. While `pending_fetch` holds paths, the snapshot's coverage is `Indexing { done }` and its `sizes` is `"indexing"`.

**Writes.** `validate` answers what the Worker's routes checked before: a decision's entry, its version against the entry's version now, and whether the tip is signed; a proposal's slug against the backlog, `resolved` and the open proposals. Since the replica holds the ledgers and the tip, the check needs no request. The runtime first brings the replica up to the tip, so "now" means the tip, as `decide` requires.

## Requirements

### Requirement: host-neutral
`clm-replica` SHALL build for `wasm32-unknown-unknown` and SHALL depend on no crate that reaches a network, a filesystem or a host: not `worker`, `clm-web-platform`, `clm-host` or `clm-git`.

#### Scenario: dependencies
- GIVEN `crates/clm-replica/Cargo.toml`
- WHEN a test reads its dependencies, and `cargo build -p clm-replica --target wasm32-unknown-unknown` runs
- THEN none of those crates is named, and the build succeeds

#### Serves
- architecture/host-adapters
- architecture/web-optional

### Requirement: apply-what-changed
After any sequence of changes, a replica's state and views SHALL equal those of a replica built from scratch, with one `Tree` change and the files' text, at the same tip, ledgers, refs and pull requests.

#### Scenario: incremental push
- GIVEN a replica at `111` and a push to `222` that modifies `t.rs` and `specs/a/SPEC.md`, deletes `u.rs` and leaves `v.rs` alone
- WHEN the push is applied as a `Tip` change carrying only `t.rs` and `specs/a/SPEC.md`, and the views are recomputed
- THEN the views equal those of a replica built from scratch at `222`

#### Scenario: force-push
- GIVEN a replica at `111` and a default branch force-pushed to `333`, which isn't a descendant of `111`
- WHEN a `Tree` change at `333` and the files whose blob SHAs the replica doesn't hold are applied
- THEN the views equal those of a replica built from scratch at `333`

#### Serves
- architecture/git-is-truth

### Requirement: batched-recompute
`apply` SHALL never recompute, and `view` SHALL answer the last computed views however many changes have been applied since, until `recompute` runs. `recompute` SHALL return the time the implementation view next changes without a change, the earliest time a live claim lapses, or none when no claim is live.

#### Scenario: three pushes, one recompute
- GIVEN a replica whose views describe `111`
- WHEN pushes to `222`, `333` and `444` are applied, then `view("snapshot")` is read, then `recompute` runs and `view("snapshot")` is read again
- THEN the first read describes `111` and the second `444`

#### Scenario: a claim lapses
- GIVEN a replica holding `impl/a` pushed at 10:00 with the 4-hour expiry, and no other claim
- WHEN `recompute` runs at 11:00
- THEN it returns 14:00

### Requirement: preparing
Before the first `Tree` change and the text of the spec files it lists are applied, `view` SHALL answer `Preparing` with the share of the files fetched. Once they are, and while the marker index still has files to fetch, the snapshot SHALL report coverage as indexing with the share done and give no proven counts, and the architecture view SHALL say it is indexing.

#### Scenario: first build
- GIVEN a new replica, and a `Tree` change listing 10 spec files and 990 non-Markdown files
- WHEN the 10 spec files are applied and `recompute` runs
- THEN `view("snapshot")` is a snapshot whose coverage is indexing, and `pending_fetch` lists the 990 files

### Requirement: stores-in-chunks
`save` SHALL write no value over 1 MiB, and `load` from what `save` wrote SHALL give the same state.

#### Scenario: a large repository
- GIVEN a replica whose serialised state is 5 MiB
- WHEN it is saved to a fake store, then loaded from it
- THEN every value the store holds is at most 1 MiB, and the loaded replica's views equal the original's

### Requirement: snapshot-matches-clm
Once its marker index is complete, the replica's snapshot of a commit SHALL report the same components, component statuses, requirement coverage, changes and backlog items as `clm status`, `clm coverage` and `clm backlog` run on a checkout of that commit.

#### Scenario: fixture repo
- GIVEN a fixture repo with an unbuilt and an active component, a test citing one requirement at its current version and another at an older one, a Markdown file citing a third, an applied change, and a backlog item
- WHEN a replica is built from it, and the `clm` commands are run on a checkout of it
- THEN both report the same components, statuses, coverage, changes and backlog

#### Serves
- capability/follow-from-browser
- capability/see-project-state
- architecture/no-silent-drift

### Requirement: snapshot-size
Once its marker index is complete, the replica's snapshot of a commit SHALL say `sizes` `"checked"`, give every component of its `spec` model `oversized` `true` or `false`, and mark a component `true` exactly when `clm size` does on a checkout of that commit with no `clm.local.toml`, for a repository whose tracked files are regular files stored as checked out (no eol conversion, LFS or local attribute files) and whose Rust files hold no inline test part (`#[cfg(test)]`) citing a component other than the one whose `paths` hold the file. While the index is incomplete, or `clm.toml` can't be read, it SHALL say in `sizes` why sizes weren't checked and leave `oversized` absent on every component.

#### Serves
- capability/know-health
- architecture/slim-components

#### Scenario: same as clm size
- GIVEN a fixture repo whose `clm.toml` sets `[component_size]` with `code_tokens = 100`, component `a` with 600 bytes of code under its `paths`, component `b` with 600 bytes of code and `size_budget: { code_tokens: 200 }`, component `c` with 200 bytes of code, a binary file and a `linguist-generated` lockfile under `c`'s `paths`, each over 400 bytes
- WHEN a replica is built from it, and `clm size` runs on a checkout of it
- THEN the snapshot marks `a` and nothing else `oversized`, as `clm size` does, and gives `b` and `c` `oversized` `false`

#### Scenario: still indexing
- GIVEN a replica whose marker index still has files to fetch
- WHEN its snapshot is read
- THEN its `sizes` says `indexing` and no component in its `spec` model carries `oversized`

#### Scenario: symlink
- GIVEN the `same as clm size` fixture with a symlink under `a`'s `paths` whose target is a 10,000-byte file outside every component's `paths`
- WHEN the replica is built
- THEN `a`'s code count is the same as without the symlink

### Requirement: implementation-ledger
The replica's implementation view SHALL list in `activity`, newest first and at most 100 entries, the pushes and claims from its events, except pushes to `clm/ledger/*`, and an entry per `landed` record (`kind` `land`) and per `approved` record (`kind` `approval`) in the ledger fold, each naming the record's branch, the person part of its writer and its time, and SHALL list no merged pull requests or `spec-approved` labels. It SHALL carry `pages`, mapping each `impl/*` branch with an open pull request to its `number` and `url` and to `checked`, whether a `checked` record names the code key of the branch's head, left out while that key isn't known. It SHALL NOT carry `pulls` or `soft_lands`.

#### Scenario: land and approval from the ledger
- GIVEN a fold holding an `approved` record by `alice/web` for `spec/x` at 10:00 and a `landed` record by `bob/1a2b3c4d` for `spec/x` at 10:10, and events holding a push to `impl/a` at 10:05, a push to `clm/ledger/bob/1a2b3c4d` at 10:10 and a merged pull request at 10:10
- WHEN the views are recomputed
- THEN `activity` is the land of `spec/x` by `bob`, the push to `impl/a`, then the approval of `spec/x` by `alice`, and nothing else

#### Scenario: checked page
- GIVEN an open pull request #7 for `impl/a` whose head's code key a `checked` record names, and #8 for `impl/b` whose head's code key none does
- WHEN the views are recomputed
- THEN `pages` maps `impl/a` to number 7 with `checked` true and `impl/b` to number 8 with `checked` false

#### Scenario: no page
- GIVEN a claimed `impl/c` with no open pull request
- WHEN the views are recomputed
- THEN `pages` has no entry for `impl/c`, and the view has no `pulls` or `soft_lands` field

#### Serves
- capability/follow-from-browser
- capability/see-project-state

### Requirement: taken-items
The replica's implementation view SHALL carry `taken`, what `clm_orchestrator::taken_from_refs` returns for the snapshot's graph, its `propose/*`, `spec/*` and `autosync/spec/*` refs with their tips' commit times, each spec ref's `changes/<id>/change.md` and the names under `changes/<id>/deltas/` at its tip, or, when the tip has no `changes/<id>/change.md`, its `changes/archive/<id>/change.md` and the names under `changes/archive/<id>/deltas/` (`orchestrator/archived-spec-work`), the time of the recompute and a 4-hour expiry.

#### Scenario: held items
- GIVEN a replica whose tip holds change `x` resolving item `a`, whose `spec/y` names item `b` in its `change.md` with a delta for `web`, whose `propose/c` was pushed an hour ago and whose `propose/d` was pushed five hours ago
- WHEN the views are recomputed
- THEN `taken` lists `x` on main with item `a`, `y` proposed with item `b` and component `web`, and the claim on `c`, and nothing for `d`

#### Scenario: rename branch
- GIVEN a replica whose `spec/r` holds `changes/archive/r/change.md` naming item `e` in `resolves`, a delta file `changes/archive/r/deltas/web.md`, and no `changes/r/change.md`, and whose tip has no change `r`
- WHEN the views are recomputed
- THEN `taken` lists `r` proposed with item `e` and component `web`

#### Serves
- capability/see-project-state
- capability/work-concurrently

#### Needs
- orchestrator/taken-from-refs

### Requirement: implementation-models
The replica's implementation view's `flight` and `blockers` SHALL be the `view` component's `flight_view` and `blockers_view` models for its tip and its `impl/*` refs, and SHALL hold no `waiting` state or `waiting_on` field (`needs-alone-order`).

#### Serves
- capability/see-project-state
- capability/follow-from-browser
- architecture/reuse-first

#### Needs
- orchestrator/queue-from-refs
- view/blockers-view

#### Scenario: blocked by a need on a claimed component
- GIVEN a replica whose component `b` has one pending requirement, which needs `a/p`, uncited on component `a`, and `impl/a` claimed an hour ago
- WHEN the views are recomputed
- THEN `flight` lists `a` as `claimed` and `b` as `blocked` by `a/p`, and `blockers`' only root is `a`, with weight 1, holding up `b`

### Requirement: capabilities-report
The replica's capabilities view SHALL be `{"report", "signed"}`: `report` `clm_effort::report` over the graph of its tip and the fold of every ledger branch it holds, with no history, each workaround flag's text being its backlog file's body; `signed` true exactly when its tip holds `docs/allowed_signers`.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- effort/effort-builds-for-wasm

#### Scenario: the report clm gives
- GIVEN a replica whose `CAPABILITIES.md` holds `x`, a component serving it with a workaround flag on one of its rules, and a ledger with an `effort` record on `impl/<that component>`
- WHEN the views are recomputed
- THEN `report` equals `clm_effort::report` over the same files, the same ledgers and the flag's text, with no history, and `signed` is false

### Requirement: architecture-report
The replica's architecture view SHALL be `{"architecture", "signed", "indexing"}`: `architecture` `clm_effort::architecture` over the capabilities view's inputs, with `current` the requirements current by the marker index when it is complete, and `None` while it isn't, when `indexing` SHALL be true; `signed` SHALL be the capabilities view's.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- effort/architecture-report

#### Scenario: the report clm gives
- GIVEN a replica whose `ARCHITECTURE.md` holds `e`, enforced by a rule with a workaround flag, a complete marker index, and a ledger with an `effort` record on `impl/<that rule's component>`
- WHEN the views are recomputed
- THEN `architecture` equals `clm_effort::architecture` over the same files, ledgers, flag text and current requirements, `indexing` is false and `signed` is false

#### Scenario: indexing
- GIVEN the same replica with its marker index incomplete
- WHEN the views are recomputed
- THEN `indexing` is true and every enforcing rule's `current` is null

### Requirement: validate-writes
`validate` SHALL refuse a decision on an entry its tip's `CAPABILITIES.md` or `ARCHITECTURE.md` doesn't hold, or on a tip whose specs don't load, as `Unknown`; on a tip that holds `docs/allowed_signers`, as `Signed`; and on an entry whose version isn't the one named, as `Stale`, in that order; and otherwise SHALL give the flags `clm_effort::unsettled` names for the entry over its ledgers. It SHALL refuse a proposal whose slug its backlog holds, its `resolved` holds, or an open proposal of its fold has, as `Taken`, and a proposal naming a component its graph doesn't hold, as `Invalid`.

#### Scenario: a reworded capability
- GIVEN a replica whose `CAPABILITIES.md` holds `x` at version `v2`
- WHEN a decision on `x` naming `v1` is validated
- THEN it is refused as `Stale`

#### Scenario: signed before stale
- GIVEN a replica whose tip holds `docs/allowed_signers` and `x` at `v2`
- WHEN a decision on `x` naming `v1` is validated
- THEN it is refused as `Signed`

#### Scenario: a taken slug
- GIVEN a replica whose backlog holds `export-csv`
- WHEN a proposal titled "Export CSV" is validated
- THEN it is refused as `Taken`

#### Serves
- architecture/git-is-truth
- architecture/humans-review-intent

### Requirement: open-proposals
The replica's implementation view SHALL carry `proposals`: the open `proposed` records of its ledger fold, each with `id`, `writer`, `at`, `slug`, `title`, `body` and `/nodes` and no `mine`, sorted by `at` then `id`, leaving out each whose slug a backlog item of its tip has or a change of its tip, archived or not, names in `resolves`, which are what its snapshot's `backlog` and `resolved` hold.

#### Scenario: filed, resolved and withdrawn
- GIVEN a fold holding `proposed` records `p1` with slug `s1` by `alice/web`, `p2` with slug `s2`, `p3` with slug `s3` and `p4` with slug `s4`, a `withdrawn` record naming `p4`, and a tip with `backlog/s2.md` and an archived change whose `resolves` names `s3`
- WHEN the views are recomputed
- THEN `proposals` lists `p1` only, with its `id`, `writer`, `at`, `slug`, `title`, `body` and `/nodes` and no `mine`, and the snapshot's `resolved` holds `s3`

#### Scenario: order
- GIVEN open proposals `p2` and `p1` at 09:00 and `p3` at 08:00
- WHEN the views are recomputed
- THEN `proposals` lists `p3`, `p1`, then `p2`

#### Scenario: none
- GIVEN a replica holding no ledger
- WHEN the views are recomputed
- THEN `proposals` is empty

#### Serves
- capability/propose-intent
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/proposal-records

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
- architecture/git-is-truth

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
- architecture/git-is-truth

#### Needs
- model/backlog-choices
