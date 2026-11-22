---
title: "`land.approval/old-calls-gone` names calls its Contract no longer offers"
kind: question
components: [land.approval]
choices:
  - "Cut old-calls-gone and its test"
  - "Keep it as it stands"
---

`land.approval/old-calls-gone` says `clm-approval` SHALL no longer offer `Repo::head(branch, refuse_unpushed)` or the free `held(fold, branch, fingerprint)`, and `crates/clm-approval/tests/old_calls_gone.rs::the_old_calls_are_gone` proves it by compiling a caller of each. It was the contract step of `20261121-wave-3-land-by-the-rule`'s expand-contract, and it is built. Since `20261122-land-approval-s-spec` dropped both calls from the Contract, the requirement states only an absence and names two calls the Contract doesn't offer, which `requirements-state-what-the-contract-shows` counts as a fault.

It can't be reworded to name neither call and still say anything, so the choice is to cut it, which deletes its test and needs your yes (`removal-needs-a-yes`), or to keep it as the guard that no unsigned hold judgement comes back as a public call. On a cut, a spec branch removes the requirement and lists the test in its `deleted-tests.txt`; on a keep, this item is deleted and the requirement stays as an exception by name.
