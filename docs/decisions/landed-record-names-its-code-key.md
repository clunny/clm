---
title: "A `landed` record names its code key, and compaction keeps the check it names"
date: 2026-10-26
status: accepted
---

`clm ledger verify` reports a landed commit as unchecked when no `checked` record names its code key, and `clm ledger compact` used to drop every `checked` record older than the newest `landed` record. A land's `checked` record is always older than its `landed` record, so the first compaction made every earlier land look unchecked and turned `clm check` on `main` red (`backlog/ledger-compact-removes-the-checks-verify-needs`).

The `landed` record carries `code_key`, the key the land's `checked` record names, which `clm land` already computed to find or append that record: the merged tree's key, or, for a land taken with others on one check, the key that check ran on, the code key of the last merge commit of their push (`batched-lands`). Naming the land's `checked` record is all `clm ledger verify` and `clm ledger compact` ask of it. Compaction keeps each `checked` record whose key a land names, and drops the rest that are older than the newest land, as before. `verify` reads the key from the record and recomputes it only for a record written before the field existed.

Alternatives considered:
- **Compaction keeps the checks a land relied on, computing each land's key from its commit.** It fixes the bug with no new field, but compaction and every `clm check` on `main` list a tree per land, a cost that grows with history.
- **`verify` checks only the lands after the newest compaction.** It needs a marker for when compaction ran, across writers who compact at different times, and it gives up the audit of older lands, which is what the `checked` records are for.
- **`verify` drops `ledger/unchecked-land`.** A `landed` record would then say nothing about checks, and a land that skipped them would go unnoticed.

The cost is one `checked` line kept per land, the same growth as the `landed` lines that are `main`'s audit trail. The record's key is the writer's word, as every record is; in a signed repository the `landed` record is signed, which makes it attributable but not checked (`signed-ledger-records`).

The same change names the land's kind `land_kind`: a record is one flat JSON object whose `kind` says which of the four record kinds it is, so no field can share that name (`backlog/landed-record-kind-field-collides-with-the-record-kind`).
