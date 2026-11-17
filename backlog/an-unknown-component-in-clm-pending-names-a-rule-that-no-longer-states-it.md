---
title: "An unknown component in clm pending names a rule that no longer states it"
kind: bug
components: [check]
---

`check/pending-narrows-on-a-graph` says `pending_for` refuses a component the graph doesn't hold with a violation of the rule `check.coverage/pending-report`, and `crates/clm-check/src/lib.rs` does so. Since `20261117-clauses-only-a-command-shows`, `check.coverage/pending-report` no longer states that refusal: it says narrowing and the refusal are `check/pending-narrows-on-a-graph`'s. So `clm pending nope` points the reader at a requirement that says nothing about unknown components. A spec change should have `pending-narrows-on-a-graph` name itself, `check/pending-narrows-on-a-graph`, as the violation's rule, and `check`'s build then change the string and its tests. Found by the review of `impl/check.coverage`.
