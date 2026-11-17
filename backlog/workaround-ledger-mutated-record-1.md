---
title: "Stopped to raise a spec change: the added field breaks verify's struct literals"
kind: workaround
components: [ledger]
requirement: ledger/mutated-record@4137cc10
---

Building `ledger/mutated-record` needs a field added to `Body::Mutated`, which component `verify` builds by struct literal in crates/clm-verify/src/mutate.rs and its tests. No requirement offers a constructor or defaults first (`coupled-changes-sequence`), so the build stopped and raised a spec change: backlog/adding-left-to-mutated-records-breaks-verifys-struct-literals.md.
