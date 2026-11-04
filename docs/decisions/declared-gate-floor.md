---
title: "A merged change's declared gate is a floor"
date: 2026-10-27
status: superseded
superseded_by: yes-no-review
---

`change-tiers` rates an added or modified `change.md` by the `gate` it declares. Read at `HEAD` alone, that let a later PR rewrite a change merged with `gate: HARD`, intent, `from` and `resolves` included, while declaring `gate: AUTO`: the diff was then `AUTO`, `declared-gate` was satisfied, and approved intent changed with no human review (`humans-review-intent`).

So a modified `change.md` that exists outside the archive at the merge base is rated the higher of the `gate` it declares at the merge base and at `HEAD`, and `HARD` when its merge-base frontmatter doesn't parse. `declared-gate` still checks the `HEAD` value against the PR's tier, so lowering a merged change's `gate` is blocked even when a human approves the diff. The field records the review the change asked for when its intent was accepted; lowering it afterwards would only misstate that. A change that turns out over-declared keeps its gate, which costs nothing once it is merged, since its later edits are rare. Raising a gate stays free.

Alternatives considered:
- Making any lowering of a declared gate `HARD`, so a human could approve it. That lets a `SOFT` change be lowered to `AUTO` and later edited unreviewed by a second PR, and asks a human to approve a change to a field that has no use once lowered.
- Rating every edit to a merged `change.md` `HARD`. That makes a typo fix in an `AUTO` change cost a review, which `computed-gate-tiers` exists to avoid.
