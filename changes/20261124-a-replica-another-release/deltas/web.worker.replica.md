---
component: web.worker.replica
---

## MODIFIED Requirements

### Requirement: refetches-what-a-release-forgot <!-- base: 8566e427fc6c8be8 -->
When an object that holds a tip wakes with a replica whose `pending_fetch` lists files, as one a newer release loaded does (`replica/markers-of-this-release`), its first read or event SHALL set the alarm to now unless one is set to fire within one minute, an alarm set for later, such as the idle alarm of a repo whose pages were open, being brought forward, and its alarms SHALL fetch those files in slices, no request or alarm making more than 50 subrequests, and re-arm while any is left, whether or not the repo has open pages, recomputing and posting `{"delivery"}` to `/notify` as `alarm-batches` states. Until the recompute, a read SHALL answer the views the replica holds.

#### Needs
- replica/markers-of-this-release

#### Scenario: a newer release
- GIVEN an object at the tip GitHub gives, with no alarm set and no page open, whose stored replica a release `0.0.0` saved with `src/a.rs` citing a requirement, in a repo whose non-Markdown files more than one alarm's budget holds
- WHEN the snapshot is read, then the object's alarms run until it re-arms no more, and the snapshot is read again
- THEN the first read answers 200 with the stored snapshot and sets the alarm to now; no invocation made more than 50 subrequests; the second read's snapshot has coverage complete, counting `src/a.rs`'s marker, and a notice was posted

#### Scenario: an idle alarm already set
- GIVEN an object at the tip GitHub gives, whose stored replica a release `0.0.0` saved with `src/a.rs` citing a requirement, and whose store holds an alarm set ten minutes ahead
- WHEN the snapshot is read, and then the alarm fires at the time it is set to
- THEN the read answers 200 with the stored snapshot and sets the alarm to now, and the alarm fetches `src/a.rs`
