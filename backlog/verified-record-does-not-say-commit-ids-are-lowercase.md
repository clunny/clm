---
title: "ledger/verified-record doesn't say a verified record's commit IDs are lowercase, though the fold rejects uppercase ones"
kind: question
components: [ledger]
---

`ledger/verified-record` says a `verified` line's `commit`, `since`, `lands` and `culprit` must hold "full 40-hex commit IDs". The build rejects an uppercase ID too (`full_id` in `crates/clm-ledger/src/record.rs`), and its `bad_result` test checks that, because git writes IDs in lowercase and `Fold::verifications` and `unresolved_reds` look records up by exact string: an uppercase ID would be held but never matched. The spec auditor flagged the lowercase rule as behaviour no requirement states.

Needed: a spec change that words the clause as "full 40-hex lowercase commit IDs, as git writes them", or one that says uppercase IDs are accepted and lowercased on read.
