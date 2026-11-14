---
title: "Slow tests run after the land at moments the project chooses, on its host or its own machines, recorded in the ledger"
date: 2026-11-07
status: accepted
---

Resolves `backlog/slow-tests-after-land` (`changes/20261107-slow-tests-after-land/`), raised again by GitHub issue #262. Extends `deploy-is-downstream`, which left slow tests a choice between the land check and an unreported pipeline.

A project may set `[verify]` in `clm.toml`: a slow-test command, a `mode` and the triggers it wants. `clm verify --due` decides from git and the ledger whether a trigger has fired since the last verified commit of `main`, and when one has, verifies `main`'s head and appends a `verified` record. A red record names the land that broke it and its components; `clm intake` lists it and `clm land` warns a branch touching those components. When the human takes it in through triage as a `kind: bug` item with `verify_failures`, `clm land` refuses code lands on the item's components until a fix whose commit resolves that item lands; a component several such items hold admits a branch that resolves any one of them (`verify-stop-admits-each-fix`). Other components keep landing. Without `[verify]`, nothing changes: slow tests stay in `[land] check`.

**Triggers are the project's, over CLM's own units of work.** `change-done` (a change was archived), `queue-drained` (nothing pending), `work-done` (a build or triage ended, local only), `every` N lands or N days (a backstop), and `clm verify --commit` for a release. With AI builds landing all day, running continuously on the newest `main`, the usual continuous-delivery answer, never rests and blames dozens of lands at once; a finished change is the unit a human approved, so a red run points at one change. Which unit fits depends on how often a project lands and what a run costs, so the project picks; `/clm:init` recommends `change-done`. A red last record keeps any later land due, so a fix is checked at once.

**Three modes, one record.** `local` runs the tests on the machine that calls it. `pipeline` is the same command run by the host's pipeline on each push to `main`, which then needs the right to push `clm/ledger/*` (on GitHub, the workflow token's `contents: write`) and, in a signed repository, a key of its own. `import` lets the pipeline only report: `clm verify` on a developer's machine reads its runs through `host` and writes the record, so the host needs no write access. Every mode writes the ledger, so triage, the stop and a release check read results one way, with or without a host (`git-is-truth`). CLM writes no pipeline (`deploy-is-downstream`).

**Finding the culprit.** A run selects its tests with `clm affected --base <last green commit>`, so it covers every change since the tests last passed. A red run over several lands bisects them along `main`'s first parents (every land is one merge commit), in `local` and `pipeline` modes at once and in `import` mode by `clm verify --bisect` on a developer's machine. Nothing rewrites `main`: a fix lands forward, or a revert (`git revert -m 1 <land>`) lands through `clm land` and leaves the reverted requirements pending for the queue.

**A human's yes starts the stop; the host only reports.** `land-locally` took the decision of a land away from the host, and `deploy-is-downstream` says a host's pipelines report and don't decide. A red record from a `pipeline` or `import` run holding lands at once would hand the host that decision back, let a flaky run stop the line with nobody having looked, and in a signed repository need the pipeline to hold a key. So a red record is a report: `clm land` warns at once, triage offers it first, and the human's yes, taking it in as a bug, is the judgement that it is real, which `clm land` then enforces from git, the split `humans-review-intent` and `deterministic-enforcement` make everywhere else. `verified` records are therefore unsigned, like every kind but approvals and lands.

**A run shares the check lock.** A local run's check holds the clone's `check.lock` like every other check (`check-runs-one-at-a-time`): CLM can't know what a project's tests share, and a slow run beside a land's check could fail either for a reason that isn't its own, a spurious red then sent through bisect and triage. A land on that clone waits while a slow run's check runs; the triggers make that occasional, and a project that can't accept it uses `pipeline` mode or runs `clm verify` from another clone, which has its own lock.

**The name.** `clm verify` keeps the usual meaning of verify, as Maven's `verify` phase runs integration tests after the build (`plain-terms`). `clm ledger verify`, which audits `main` against the ledger, is the odd one; the glossary defines both, and `backlog/ledger-verify-is-an-audit` renames it.

Rejected:
- **Continuous, latest-wins runs on `main`.** Rejected by the human: with AI landing continuously the suite never rests and each red spans many lands.
- **CLM reading host results with no ledger record.** Two ways to read results, and none for a project with no host.
- **Splitting `[land] check` into gating and after-land parts.** CLM would still have to make the after part run somewhere: this design under another name.
- **Starting the run in the background from `clm land`.** It dies with the session or a sleeping laptop; `work-done` covers the same need at a point where the session is still there.
- **Blocking every land while `main` is red.** One slow failure would stop unrelated work; the stop holds only the components the culprit touched.
- **A red record holding lands at once.** The host would decide lands again (`land-locally`), and a flake would stop the line unseen. Considered in the first draft; replaced after the architecture audit.
- **A lock of its own for slow runs**, so a land never waits. It leaves collisions to the project, the warn-only approach `check-runs-one-at-a-time` rejected.
