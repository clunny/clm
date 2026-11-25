---
component: web.worker.replica
---

## ADDED Requirements

### Requirement: one-notice-per-lapse
When a claim lapses while the repo has open pages and no event was applied, the object SHALL post one notice for the lapse, at the first second the claim reads lapsed (`replica/batched-recompute`), and no alarm SHALL fire for the lapse at an earlier second.

#### Needs
- replica/batched-recompute

#### Scenario: a claim lapses once
- GIVEN a replica holding a claim on `a` pushed at 10:00 with the 4-hour expiry, read at 13:58
- WHEN the alarms run until 14:00:01
- THEN an alarm fired at 14:00:01 and none at 14:00, the implementation view no longer lists the claim, and exactly one notice was posted after 13:58
