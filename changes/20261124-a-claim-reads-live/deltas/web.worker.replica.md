---
component: web.worker.replica
---

## MODIFIED Requirements

### Requirement: times-as-instants <!-- base: 5944fea6eb966906 -->
The times the object gives the replica, a ref's tip commit time from a push, a resync or a first build, a push's head commit time and an event's time, SHALL be the instants GitHub's RFC 3339 times name, their offsets counted, so a claim lapses and the activity feed orders by the instant a commit was made, whatever offset its committer's clock had. A push whose head commit time is missing or can't be read SHALL be recorded at the time it is applied; an event of GitHub's events listing whose time can't be read SHALL be left out of the feed; and a listed ref whose commit time can't be read SHALL still be held, at the time it is listed.

#### Scenario: a commit made two hours east
- GIVEN a built replica, and a push creating `impl/a` whose head commit's `timestamp` is `2026-10-01T12:00:00.5+02:00`; and a second built replica whose resync's refs listing gives `impl/b`, which it doesn't hold, a `committedDate` of `2026-10-01T12:00:00+02:00`
- WHEN the push is applied to the first and the resync runs on the second, and the replica each object saved is loaded and recomputed at `2026-10-01T11:00:00Z`
- THEN the first's `activity` holds the claim of `a` at 10:00 UTC, and each `recompute` returns 14:00:01 UTC, the first second the claim reads lapsed (`replica/batched-recompute`), as the time the implementation view next changes

#### Scenario: a time GitHub gives that can't be read
- GIVEN a built replica read a minute ago, holding `impl/b`, and an object that holds nothing
- WHEN a push creating `impl/a` whose head commit's `timestamp` is `yesterday` is applied at 10:00; the object that holds nothing makes its first build over an events listing holding a push whose `created_at` is `soon`; and a resync's refs listing gives `impl/b` a `committedDate` of `soon`
- THEN the claim of `a` is at 10:00; the feed holds nothing for that push; and `impl/b` is still held

#### Needs
- ledger/record-times
- replica/batched-recompute
