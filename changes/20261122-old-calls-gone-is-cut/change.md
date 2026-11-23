---
status: applied
resolves: [land-approval-s-old-calls-gone-names-calls-its-contract-no-longer-offers]
---

## Intent

Cut `land.approval/old-calls-gone` and its test, `crates/clm-approval/tests/old_calls_gone.rs::the_old_calls_are_gone`. The human agreed it on 2026-11-22 in the wave 4 session, answering `land-approval-s-old-calls-gone-names-calls-its-contract-no-longer-offers` with "Cut old-calls-gone and its test".

## Problem

The requirement was the contract step of `20261121-wave-3-land-by-the-rule`'s expand-contract, and it is built: `clm-approval` no longer offers `Repo::head(branch, refuse_unpushed)` or the free `held(fold, branch, fingerprint)`. Since `20261122-land-approval-s-spec` dropped both calls from the Contract, the requirement states only an absence and names two calls the Contract doesn't offer, which `requirements-state-what-the-contract-shows` counts as a fault. It can't be reworded to name neither call and still say anything.

## Outcome

- `land.approval/old-calls-gone` is removed, and `deleted-tests.txt` lets `land.approval`'s next build delete the one test that cites it.
- What it guarded stays stated where it belongs: a land takes its head by the rule `land.approval/branch-head` and `land`'s requirements state, and a hold is judged with signatures through `Repo::held` (`land.approval/request-refusal` and `land/changes-requested-holds`).
- `three-routes-onto-main` records that the requirement was built and then cut.
