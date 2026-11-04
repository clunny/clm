---
title: "`undeclared-imports` also judges the graph a pending change would leave"
date: 2026-10-31
status: accepted
---

`undeclared-imports` judged the code's import edges only against `specs/` as it is, so a `proposed` change whose deltas dropped a `depends_on` entry its node's code still uses passed `clm check`. After `clm apply` the same tree failed the rule, but `clm sync` and `clm gate` don't run it, so only `clm land` caught it, at the end (`backlog/clm-check-on-a-proposed-change-misses-the-undeclared-imports-its`, met on `spec/20261031-split-view`).

Now the rule also judges the same edges against the graph each `proposed` or `approved` change would leave, which `apply`'s `resulting(graph, change)` already gives `removed-names-before-apply`. A pair only the resulting graph leaves undeclared is reported with `wouldn't depend on ... after <change>`; one both leave undeclared keeps the current line alone.

Rejected: running the rule in `clm gate` or `clm sync`. It would catch the miss only after `clm apply`, the step that ends the draft, and `gate` would gain a tree-sitter dependency for one rule `clm check` already runs; judging the would-be graph catches it in the check a draft runs before applying, reusing the graph `removed-names-before-apply` builds (`reuse-first`).
