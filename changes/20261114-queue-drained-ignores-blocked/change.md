---
status: proposed
resolves: [the-queue-drained-trigger-never-fires-while-any-requirement-is]
---

## Intent

Make the `queue-drained` trigger of `[verify]` hold when no requirement is ready to build, as `clm next` reads a requirement, instead of when none is pending. A requirement blocked by a backlog item or by a requirement it waits on is not work a build is about to do, so it no longer keeps the slow tests and the mutation pass from running once the builds have finished. This is the first step under "Next, after phase 3" of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims`, and it is within the freeze: the plan names it.

## Problem

`verify/verify-due` says `queue-drained` holds "when no requirement in `origin/main`'s tree is pending", and the code does exactly that: `queue_drained` in `crates/clm-verify/src/due.rs` returns whether `clm_coverage::pending` is empty, and its test, `queue_drained_waits_for_the_last_pending_requirement`, proves it. So the item's claim is right, and this is the requirement as written, not code that fell short of it.

A blocked requirement is pending. This repository holds four on purpose: `capability/measure-by-evals`, blocked by the plan until phase 6, and three of `land`, blocked by a question only a measurement answers. The trigger has therefore never held here and can't until those are built. Every verification came from `every = { lands = 20 }` or was run by hand, so a finished build's lands went unverified, and its new tests unjudged by the mutation pass, for up to 19 further lands.

The build loop already asks at the right moments: after each land of one of its runs and when it goes idle, it runs `clm verify --due --dry-run` (`orchestrator.loop`). Only the trigger's answer is wrong.

## Approaches

- **Every pending requirement is blocked (chosen).** `check.coverage`'s `pending` already returns each pending requirement with what blocks it: `blocked_by`, the backlog items, and `waits_on`, the requirements it needs, those a move took an obsolete one to, and the rules a capability waits on. `clm next` calls a requirement blocked when either holds something. The trigger holds when that is so of every pending requirement, and so also when none is pending. It needs no new dependency, no new Contract of another component, and one requirement of `verify`.
- **Ask the queue itself, claims included.** Have verify compute `orchestrator`'s queue and hold when `clm next` would print no component. A claimed component isn't printed, so the trigger would hold while builds are still running: the slow tests would run in the middle of a burst and again at its end, each run holding the check lock that lands wait on. It would also make `verify` depend on `orchestrator` and read `impl/*` refs. Rejected. Claims need no reading at all: a requirement a build is working on is still pending and unblocked in `origin/main` until that build lands, so the chosen rule already waits for it, and a lapsed claim changes nothing either.
- **A second trigger beside the first**, such as `nothing-ready`, leaving `queue-drained` as it is. It would add a settings value, a migration and a choice no project wants: nobody chooses a trigger that a single parked requirement switches off for good. Rejected (`yagni`).

## Outcome

- `clm verify --due` finds `queue-drained` when at least one land is unverified and every pending requirement of `origin/main` is blocked, or none is pending.
- It still doesn't hold while a requirement is ready, whether or not a build holds its component, nor when the spec graph has violations.
- The guide's line for the trigger and the decision `verify-after-land` say the same.
- `verify`'s build changes `queue_drained` and its test, and re-cites the tests of `verify-due`'s other scenarios, which stay true as written. No capability test observes the trigger.

## What an adopter's project sees

Only a project whose `[verify] triggers` holds `queue-drained` sees anything, and only while it has a requirement that is pending and blocked. Before, the trigger was dead there for as long as that requirement stayed blocked. After it upgrades `clm`:

- `clm verify --due` prints `verify due queue-drained` and runs the slow tests as soon as a land is unverified and nothing is left that a build could take. The first call after the upgrade is likely to run at once, over every land since the last record.
- In `local` mode the build loop asks after each of its lands and when it goes idle, so the run comes when the last ready component's build lands. In `pipeline` mode the host asks on each push to `main`, so every push that leaves nothing ready runs the slow tests, as it already does in a project with nothing pending.
- With `[verify] mutate` set, the mutation pass follows each passing run, so new tests are judged at the end of a burst of builds instead of after `every.lands` further lands.
- A project with nothing blocked, or without the trigger, sees no change.

No migration: no file `clm init` writes, no settings key and no format changes, and the template's comment names the trigger without defining it. The trigger's meaning changes for a project that holds the key; the guide's page says the new meaning, and there is nothing for such a project to do (`upgrades-carry-migrations` covers a file, key or format, and this is none).

## Constraints

- `verify` keeps its dependencies: the rule is computed from `check.coverage`'s `pending`, which `verify` already calls.
- The trigger's name and the settings stay as they are.
