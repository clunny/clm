---
title: "Adding `left` to a `mutated` record breaks the struct literals `verify` builds it with"
kind: question
components: [ledger, verify]
blocks: [ledger/mutated-record]
choices: ["ledger offers a constructor first", "Mutated becomes a struct with Default"]
---

`ledger/mutated-record` (from `20261116-a-mutation-pass-that-stops`) adds a `left` field to the `mutated` record, so `clm_ledger::Body::Mutated` gains a field. Component `verify` builds that variant by struct literal in its code (`crates/clm-verify/src/mutate.rs`, the record the mutation pass appends) and its tests (`crates/clm-verify/tests/file.rs`, `crates/clm-verify/tests/mutate.rs`), so the field breaks `verify`'s build the moment it exists. A build may change only its own component's files (`path-ownership`), and `coupled-changes-sequence` says an added field a user builds by struct literal lands in steps: the provider first offers a constructor or defaults, the user moves to them, and the field comes last. The change wrote no such step, so the `ledger` build left the requirement unbuilt; `verify/mutation-pass`, which `Needs` it, waits with it.

The spec change needed: a `ledger` requirement that offers a way to build a `mutated` body that keeps compiling when a field is added (for instance a `Body::mutated(commit, since, killed, unproven, not_judged)` constructor, or the variant holding a `Mutated` struct that derives `Default`), a `verify` requirement that moves its three literals to it, which `ledger/mutated-record` then `Needs`, and `verify/mutation-pass` then writing `left` through the new field.
