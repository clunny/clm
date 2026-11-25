---
component: web.worker.replica
---

## MODIFIED Requirements

### Requirement: alarm-batches <!-- base: 9c14a20a4c828302 -->
Applying an event SHALL set the alarm to one minute later unless one is set to fire within that minute, and SHALL NOT recompute. The alarm SHALL recompute once, save, then post `{"delivery"}` naming the latest delivery applied to the repo's `Live` object's `/notify`. While the repo has open pages, the alarm SHALL also fire at the time the last recompute said the implementation view next changes, recompute, and post `{"delivery"}` to `/notify`, naming 32 lowercase hex digits no event has when no event was applied since the last notice.

#### Scenario: a burst of pushes
- GIVEN a replica whose views describe `111`
- WHEN pushes to `222`, `333` and `444` are applied within 30 seconds, and the alarm then fires
- THEN the views were recomputed once, describe `444`, and the `Live` object was posted one notice, naming the third push's delivery

#### Scenario: an idle alarm pulled in
- GIVEN a replica read a minute ago, whose alarm is set ten minutes ahead for a resync
- WHEN a push is applied
- THEN the alarm is set one minute after it

#### Scenario: a claim lapses while a page is open
- GIVEN a replica holding a claim on `a` pushed at 10:00, read at 13:58, its last recompute naming the time `t` the implementation view next changes
- WHEN the alarms run until a second past `t`
- THEN an alarm fired at `t`, the implementation view no longer lists the claim, and each notice posted since 13:58 is `{"delivery"}` naming 32 hex digits no event has

### Requirement: times-as-instants <!-- base: 5944fea6eb966906 -->
The times the object gives the replica, a ref's tip commit time from a push, a resync or a first build, a push's head commit time and an event's time, SHALL be the instants GitHub's RFC 3339 times name, their offsets counted, so a claim lapses and the activity feed orders by the instant a commit was made, whatever offset its committer's clock had. A push whose head commit time is missing or can't be read SHALL be recorded at the time it is applied; an event of GitHub's events listing whose time can't be read SHALL be left out of the feed; and a listed ref whose commit time can't be read SHALL still be held, at the time it is listed.

#### Scenario: a commit made two hours east
- GIVEN a built replica, and a push creating `impl/a` whose head commit's `timestamp` is `2026-10-01T12:00:00.5+02:00`; and a second built replica whose resync's refs listing gives `impl/b`, which it doesn't hold, a `committedDate` of `2026-10-01T12:00:00+02:00`
- WHEN the push is applied to the first and the resync runs on the second, and the replica each object saved is loaded and recomputed at `2026-10-01T11:00:00Z`
- THEN the first's `activity` holds the claim of `a` at 10:00 UTC, and each `recompute` returns, as the time the implementation view next changes, the time a replica holding only a claim pushed at 10:00 UTC returns when recomputed at 11:00 UTC

#### Scenario: a time GitHub gives that can't be read
- GIVEN a built replica read a minute ago, holding `impl/b`, and an object that holds nothing
- WHEN a push creating `impl/a` whose head commit's `timestamp` is `yesterday` is applied at 10:00; the object that holds nothing makes its first build over an events listing holding a push whose `created_at` is `soon`; and a resync's refs listing gives `impl/b` a `committedDate` of `soon`
- THEN the claim of `a` is at 10:00; the feed holds nothing for that push; and `impl/b` is still held

#### Needs
- ledger/record-times
