---
title: "A flaky test in a bisect probe makes clm verify blame an innocent land"
kind: idea
components: [verify]
---

`clm verify`'s search for the land that broke a red run assumes, as `git bisect` does, that a land after a red one stays red (`verify-bisect`). A test that fails by chance on one probe makes that probe red, so the search names a land that broke nothing as the culprit, and once the human takes the failure in, `clm land` holds code lands on that land's components rather than the ones that broke.

`20261109-verify-skill` left flakiness to the session: `/clm:verify` re-runs the failing tests at the red commit and says whether they failed again, but it can't re-check a probe, and nothing does for `clm verify --due` runs. Confirming a red probe once before moving the search (one more run of the check on that land) would cost a run only on a red probe. Worth doing when a misattributed culprit shows up in practice.
