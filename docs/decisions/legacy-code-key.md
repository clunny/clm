---
title: "A land recorded without a code key is keyed with the doc-only list frozen as it was"
date: 2026-10-28
status: accepted
---

A `landed` record written before `landed-record-names-its-code-key` has no `code_key`, and `clm ledger verify` and `clm ledger compact` compute its key from the commit's tree. Their `checked` records hold the key the doc-only list gave at the time. When the list changes, as `20261027-guide-links-outside-the` changes it by counting `docs/spec-format.md` as code, recomputing with the live list gives every such land a key no `checked` record names: `verify` reports them all unchecked, and `compact` prunes the checks they rely on, which can't be undone.

A record without a key is now keyed with the *legacy* list: the doc-only list as it was when those records were written, frozen in `clm-ledger` beside `doc_only` and never following it. The live `doc_only` and `code_key` move with the repository, and every land since records its key, so the frozen list is read only for the lands that predate the field.

Alternatives considered:
- **Accept a keyless land when either its live or its legacy key is checked.** Two lookups per old land, and a future list change would need a third; the frozen list names the one list those checks were made with.
- **Treat any `checked` record older than a keyless land as its check.** Loses the audit: a land that skipped its check would pass on another tree's record.
- **Rewrite the old records with their keys.** Ledger branches are append-only facts per writer (`per-writer-ledgers`), and another writer's branch is never touched.

The cost is one frozen list that nothing else reads. The next change to the doc-only list needs no second frozen list: every land that follows `landed-record-names-its-code-key` carries its key.
