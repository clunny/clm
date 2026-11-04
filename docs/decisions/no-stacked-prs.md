---
title: "No stacked PRs"
date: 2026-10-21
status: accepted
---

PR #2 was merged into its base branch before GitHub retargeted it, so its code missed `main` (fixed by PR #3). Every PR targets `main`. Implementation of a change starts after its spec PR merges.
