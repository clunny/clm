---
title: "A spec branch can't remove a capability entry a test cites"
kind: bug
components: [check]
origin: 20261113-human-layer-rewrite
---

Step: a spec branch removes a human-layer entry (a capability in CAPABILITIES.md) that a capability test cites.
What happened: clm check fails the branch with check/dangling-markers on the test's marker. The branch can't delete the test, since a test file is a code path and the gate blocks a mixed branch, so no single spec branch can remove a cited entry. A requirement removed by a delta reads as obsolete and is cleaned up by its component's build; an entry removed by a direct edit has no such state.
Expected: the markers of an entry the branch removes read as obsolete on the capability component, as a removed requirement's do, so the spec branch lands and the build removes the test.
Worked around by: landing a code branch first that deletes the citing tests, which leaves the capability unbuilt on main until the spec branch lands.
Rules involved: check/dangling-markers, gate/classify, human-layer-proof.
