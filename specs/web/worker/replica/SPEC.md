---
title: Replica object
status: active
owners: ["@clunny"]
depends_on: [replica, ledger, web.worker.platform, web.live]
paths: ["crates/clm-web-replica/**"]
successor: ""
serves: [capability/follow-from-browser, capability/see-project-state]
---

## Contract
Crate `clm-web-replica`: the Durable Object class `Replica`, one per repo, named `<o>/<r>` (`id_from_name`), which runs `replica`'s core on Cloudflare with a GitHub source and the object's own storage (`web-repo-replica`). `clm-web` re-exports the class and reaches each object through a `Replicas` platform trait, which tests fake. Its routes, all internal (the Worker is the only caller, through the `REPLICA` binding):

- `POST /event`: a verified webhook's event name, delivery and body, plus the installation's id. Answers 202 once the event is applied.
- `GET /view/<name>?fresh=<0|1>`: the view `replica`'s `view` gives, as JSON, with `sha` and `computed_at`, or 503 `{"preparing": <share>}`. With `fresh=1` it first brings itself up to the tip and recomputes if anything was stale.
- `POST /write`: a web write that needs the replica to validate (decide, propose, withdraw, answer), the requester's login and token, in the body `web.worker/writes-to-object` gives. It brings itself up to the tip, validates, appends the record through `web.worker.platform`'s `ledger::append` with the requester's token, applies it, and answers with the status and body the Worker's route answers, or 503 `{"preparing": <share>}` while it has no views. A decide, a proposal or an answer recomputes before the answer.

A read also tells the object the repo has open pages; the object keeps that for 5 minutes.

## Design
**Source.** The GitHub source, built on `web.worker.platform`'s installation tokens, `gh` calls and batched GraphQL reads (`files::texts`, `ledger::read`, `live::pulls`, the refs queries `live` makes today, `files::listing` for code keys), which move or are called from here (`no-dead-code`). Each alarm or request keeps within 50 subrequests, the Free plan's limit per invocation: a first build or a recovery fetches in slices and re-arms the alarm for the next.

**Events.**
- A `push` to the default branch: `compare` the replica's tip with the push's `after`. A fast-forward that lists at most 300 files is fetched by path; anything else lists the tree and refetches every blob whose SHA changed.
- A `push` to `clm/ledger/*`: that branch's `records.jsonl` at its new tip.
- A `push` that creates, moves or deletes an `impl/*`, `spec/*`, `autosync/spec/*` or `propose/*` ref: the ref, its tip's commit time, and the spec ref's change files or the `impl/*` head's code key.
- A `pull_request`: the pull request's fields `live::pulls` reads.
- `installation_repositories` with `removed`, or `installation` with `deleted`: delete the storage of each named repo's object.

Each event is applied, recorded for the activity feed when it is a push or a branch creation, and saved, and the alarm is set for one minute later unless one is set.

**Alarm.** It recomputes, saves, and then:
- posts `{"delivery": <the latest delivery applied>}` to the repo's `Live` object's `/notify` (`web.live`), so open pages refresh;
- sends the Web Push notifications the recompute's changes call for (`push-waiting`, `push-review`), posting them to the `Live` object's `/send` as the Worker did;
- re-arms for the time `recompute` returned while the repo has open pages, and, while it has, about every 10 minutes for a resync that lists the refs and open pull requests and applies any difference.

**Storage.** `replica`'s chunks are values of the object's SQLite storage, each at most 1 MiB, under Cloudflare's 2 MB per value. The object loads them once when it wakes and keeps the replica in memory while it is awake, so reads and recomputes cost no rows read, and saves only the chunks a change touched.

**Platform** (`object-class`). The class implements, over `worker`'s `Env`, `Fetch` and clock, only the platform traits the object uses (`GitHub`, `Clock`, `Secrets`, `Live`, `Log`), as `clm-web-live`'s `durable.rs` implements its own `Host`, and calls `web.worker.platform`'s uncached forms (`object-reads`). What the Worker keeps in the Cache API the object keeps in memory while it is awake: an installation token until five minutes before it expires, and each login's permission for five minutes, as `access::permission` does. It never names `Kv`, `Cache` or the KV and Cache API bindings, which `web.worker/cache-is-disposable`'s isolation test reads. Ledger records are built with `clm-ledger`'s types (`depends_on` lists `ledger`).

**Budget.** An object's request or alarm has 30 s of CPU. The account's daily allowance on the Free plan (100,000 requests, 5 million rows read, 100,000 rows written, 13,000 GB-s, 5 GB stored) is what binds; `web-on-free-plan` says when to move to Paid.

## Requirements

### Requirement: applies-events
On a verified `push` to the default branch, the object SHALL fetch only the files the compare between its tip and the push's `after` lists as added or modified, and only when the compare is a fast-forward of at most 300 files, and otherwise SHALL list the tree and fetch the blobs whose SHAs it doesn't hold. On a push to a ledger branch it SHALL fetch only that branch's records, and on a push that moves a ref the views read, only that ref's change files or code key.

#### Needs
- replica/apply-what-changed
- replica/stores-in-chunks

#### Scenario: a push to main
- GIVEN a replica at `111` and a push to `222` whose compare lists `t.rs` modified and `u.rs` removed
- WHEN the event is applied
- THEN the only file fetched is `t.rs` at `222`, and no tree was listed

#### Scenario: a ledger append
- GIVEN a replica holding `clm/ledger/alice/web`
- WHEN a push moving that branch is applied
- THEN the only fetch is that branch's `records.jsonl` at its new tip

#### Scenario: truncated compare
- GIVEN a replica at `111` and a push to `222` whose compare reports more than 300 files
- WHEN the event is applied
- THEN the tree at `222` is listed, and only blobs whose SHAs the replica doesn't hold are fetched

### Requirement: alarm-batches
Applying an event SHALL set the alarm one minute later unless one is set, and SHALL NOT recompute. The alarm SHALL recompute once, save, then post `{"delivery"}` naming the latest delivery applied to the repo's `Live` object's `/notify`.

#### Needs
- replica/batched-recompute

#### Scenario: a burst of pushes
- GIVEN a replica whose views describe `111`
- WHEN pushes to `222`, `333` and `444` are applied within 30 seconds, and the alarm then fires
- THEN the views were recomputed once, describe `444`, and the `Live` object was posted one notice, naming the third push's delivery

### Requirement: first-build
A read of a repo whose object holds nothing SHALL answer 503 `{"preparing": 0}` and start the build, and no request or alarm of the build SHALL make more than 50 subrequests. The views SHALL be computed, with coverage indexing, as soon as the spec files are in, before the non-Markdown files are scanned.

#### Needs
- replica/preparing
- replica/batched-recompute

#### Scenario: a repo of 1,000 files
- GIVEN a repo with 10 spec files and 990 non-Markdown files, and an object that holds nothing
- WHEN the snapshot is read, and the object's alarms run until it re-arms no more
- THEN the first read answered 503 `{"preparing": 0}`, no invocation made more than 50 subrequests, a read after the first alarm answered a snapshot with coverage indexing, and a read after the last a snapshot with coverage complete

### Requirement: resync
While a repo has open pages, the object SHALL list its refs and open pull requests about every 10 minutes and apply any difference from what it holds, and SHALL list nothing for a repo nobody has read for 5 minutes.

#### Needs
- replica/apply-what-changed

#### Scenario: a lost webhook
- GIVEN a repo read a minute ago, and an `impl/x` created on GitHub whose webhook never arrived
- WHEN the resync runs
- THEN the next recompute's implementation view lists the claim on `x`

#### Scenario: nobody looking
- GIVEN a repo last read an hour ago
- WHEN 20 minutes pass with no event
- THEN the object made no request to GitHub

### Requirement: writes-through-object
A write SHALL be validated against the replica after the object has brought itself up to the tip, by reading the default branch's ref and applying any push it hasn't seen, and SHALL be appended with the requester's token and applied to the replica before the answer. A decide, a proposal or an answer SHALL recompute before the answer, so a read right after it shows the record. An answer SHALL be validated by `replica/validate-answers`, `Unknown` answering 404 `{"error": "unknown"}` and `Invalid` 422 `{"error": "answer"}`, and appended as an `answered` record with `item` and `answer`, its `id` in the 200 answer. While the replica has no views to answer (`Preparing`), a write SHALL answer 503 `{"preparing": <share>}` as a read does, before it reads the ref, start the build when none is under way, and append nothing. When the object can't read the ref, or its tip isn't that ref's commit after applying the push, a write SHALL answer 502 and append nothing, so nothing is judged against a tip it knows to be behind; a tip that moved with files still to fetch has moved.

#### Scenario: an answer read back
- GIVEN a replica whose backlog holds question `q` with choices `yes` and `no`
- WHEN an answer `yes` to `q` is written through the object, and the implementation view is read right after
- THEN the view's `answers` holds it

#### Needs
- replica/validate-answers
- replica/open-answers
- replica/validate-writes
- replica/apply-what-changed

#### Scenario: a decision shows at once
- GIVEN a replica whose capabilities view shows `x` undecided
- WHEN a decision on `x` is written, and the capabilities view is read
- THEN the view shows the decision

#### Scenario: the tip can't be read
- GIVEN a replica holding `x`, and a GitHub that fails the read of the default branch's ref
- WHEN a decision on `x` naming its version, a proposal and a withdrawal are written
- THEN each answers 502, and nothing is appended

#### Scenario: still preparing
- GIVEN an object that holds nothing
- WHEN a proposal is written
- THEN it answers 503 `{"preparing": 0}`, the alarm is set, and nothing is appended

#### Scenario: reworded since the page was drawn
- GIVEN a replica that hasn't applied a push rewording `x`, which GitHub's default branch already holds
- WHEN a decision on `x` naming its old version is written
- THEN the object applies the push, refuses the decision as stale, and appends nothing

### Requirement: push-waiting
After a recompute whose tip moved, when `waiting_notice` gives a payload for the previous and new backlog items, the object SHALL post that payload, one message per recipient and at most 10, to the repo's `Live` object's `/send`, the recipients being, for each branch `clm/ledger/<login>/web` whose `<login>` GitHub says has a permission other than `none` on the repo, that branch's web subscriptions: the live subscriptions of its records folded alone, whatever `writer` they claim, which no record of another branch ends (`web-subscriptions-by-branch`). A `subscribed` record outside such a branch SHALL receive nothing. It SHALL post no send when no VAPID key is set, there was no previous recompute, or `waiting_notice` gives none.

#### Scenario: a question lands
- GIVEN a replica at `111` with no `question` items, a VAPID key, live subscriptions in `clm/ledger/alice/web` and `clm/ledger/bob/web`, and GitHub answering `read` for `alice` and `none` for `bob`
- WHEN a push moving `main` to `222`, whose backlog adds the `question` item "Which region?", is applied and the alarm fires
- THEN one send was posted to the object named `o/a`, holding one message, for `alice`'s subscription, whose payload's body is "Which region?"

#### Scenario: nothing new waits
- GIVEN the same, with `222` adding only an `idea` item
- WHEN the push is applied and the alarm fires
- THEN no send was posted

#### Scenario: a claimed writer in another branch
- GIVEN the same as *a question lands*, with `alice`'s subscription `s1` in her branch, a `subscribed` record with writer `alice/web` and another endpoint in `clm/ledger/mallory/web`, an `unsubscribed` record naming `s1` in that branch too, and GitHub answering `none` for `mallory`
- WHEN the push is applied and the alarm fires
- THEN one send was posted, holding one message, for `s1`'s endpoint only

#### Needs
- ledger/subscription-records
- web.live/push-send
- web.live/notices

### Requirement: push-review
On a `pull_request` event for which `review_notice` gives a payload, the object SHALL post it to the repo's `Live` object's `/send` for the recipients and within the limit `push-waiting` gives, and SHALL post no send when it gives none.

#### Scenario: opened for review
- GIVEN a VAPID key, a live subscription by `alice/web`, and GitHub answering `write` for `alice` on `o/a`
- WHEN a `pull_request` event with action `opened` for head `spec/x` of `o/a`, titled "Add x", is applied
- THEN one send was posted holding one message for `alice`'s subscription, whose payload's body is "spec/x: Add x"

#### Scenario: not for review
- GIVEN the same
- WHEN the event's action is `closed`
- THEN no send was posted

#### Needs
- ledger/subscription-records
- web.live/push-send
- web.live/notices

### Requirement: uninstall-deletes
An `installation_repositories` event with action `removed`, or an `installation` event with action `deleted`, SHALL delete the storage of each named repo's object.

#### Scenario: a repo removed
- GIVEN objects holding `o/a` and `o/b`
- WHEN an `installation_repositories` event removes `o/a`
- THEN `o/a`'s object holds nothing and `o/b`'s is unchanged

#### Serves
- architecture/git-is-truth

### Requirement: object-class
`clm-web-replica` SHALL define, for `wasm32`, the Durable Object class `Replica` (`#[durable_object]`), which answers `/event`, `/view/<name>` and `/write` and runs the alarm through `Object` over the object's SQLite storage and alarm and a platform of its own that implements only `GitHub`, `Clock`, `Secrets`, `Live` and `Log`, and no source file of the crate SHALL name `Kv` or `Cache`.

#### Scenario: the class builds
- GIVEN the workspace
- WHEN `cargo build -p clm-web-replica --target wasm32-unknown-unknown` runs
- THEN it succeeds, and the crate's sources hold `#[durable_object]` on a type named `Replica`

#### Scenario: no cache
- GIVEN `crates/clm-web-replica/src`
- WHEN `web.worker`'s isolation test reads it
- THEN no file names `Kv` or `Cache`

#### Needs
- web.worker.platform/object-reads

#### Serves
- architecture/git-is-truth
