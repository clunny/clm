---
title: "The build loop launches a component before the component whose requirements it Needs"
kind: bug
components: [orchestrator.loop]
---

Step: `clm build start` / `clm build step` during a wave whose components include one (apply) with requirements that list another wave component's (model) requirements under `#### Needs`.

What happened: the loop claimed `impl/apply` and printed `launch build:apply` while model's pending requirements were unbuilt. `clm next` listed only apply, config and context as ready. Model, vcs, machine and gate.fingerprint were pending, with nothing in `clm view blockers` or `clm pending` blocking them, yet they were never offered. The queue said the rest "waits for disk" with 95G free. Building apply first means it must be built a second time once model lands, for the requirements that Need model's.

Also: when the session declined to launch the printed `build:apply`, `clm build release apply` refused with "still running or held by a session taken over", although no run had been launched or reported with `clm build launched`. The session had to report `clm build returned build:apply --result none --cause stopped` for a run that never started before release would work.

Expected: the loop prefers a component whose requirements others Need over those that Need them (or says why it can't), `clm next` names why a pending, unblocked component is not ready, and `release` works for a run that was printed but never launched.

Rule involved: `needs-alone-order`, `build-loop-in-clm`.
