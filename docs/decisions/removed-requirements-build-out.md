---
title: "Removing a requirement is built like adding one: the spec PR removes the text, the node's next build removes the code"
date: 2026-10-23
status: accepted
---

`check/dangling-markers` fails any `req:<id>` marker whose requirement no longer exists, so that removing or renaming a requirement forces its tests to change. But the spec PR that removes a requirement can't touch tests without becoming `mixed` (`humans-review-intent`), so until now no requirement with a test could be removed at all.

A marker that cites a requirement a change outside the archive lists under `REMOVED Requirements` is now *obsolete* rather than dangling. `clm check` accepts it while the change is live, and `clm coverage` lists it as `obsolete` with its locations. The code it marks, test included, is removed when the node is next built (`no-dead-code`). Archiving the change turns the exemption off, so the change can't finish while the code is still there.

This is `expand-contract` applied to requirements: the spec says what goes, and the build makes it go. The exemption is derived from the change's deltas, so nothing is written in the code to claim it.

A capability removed by a hand edit of `CAPABILITIES.md` is built out the same way, once its change lists it under `removes_capabilities` (`removed-capabilities-build-out`).

Rejected:
- Letting the spec PR delete the tests. That makes spec PRs mixed.
- Keeping the requirement with a `deprecated` mark. `spec-is-current` says removed requirements are deleted.
- A `clm:remove-by` marker on each test. The spec author would have to edit the tests, which is the same mixed PR.
