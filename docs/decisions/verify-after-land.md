---
title: "Slow tests run after the land at moments the project chooses, on its host or its own machines, recorded in the ledger"
date: 2026-11-07
status: accepted
---

Resolves `backlog/slow-tests-after-land` (`changes/20261107-slow-tests-after-land/`), raised again by GitHub issue #262. Extends `deploy-is-downstream`, which left slow tests a choice between the land check and an unreported pipeline.

A project may set `[verify]` in `clm.toml`: a slow-test command, a `mode` and the triggers it wants. `clm verify --due` decides from git and the ledger whether a trigger has fired since the last verified commit of `main`, and when one has, verifies `main`'s head and appends a `verified` record. A red record names the land that broke it and its components; `clm intake` lists it, triage takes it in as a `kind: bug` item with `verify_failures`, and `clm land` refuses code lands on those components until a fix whose commit resolves that item lands or a later run passes. Other components keep landing. Without `[verify]`, nothing changes: slow tests stay in `[land] check`.

**Triggers are the project's, over CLM's own units of work.** `change-done` (a change was archived), `queue-drained` (nothing pending), `work-done` (a build or triage ended, local only), `every` N lands or N days (a backstop), and `clm verify --commit` for a release. With AI builds landing all day, running continuously on the newest `main`, the usual continuous-delivery answer, never rests and blames dozens of lands at once; a finished change is the unit a human approved, so a red run points at one change. Which unit fits depends on how often a project lands and what a run costs, so the project picks; `/clm:init` recommends `change-done`. A red last record keeps any later land due, so a fix is checked at once.

**Three modes, one record.** `local` runs the tests on the machine that calls it. `pipeline` is the same command run by the host's pipeline on each push to `main`, which then needs the right to push `clm/ledger/*` (on GitHub, the workflow token's `contents: write`) and, in a signed repository, a key of its own. `import` lets the pipeline only report: `clm verify` on a developer's machine reads its runs through `host` and writes the record, so the host needs no write access. Every mode writes the ledger, so triage, the stop and a release check read results one way, with or without a host (`git-is-truth`). CLM writes no pipeline (`deploy-is-downstream`).

**Finding the culprit.** A run selects its tests with `clm affected --base <last green commit>`, so it covers every change since the tests last passed. A red run over several lands bisects them along `main`'s first parents (every land is one merge commit), in `local` and `pipeline` modes at once and in `import` mode by `clm verify --bisect` on a developer's machine. Nothing rewrites `main`: a fix lands forward, or a revert (`git revert -m 1 <land>`) lands through `clm land` and leaves the reverted requirements pending for the queue.

**A run never holds up a land.** It runs in its own tree under its own lock, holding neither the land lock nor the check lock, so the project keeps its slow tests from colliding with `[land] check` on one machine.

Rejected:
- **Continuous, latest-wins runs on `main`.** Rejected by the human: with AI landing continuously the suite never rests and each red spans many lands.
- **CLM reading host results with no ledger record.** Two ways to read results, and none for a project with no host.
- **Splitting `[land] check` into gating and after-land parts.** CLM would still have to make the after part run somewhere: this design under another name.
- **Starting the run in the background from `clm land`.** It dies with the session or a sleeping laptop; `work-done` covers the same need at a point where the session is still there.
- **Blocking every land while `main` is red.** One slow failure would stop unrelated work; the stop holds only the components the culprit touched.
