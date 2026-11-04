---
title: "No stated route for a guide-only edit that needs no change"
kind: bug
components: [clm]
---

Step: editing only pages under docs/guide/ (rewording guide pages, adding one), with no requirement or scenario affected.

What happened: CLAUDE.md's small-change route (`small-change-no-claim`) says `clm gate --base origin/main` must print `code`, but a branch touching only docs/guide/ prints `spec`, because docs/ is a spec path (`guide-from-docs`). The spec-branch route assumes a `spec/<change-id>` branch with a change.md and deltas, which a pure wording edit doesn't have. Neither route as written fits, so the session has to guess that a non-spec/ branch with no change, gating `spec` with no review, is acceptable to land.

Expected: CLAUDE.md (or the guide) names the route for a guide-only or decision-only edit with no change: its branch name, that the gate prints `spec`, and that it lands on checks.
