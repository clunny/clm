---
title: "Apply is the final step of the spec PR"
date: 2026-10-21
status: accepted
---

The author (with Claude) runs `clm apply` as the PR's last step, which folds deltas into `specs/`. Reviewers see the applied result, `clm gate`'s up-to-date rule (`gate/up-to-date`) serializes applies with no branch protection (`land-locally`), and no bot token is needed. Resolves the open question of where apply happens.
