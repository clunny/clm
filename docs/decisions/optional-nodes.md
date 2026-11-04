---
title: "Optional nodes enforce web-optional: no required node depends on one"
date: 2026-10-23
status: accepted
---

`web-optional` is checked by a program (`deterministic-enforcement`), not left to prose. A node's frontmatter may set `optional: true`, and its descendants inherit it. Loading the spec graph fails when a required node has a `depends_on` entry naming an optional one, as it already fails on a cycle, so `clm check`, `clm apply` and CI all catch it. `web` is marked optional once the model reads the key (`mark-web-optional`), and any later add-on can be marked the same way.

The check covers the spec graph only. Code that imports another node's code without a `depends_on` edge is what `dependency-check` in the backlog would catch.

Alternatives considered:
- A check that names `web`: smaller, but it hard-codes one node into `check`, and each later optional part would need its own rule.
- Prose only, with the check deferred: it leaves the invariant unenforced.
