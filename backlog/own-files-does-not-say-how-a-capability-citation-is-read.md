---
title: "gate/own-files doesn't say how a capability test's citation is read"
kind: question
components: [gate]
choices:
  - "As a marker: `req:capability/seeing` cites `seeing`, not `see`, as now"
  - "As a plain substring, as the code-fix search reads `req:<component>/`"
---

`gate/own-files` lets a serving build edit a capability test "whose text at the merge base holds `req:capability/<slug>`". `foreign_files` (`crates/clm-gate/src/lib.rs`) reads that text with `clm_coverage::marker_spans` and matches a marker whose ID is the served capability, so a file citing `capability/seeing` is foreign to a build serving `capability/see`; the test `a_capability_test_citing_a_longer_slug_is_foreign` in `crates/clm-gate/tests/own_files.rs` proves it. Read literally, the text holds the substring either way, and the Design's code-fix search (`code-fix-resolves-bug`) says its own search runs "as is, with no marker parsing", so the two read differently. Both reviewers of the `gate` build that re-cited `own-files` at `37282f7d` found the gap. A spec branch should say which reading holds, so the test proves a written rule (`code-fix-needs-a-written-rule`).
