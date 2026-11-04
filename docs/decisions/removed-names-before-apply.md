---
title: "`removed-requirement-names` also judges the graph a pending change would leave"
date: 2026-10-30
status: accepted
---

`removed-requirement-names` reported a name a change's removal left stale only once the change was applied, so a draft passed `clm check` and met the report in `clm sync`, paying a second delta edit, commit and re-apply (`backlog/check-removed-requirement-names-reports-only-after-apply-not-on`).

Now `clm check` also runs the rule's scan on the graph each `proposed` or `approved` change would leave, which `apply`'s `resulting-graph` already builds for `validate` and `resulting(graph, change)` now returns, with the IDs that change removes. A name the change's own deltas rewrite is gone from that graph, so it is never reported; a name in a node the change doesn't touch is reported at the line it has today.

Rejected: reporting, in today's `specs/`, the names of IDs a pending change will remove. The requirement still exists there, and the change's own deltas often rewrite the text that names it, so the report would be false until apply. Rejected too: moving the rule into the model's graph check so `apply/resulting-graph` runs it; the removed IDs come from `check.coverage`, which the model can't depend on.
