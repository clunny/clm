---
title: "Each build place holds a slot, which its tests key host resources on"
date: 2026-11-07
status: accepted
---

Every build place on a clone holds a *slot*: the smallest number from 1 up that no other place that counts holds, chosen when the place is taken and kept while it is reused (`orchestrator/place-slots`). `clm land` and `clm land --check-only` run `[land] check` with `CLM_SLOT` set to the slot of the place that names the branch checked, or 0 (`land/check-slot`), and `clm run`'s prompt names the build's slot and has the session set `CLM_SLOT` on every test command it runs by hand (`orchestrator.run/prompt-slot`). A project's tests key any port or path they must know in advance on it (`tests-run-concurrently`).

**Why.** The check lock (`check-runs-one-at-a-time`) serialises only `[land] check`. Builds run the project's tests outside it all the time: after each edit, in mutation spot-checks, in the review's re-runs. A project whose check starts a preview server on a fixed port failed with "port already used" beside another build (GitHub issue #266), with the lock in place. Taking the lock around every test run would serialise the builds themselves, the thing that makes CLM fast. The human judged it the project's part to make its tests able to overlap, as an opinionated rule on every CLM-built project; CLM's part is a number that tells the runs apart.

**Why a small number from the place.** A port needs a number, and a project computes `base + slot` simply when slots are small and dense. The place is already the thing that tells builds of a clone apart and is chosen under one lock, so two builds can't get the same slot. A name (the component ID) would need hashing into a port range, where two names can collide. A hash of the worktree's path has the same flaw and would change for the same build in another worktree.

**Why 0 outside a place.** The human's own checkout, a draft's code-fix branch and anything else not in a place share slot 0, which an unset `CLM_SLOT` also means. Those runs are few and attended; giving each a slot would need a place for each.

**Alternatives.**
- *Widen the check lock to every test run.* Rejected: it serialises builds.
- *A slot only inside `[land] check`.* Rejected: builds' hand-run tests are where most runs happen.
- *Only state the rule, give no value.* Rejected: tests that must know a port in advance, such as a browser suite pointed at a server the check starts, have nothing to key it on but a value CLM supplies.

## The rule in full

`ARCHITECTURE.md`'s entry `tests-run-concurrently` states this in short (`human-layer-entries-stay-short`); the rule in full:

A project's tests, and the check command that runs them, SHALL pass while other runs of them overlap on the same host, each in its own checkout: no test binds a fixed port, writes a fixed path outside its checkout and the system's temporary directory, or relies on a server or database another run shares. A port or path a run must know in advance, such as the port of a server the check starts, is keyed on `CLM_SLOT`, the number CLM gives each build and its checks, distinct among the builds of one clone, and 0 when unset (`build-slots`). CLM runs builds in parallel and runs their tests outside any lock, so making the tests able to overlap is the project's part of that.

**Scenario: a preview server**
- GIVEN a project whose check starts a preview server for its browser tests, and two builds of it on one clone
- WHEN both run their tests at once
- THEN each server listens on a port derived from its own `CLM_SLOT`, or one the system chose, and both runs pass

**Guarantee**
Two runs of the project's tests on one host, from different build places of a clone, never fail each other through a resource the host holds, so CLM can build in parallel without serialising the tests its builds run.

**Untested**
Whether a project's tests take a host resource another run shares is something CLM can't see; a collision shows as a test that fails beside another build and passes alone. Runs outside a build place (the human's own checkout, `clm verify`'s local run, a draft's code fix) all have slot 0, so two of them can still collide on a port keyed on it; the check lock keeps their `[land] check` and `[verify] check` runs apart, but not their tests run by hand.
