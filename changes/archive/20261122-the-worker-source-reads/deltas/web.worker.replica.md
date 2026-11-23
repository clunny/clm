---
component: web.worker.replica
base: { design: "5ea1c5ea0539e399" }
---

## MODIFIED Design
**Source.** The GitHub source, built on `web.worker.platform`'s installation tokens, `gh` calls and batched GraphQL reads (`files::texts`, `ledger::read`, `live::pulls`, the refs queries `live` makes today, `files::listing` for code keys), which move or are called from here (`no-dead-code`). Each alarm or request keeps within 50 subrequests, the Free plan's limit per invocation: a first build or a recovery fetches in slices and re-arms the alarm for the next. It fetches by `replica`'s own rule, `clm_replica::keeps_text` and `clm_replica::wanted` (`replica/files-it-fetches`), and reads every time GitHub gives with `install::unix_time`, holding no copy of either: a copy that drifted from the replica's rule would leave a file `pending_fetch` lists unfetched, and the replica `Preparing` for good (`fetches-what-the-replica-wants`, `times-as-instants`).

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

## ADDED Requirements

### Requirement: times-as-instants
The times the object gives the replica, a ref's tip commit time from a push, a resync or a first build, a push's head commit time and an event's time, SHALL be the instants GitHub's RFC 3339 times name, their offsets counted, so a claim lapses and the activity feed orders by the instant a commit was made, whatever offset its committer's clock had.

#### Scenario: a commit made two hours east
- GIVEN a built replica, and a push creating `impl/a` whose head commit's `timestamp` is `2026-10-01T12:00:00.5+02:00`; and a second built replica whose resync's refs listing gives `impl/b`, which it doesn't hold, a `committedDate` of `2026-10-01T12:00:00+02:00`
- WHEN the push is applied to the first and the resync runs on the second, and the replica each object saved is loaded and recomputed at `2026-10-01T11:00:00Z`
- THEN the first's `activity` holds the claim of `a` at 10:00 UTC, and each `recompute` returns 14:00 UTC as the time the implementation view next changes

#### Needs
- web.worker.platform/unix-time

### Requirement: fetches-what-the-replica-wants
Of the files a compare or a tree listing names, the object SHALL fetch exactly those `clm_replica::wanted` accepts (`replica/files-it-fetches`), and in a first build or a recovery SHALL fetch those `clm_replica::keeps_text` accepts before any other, so every file `pending_fetch` lists is fetched while the budget lasts.

#### Scenario: a push listing every kind of file
- GIVEN a replica at `111`, and a push to `222` whose compare lists `web/.gitattributes`, `clm.toml`, `docs/decisions/d.md`, `src/b.rs`, `README.md` and `docs/guide.md` as modified
- WHEN the event is applied
- THEN the files fetched are `web/.gitattributes`, `clm.toml`, `docs/decisions/d.md` and `src/b.rs` at `222`, and neither `README.md` nor `docs/guide.md`
