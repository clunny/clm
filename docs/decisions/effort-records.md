---
title: "Effort is read from transcripts, recorded at land, and shown per capability"
date: 2026-10-30
status: accepted
---

Builds `effort-accounted` and the capability `review-capability-cost` (`changes/archive/20261029-refounding/design.md` §4). The human reviews what each capability costs, not rule by rule, and decides keep, cut, redesign or accept.

- **A node of its own, `effort`, with the crate `clm-effort`:** the transcript reader, `clm effort record`, the cost roll-up and `clm capabilities`. `ledger` gains only the `effort` and `decided` record kinds; `ledger` and `view` are over their size budgets already.
- **An `effort` record's `id` is `effort_id(session, branch)`**, the first 32 hex digits of SHA-256 over `<session>\n<branch>`. The fold keeps the latest by `at` for each `id`, whoever wrote it, and a run whose counts don't exceed the folded record appends nothing, so recording again is a no-op.
- **`clm land` records the branch's effort with its `landed` record**, in one commit; failing to read the transcripts warns and never fails the land.
- **Attribution:** `impl/<node>` names a node; `spec/<id>` is split equally over its change's delta nodes; everything else is unattributed. A node's cost goes to capabilities by its rules, each rule weighing 1, split equally over the capabilities it serves; a rule serving only architecture requirements puts its weight on an `architecture only` line. Approvals count whole, once for each capability the branch's nodes serve.
- **Prices** are a table in `clm-effort`, checked against the list prices on a stated date, matched by the model ID's family, with the `sonnet` row as the fallback; every cache write at one rate. The figures are estimates, never a bill, and the page says so.
- **"Can we live without it?" is computed**, never written by a model (`deterministic-enforcement`): how many of the capability's rules serve only it, and what the others also serve. A session presenting the page may add its own reading.

Rejected: a rule-level cost (the transcripts know branches, not rules); storing effort only on the machine that ran the session (`git-is-truth`: the ledger carries it to every clone); weighing rules by their size or test count (one weight per rule is explainable on the page, and the page says the split is an estimate).
