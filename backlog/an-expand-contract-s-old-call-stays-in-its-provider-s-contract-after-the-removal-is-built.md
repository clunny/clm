---
title: "An expand-contract's old call stays in its provider's Contract after the removal is built"
kind: idea
components: [orchestrator.triage, plugin]
origin: 20261122-land-approval-s-spec
---

A change that plans an expand-contract writes the provider's Contract and Design with the old call kept "until" its users move, and a removing requirement (or a `clm:remove-by` marker) that takes the call out once they have. The removal is built by code, but a build can't edit `specs/`, so the Contract and Design keep naming the old call the moment the removal lands, against `spec-is-current`, until someone notices and drafts a spec branch. It happened with `20261121-wave-3-land-by-the-rule`: `land.approval/old-calls-gone` was built and `land.approval`'s Contract still offered `Repo::head` and the free `held` (fixed by `20261122-land-approval-s-spec`). The same will happen to `land`'s `run_check` once `backlog/the-check-runner-s-warning-names-clm-land-when-clm-verify-runs-it` is built.

Ways it might be closed, none weighed yet: the planning change states the contract step's Contract edit as a second change on the same branch, left `proposed` and applied when the removal is built, as `contract-step-deferred` already does for a whole removal; or the removing build files the Contract edit as a `bug` item when it lands, so the next triage drafts it; or `clm check` reports a Contract that names a call a `clm:remove-by` marker or a built removing requirement has taken out.
