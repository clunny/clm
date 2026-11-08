---
title: "A clone runs one `[land] check` at a time"
date: 2026-11-05
status: accepted
---

Every run of `[land] check` on a clone, by `clm land` or by `clm land --check-only`, holds an exclusive advisory lock on `.clm/check.lock` in the main worktree's CLM root while the command runs (`land/check-lock`). Builds and drafts that CLM runs side by side on one clone therefore never run their checks at the same time.

**Why.** The land lock (`land-target-lock`) guards the shared cargo target, so it covers only lands that compile into `.clm/land`; a build's pre-land `--check-only` run compiles into its own `target/` and took no lock. But a project's tests share more than a target: a fixed local port, a database, a browser profile. notespeed's browser tests bind a fixed port, so whichever of two overlapping checks started second failed for a reason unrelated to its branch, and a build spent its stop budget on retries or left its work unlanded. CLM can't know which host resources a project's tests take, so it doesn't overlap them at all. With no overlap, a failure caused by another run's check can't happen, so no build has to tell one apart or keep it off its retry budget.

**Why a lock of its own.** `land.lock` is held from the checkout to the push and only for lands on the shared target; widening it to every check would make a check-only run wait on gates and pushes and change which lands use the land tree. The check lock covers the check command alone. A land takes it after `land.lock` and `--check-only` takes only it, so the two can't deadlock.

**Cost.** Checks on a clone run one after another, so parallel builds wait for each other's pre-land checks as their lands already did for each other's land checks. Building and testing the code still run in parallel; only the configured check command is serialised.

**Amended 2026-11-07.** The lock covered only `[land] check`, while builds run the project's tests outside it on every edit and in spot-checks, and one project's fixed-port tests still failed beside another build (GitHub issue #266). Serialising those runs would undo parallel builds. The lock stays for every `[land] check` run, as it is; the per-run slot this decision rejected is now the rule (`build-slots`): every CLM-built project's tests must run concurrently on one host, keying what they must know in advance on `CLM_SLOT` (`tests-run-concurrently`). The objections below stand as costs: the rule binds every project as a carried constraint rather than relying on each to adopt it, and covers paths and servers as well as ports.

**Amended 2026-11-07, builds' checks.** With slots in place, the lock was the bottleneck its last alternative waited for: every build's pre-land `--check-only` queued behind every land's check and every other build's (`worktree-check-counts`). A `--check-only` run in a build place, whose `CLM_SLOT` isn't 0, now takes no lock, since its slot keeps its tests apart; a land's check, a `--check-only` run outside a place (slot 0) and `clm verify`'s local run still take it (`land/check-lock`).

**Alternatives.**
- *A port or slot per run* (an environment variable such as a run number the project's tests read). Rejected here, and later adopted beside the lock (see the amendment): it works only for projects that adopt it, covers ports and nothing else, and a project that forgets fails intermittently just as before.
- *Only warn in the method that fixed-port tests fail under parallel builds.* Rejected: it moves the defect to every adopter, and builds still waste their retries.
- *A setting to turn the lock off for projects whose tests share nothing.* Not now (`yagni`): nothing has shown the serialised checks to be the bottleneck; add it when one does.
