---
status: proposed
resolves: [a-replica-another-release-saved-waits-for-its-idle-alarm]
---

## Intent

A Worker's replica object that wakes after a deploy, holding a replica the older release saved, starts its re-index at once even when the store already holds a far-off idle alarm: `web.worker.replica/refetches-what-a-release-forgot` now has the first read or event bring such an alarm forward, where it said only to set one when none was set.
