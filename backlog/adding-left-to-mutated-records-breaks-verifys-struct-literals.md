---
title: "Adding `left` to a `mutated` record breaks the struct literals `verify` builds it with"
kind: question
components: [ledger, verify]
blocks: [ledger/mutated-record]
---

`ledger/mutated-record` (from `20261116-a-mutation-pass-that-stops`) adds a `left` field to the `mutated` record, so `clm_ledger::Body::Mutated` gains a field. Component `verify` builds that variant by struct literal in four places, its code (`crates/clm-verify/src/mutate.rs`, the record the mutation pass appends) and its tests (`crates/clm-verify/tests/file.rs`, and twice in `crates/clm-verify/tests/mutate.rs`), so the field breaks `verify`'s build the moment it exists. A build may change only its own component's files (`path-ownership`), and `coupled-changes-sequence` says an added field a user builds by struct literal lands in steps: the provider first offers a constructor or defaults, the user moves to them, and the field comes last. The change wrote no such step, so the `ledger` build left the requirement unbuilt; `verify/mutation-pass`, which `Needs` it, waits with it.

The spec change needed, in three steps. First, a `ledger` requirement offering a constructor such as `Body::mutated(commit, since, killed, unproven, not_judged)` beside the variant, its shape unchanged (a change to the shape itself, such as a `Mutated` struct inside the variant, would also break `verify`'s patterns in `src/file.rs` and its tests). Second, a `verify` requirement, needing the first, moving its four literals to the constructor; `ledger/mutated-record` then `Needs` it, and adds the field with the constructor setting `left` empty. Third, `verify/mutation-pass` sets `left` on the body the constructor returns, by matching it mutably or through a second constructor that takes `left`, so no caller's call changes when the field lands.
