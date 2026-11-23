---
title: Replica
status: active
owners: ["@clunny"]
depends_on: [model, check, check.coverage, config, orchestrator, view, ledger, effort]
paths: ["crates/clm-replica/**"]
successor: ""
serves: [capability/see-project-state, capability/follow-from-browser, capability/review-capability-cost]
---

## Contract
Crate `clm-replica`: a repository's derived state, kept up to date from the changes a source reports, and the views computed from it (`web-repo-replica`). It is host-neutral and runtime-neutral: it knows nothing of GitHub, Cloudflare, a browser or a filesystem, and builds for the host and for `wasm32-unknown-unknown`. A runtime (`web.worker.replica` on Cloudflare; later a local `clm serve`) gives it a `Source` and a `Store` and calls it.

- `Replica::default()`: a new replica, holding nothing, which a runtime makes when it starts again from git.
- `Replica::load(store)` and `save(store)`: the state, to and from a `Store` (`get(key) -> Option<bytes>`, `put(key, bytes)`, `delete_all()`), in chunks of at most 1 MiB each, so a store with a per-value limit holds it; `load` gives a new replica from a store that holds none that reads (`stores-in-chunks`).
- `apply(change)`: takes a change a source has already fetched and marks the views stale. The changes are:
  - `Tip { sha, files, removed }`: the default branch at a new commit, with the text of each file that changed and that `wanted` accepts, and the paths removed. Each file comes with its blob SHA, size, mode and whether it is binary.
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

## Design
The state is plain data: maps from path to text and blob SHA, the marker index, ledger sources by branch, refs, pull requests and at most 100 events. It is serialised with `serde_json` and split into chunks by `save`. The spec files are those `keeps_text` accepts: the files the model reads, `clm.toml` and every `.gitattributes`. Every file under `changes/` is kept, the archive's deltas with its `change.md`, since the coverage's moves and the capabilities report's change counts read archived deltas.

**Recompute.** It runs the code the Worker ran on request before this component:
- the snapshot: `clm_model::load_files`, then `clm_check::report` with the index's markers, then `clm_view`'s `spec` model with sizes from `clm_check::size_of` over the listing;
- the implementation's shared part: `flight_view`, `blockers_view`, `taken_from_refs`, the activity feed, the pages with their `checked` and the open proposals;
- the capabilities report: `clm_effort::report` with the flags' text, no history;
- the architecture report: `clm_effort::architecture` with the current requirements from the index once it is complete.

Computation is whole-state: nothing is recomputed per component until a repository shows it needs that (`yagni`).

**Marker index.** A `Tip` change rescans only the files it carries, with `clm_coverage::markers_in`. A file is binary when the source says so, or when its text has a NUL byte in its first 8000 bytes, as `clm size` decides.

**Writes.** `validate` answers what the Worker's routes checked before: a decision's entry, its version against the entry's version now, and whether the tip is signed; a proposal's slug against the backlog, `resolved` and the open proposals. Since the replica holds the ledgers and the tip, the check needs no request. The runtime first brings the replica up to the tip, so "now" means the tip, as `decide` requires.

## Requirements

### Requirement: host-neutral
`clm-replica` SHALL build for `wasm32-unknown-unknown`, and its build SHALL take in no crate that reaches a network, a filesystem or a host: neither its normal or build dependencies nor any crate they bring in SHALL be `worker`, `clm-web-platform`, `clm-host` or `clm-git`.

#### Scenario: dependencies
- GIVEN the workspace
- WHEN `cargo tree -p clm-replica -e normal,build` and `cargo build -p clm-replica --target wasm32-unknown-unknown` run
- THEN the tree names none of those crates, and the build succeeds

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
`apply` SHALL never recompute, and `view` SHALL answer the last computed views however many changes have been applied since, until `recompute` runs. `stale()` SHALL be true from any `apply` until a `recompute` that computes, and false after one. Before any `Tree` change, and while a regular file of the listing that `keeps_text` accepts has still to be fetched (given with its text or marked binary), `recompute` SHALL compute nothing, leaving every view and `stale()` as they were, and return none. `view` SHALL answer nothing for a name other than `snapshot`, `implementation`, `capabilities` and `architecture`. `recompute` SHALL return the time the implementation view next changes without a change: of the live `impl/<component>` and `propose/<item>` claims, the earliest time one lapses, 4 hours after its tip's time, or one second after `now` for a claim that lapses at `now` itself, which is still live; or none when no claim is live.

#### Scenario: three pushes, one recompute
- GIVEN a replica whose views describe `111`
- WHEN pushes to `222`, `333` and `444` are applied, then `view("snapshot")` is read, then `recompute` runs and `view("snapshot")` is read again
- THEN the first read describes `111` and the second `444`, and `stale()` is true before the recompute and false after it

#### Scenario: a claim lapses
- GIVEN a replica holding `impl/a` pushed at 10:00 with the 4-hour expiry, and no other claim
- WHEN `recompute` runs at 11:00, at 14:00 and at 15:00, and, on a replica whose only claim is `propose/x` pushed at 10:30, at 11:00
- THEN they return 14:00, 14:00:01 and none, and 14:30

#### Scenario: spec files still to fetch
- GIVEN a new replica given only a `Tree` change at `111` listing spec files, and a ready replica at `111` given a `Tree` change at `222` that lists a spec file whose text it doesn't hold
- WHEN `recompute` runs on each
- THEN it returns none; the first's `stale()` is true and its `view("snapshot")` is `Preparing`, and the second's `view("snapshot")` still describes `111`

#### Scenario: an unknown view
- GIVEN a ready replica
- WHEN `view("nothing")` is read
- THEN it answers nothing

### Requirement: preparing
Before the first `Tree` change is applied and every regular file of the listing that `keeps_text` accepts has been fetched (given with its text or marked binary), `view` SHALL answer `Preparing` with the share of those files it has fetched, 0 before any `Tree` change. Once they are, and while the marker index still has files to fetch, the snapshot SHALL report coverage as indexing with the share done and give no proven counts.

#### Scenario: first build
- GIVEN a new replica, and a `Tree` change listing 10 spec files and 990 non-Markdown files
- WHEN the 10 spec files are applied and `recompute` runs
- THEN `view("snapshot")` is a snapshot whose coverage is indexing, and `pending_fetch` lists the 990 files

### Requirement: stores-in-chunks
`save` SHALL write no value over 1 MiB and leave in the store only what it wrote, and `load` from what `save` wrote SHALL give the same state. `load` from a store that holds no replica, whose chunk count names a chunk it lacks, or whose chunks don't read as a saved replica, SHALL give a new replica, the same as `Replica::default()`.

#### Scenario: a large repository
- GIVEN a replica whose serialised state is 5 MiB
- WHEN it is saved to a fake store, then loaded from it, and then a replica of a few KiB is saved to the same store
- THEN every value the store holds is at most 1 MiB, the loaded replica's views equal the original's, and after the second save the store holds only that save's values

#### Scenario: an empty or unreadable store
- GIVEN an empty store, a store whose count is 2 holding only chunk 0, and a store whose count is 1 and whose chunk 0 holds `{`
- WHEN a replica is loaded from each
- THEN each gives `Replica::default()`

### Requirement: snapshot-matches-clm
Once its marker index is complete, the replica's snapshot of a commit SHALL report the same components, component statuses, requirement coverage, changes and backlog items as `clm status`, `clm coverage` and `clm backlog` run on a checkout of that commit, each component's `status` being the word `clm status` writes on that component's row; a `spec` model equal to what `clm view spec --format json` prints there, with the sizes `snapshot-size` gives; each component's text assets in `assets` with the hashes a checkout's give; `decisions`, each decision's `name`, `title`, `date` and `status` lowercased; and `violations`, the violations the model gives in loading the commit's files, each as `clm check` prints one.

#### Scenario: fixture repo
- GIVEN a fixture repo with an unbuilt and an active component, a test citing one requirement at its current version and another at an older one, a Markdown file citing a third, an applied change, a backlog item, a text asset `specs/b/assets/note.txt`, a binary file outside the spec directories, and `docs/decisions/d.md` with title `D`, date `2026-10-01` and status `accepted`
- WHEN a replica is built from it, and the `clm` commands are run on a checkout of it, and again with a backlog item whose frontmatter doesn't parse added
- THEN both report the same components, statuses (the unbuilt component's `status` the word on its `clm status` row), coverage, changes and backlog, the snapshot's `spec` is the model `clm view spec --format json` prints, `note.txt`'s hash is the checkout's, `decisions` is `d` with `D`, `2026-10-01` and `accepted`, and `violations` is empty, and then holds the line `clm check` prints for that item

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
The replica's implementation view SHALL list in `activity`, newest first and at most 100 entries, the pushes and claims from its events, and an entry per `landed` record (`kind` `land`) and per `approved` record (`kind` `approval`) in the ledger fold, each naming the record's branch, the person part of its writer and its time, and SHALL list no merged or closed pull request. A push to a `clm/ledger/*` branch SHALL add nothing. A claim SHALL be the creation of an `impl/<component>` branch, its `ref` the component, and the creation of any other branch, a nested one such as `impl/archive/<id>` included, SHALL add nothing. A record whose `at` doesn't read as an RFC 3339 time SHALL be left out, and one with an offset SHALL count at the instant it names. It SHALL carry `pages`, mapping each branch whose name begins `impl/` and that has an open pull request from the repository itself, not from a fork, to its `number` and `url` and to `checked`, whether a `checked` record names the code key of the branch's head, left out while that key isn't known or while the pull request's head isn't the commit whose key the source fetched; when two such pull requests share a branch, its page SHALL be the one with the lower number. It SHALL NOT carry `pulls` or `soft_lands`.

#### Scenario: land and approval from the ledger
- GIVEN a fold holding an `approved` record by `alice/web` for `spec/x` at 10:00 and a `landed` record by `bob/1a2b3c4d` for `spec/x` at 10:10, events holding a push to `impl/a` at 10:05 and a push to `clm/ledger/bob/1a2b3c4d` at 10:10, and a closed pull request #9 for `impl/a`
- WHEN the views are recomputed
- THEN `activity` is the land of `spec/x` by `bob`, the push to `impl/a`, then the approval of `spec/x` by `alice`, and nothing else

#### Scenario: checked page
- GIVEN an open pull request #7 for `impl/a` whose head's code key a `checked` record names, #8 for `impl/b` whose head's code key none does, #6 also open for `impl/b` at the same head, #9 for `impl/d` whose head's code key isn't known, a fork's #5 from a branch `impl/a`, and #10 for `spec/x`
- WHEN the views are recomputed, and again after #7's head moves to a commit whose key the source hasn't fetched
- THEN `pages` maps `impl/a` to number 7 with `checked` true and `impl/b` to number 6 with `checked` false and `impl/d` to number 9 with no `checked`, and nothing else, and then `impl/a` with no `checked`

#### Scenario: no page
- GIVEN a claimed `impl/c` with no open pull request
- WHEN the views are recomputed
- THEN `pages` has no entry for `impl/c`, and the view has no `pulls` or `soft_lands` field

#### Scenario: claims and odd records
- GIVEN events holding the creation of `impl/a` at 09:00 UTC on 2026-10-01, of `spec/y` at 09:05 and of `impl/archive/x` at 09:10, and a fold holding an `approved` record for `spec/z` whose `at` is `2026-10-01`, and one for `spec/w` whose `at` is `2026-10-01T12:00:00+02:00`
- WHEN the views are recomputed
- THEN `activity` is the approval of `spec/w` at 10:00 UTC and the claim with `ref` `a`, and nothing else

#### Serves
- capability/follow-from-browser
- capability/see-project-state

### Requirement: taken-items
The replica's implementation view SHALL carry `taken`, what holds the backlog's items, as `orchestrator/taken-from-refs` gives it for the snapshot's graph, its `propose/*`, `spec/*` and `autosync/spec/*` refs with their tips' commit times, each spec ref's `changes/<id>/change.md` and the names under `changes/<id>/deltas/` at its tip, or, when the tip has no `changes/<id>/change.md`, its `changes/archive/<id>/change.md` and the names under `changes/archive/<id>/deltas/` (`orchestrator/archived-spec-work`), the time of the recompute and a 4-hour expiry.

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
The replica's implementation view SHALL carry `sha`, its tip; `now`, the time of the recompute; `expiry_hours`, 4; and `coverage`, `{"state": "complete"}` once its marker index is complete and otherwise `{"state": "indexing", "done": <share>}`, the share of the files to scan that are scanned. Its `flight` and `blockers` SHALL be the `view` component's flight and blockers models (`view/flight-view`, `view/blockers-view`) for its tip, its `impl/<component>` refs, the time of the recompute and the 4-hour expiry, counting no marker while the index is incomplete, and SHALL hold no `waiting` state or `waiting_on` field (`needs-alone-order`).

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

#### Scenario: the view's frame
- GIVEN a replica at `111` whose `t.rs` cites `a/p` at its current version, with every file scanned, and the same replica with a second source file `u.rs` still to scan
- WHEN the views are recomputed at 12:00
- THEN the first's implementation view carries `sha` `111`, `now` 12:00, `expiry_hours` 4 and `coverage` `{"state": "complete"}`, and the second's carries `coverage` `{"state": "indexing", "done": 0.5}` and a `flight` that is the model for no marker

### Requirement: capabilities-report
The replica's capabilities view SHALL be `{"report", "signed"}`: `report` the capabilities report `effort` gives (`effort/capabilities-json`) for the graph of its tip and the fold of every ledger branch it holds, with no history, each workaround flag's text being its backlog file's body; `signed` true exactly when its tip holds `docs/allowed_signers`.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- effort/effort-builds-for-wasm

#### Scenario: the report clm gives
- GIVEN a replica whose `CAPABILITIES.md` holds `x`, a component serving it with a workaround flag on one of its rules, and a ledger with an `effort` record on `impl/<that component>`
- WHEN the views are recomputed
- THEN `report` equals the report `effort` gives over the same files, the same ledgers and the flag's text, with no history, and `signed` is false

### Requirement: architecture-report
The replica's architecture view SHALL be `{"architecture", "signed", "indexing"}`: `architecture` the architecture review `effort` gives (`effort/architecture-report`) for the capabilities view's inputs, with `current` the requirements current by the marker index when it is complete, and `None` while it isn't, when `indexing` SHALL be true; `signed` SHALL be the capabilities view's.

#### Serves
- capability/review-capability-cost
- capability/follow-from-browser

#### Needs
- effort/architecture-report

#### Scenario: the report clm gives
- GIVEN a replica whose `ARCHITECTURE.md` holds `e`, enforced by a rule with a workaround flag, a complete marker index, and a ledger with an `effort` record on `impl/<that rule's component>`
- WHEN the views are recomputed
- THEN `architecture` equals the review `effort` gives over the same files, ledgers, flag text and current requirements, `indexing` is false and `signed` is false

#### Scenario: indexing
- GIVEN the same replica with its marker index incomplete
- WHEN the views are recomputed
- THEN `indexing` is true and every enforcing rule's `current` is null

### Requirement: validate-writes
`validate` SHALL refuse every write as `Unknown` before it has fetched every regular file of its listing that `keeps_text` accepts, and while the model gives a violation in loading them. It SHALL refuse a decision on an entry its tip's `CAPABILITIES.md` or `ARCHITECTURE.md` doesn't hold as `Unknown`; on a tip that holds `docs/allowed_signers`, as `Signed`; and on an entry whose version isn't the one named, as `Stale`, in that order; and otherwise SHALL give the slugs of the workaround flags open and unsettled on the entry over its ledgers (`effort/unsettled-flags`). A proposal's slug SHALL be its title lowercased, split on anything but ASCII letters and digits, the words joined with `-` while the result stays within 64 bytes, and a first word longer than that cut to 64 bytes. It SHALL refuse as `Invalid` a proposal whose title gives no slug or that names a component its graph doesn't hold; otherwise as `Taken` one whose slug its backlog holds, its `resolved` holds, or an open proposal of its fold has; and otherwise SHALL give the slug.

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
- WHEN a proposal titled "Export CSV" is validated, and one with the same title naming component `nope`
- THEN the first is refused as `Taken`, and the second as `Invalid`

#### Scenario: nothing to judge against
- GIVEN a new replica, and a ready replica whose `specs/b/SPEC.md` doesn't load
- WHEN a decision on `capability/x` at its version, a proposal titled "Dark mode" and the answer `yes` to question `q` with choices `yes` and `no` are validated on each
- THEN all six are refused as `Unknown`

#### Scenario: slug
- GIVEN a replica whose backlog holds none of the slugs below
- WHEN proposals titled `  Dark mode: on/off!  `, `Ünïcode only`, 70 `a`s, 62 `a`s followed by ` bb`, and `!!!` are validated
- THEN they give `dark-mode-on-off`, `n-code-only`, 64 `a`s and 62 `a`s, and the last is refused as `Invalid`

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

### Requirement: files-it-fetches
`keeps_text(path)` SHALL be true exactly for `ARCHITECTURE.md`, `CAPABILITIES.md`, `clm.toml`, a `.gitattributes` at the top or in any directory, and every path under `specs/`, `changes/`, `backlog/` or `docs/decisions/`, the change archive included; `wanted(path)` SHALL be true for those and for every path that doesn't end `.md`. `pending_fetch` SHALL list each regular file (mode `100644` or `100755`) of the listing that `wanted` accepts that the replica hasn't been given at the listed blob SHA, with its text or marked binary, those `keeps_text` accepts first, each group in path order, and no symlink or submodule; a file a `Tip` change carried with no text and not marked binary SHALL stay listed.

#### Scenario: which files the replica reads
- GIVEN the paths `specs/a/SPEC.md`, `changes/archive/x/deltas/a.md`, `docs/decisions/d.md`, `backlog/i.md`, `CAPABILITIES.md`, `clm.toml`, `.gitattributes`, `web/.gitattributes`, `src/lib.rs`, `README.md` and `docs/guide.md`
- WHEN `keeps_text` and `wanted` are asked of each
- THEN `keeps_text` is true for the first eight and false for the rest, and `wanted` is true for the first nine and false for `README.md` and `docs/guide.md`

#### Scenario: what is still to fetch
- GIVEN a replica given a `Tree` change listing `specs/a/SPEC.md`, `src/b.rs`, `README.md`, a symlink `a/link` (mode `120000`) and a submodule `vendor/x` (mode `160000`)
- WHEN `pending_fetch` is read, then a `Tip` change carrying `src/b.rs` with no text and not marked binary is applied and `pending_fetch` is read again
- THEN the first read lists `specs/a/SPEC.md` then `src/b.rs`, and nothing else, and the second still lists `src/b.rs`

#### Serves
- capability/follow-from-browser
- architecture/reuse-first
