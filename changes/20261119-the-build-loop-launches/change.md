---
status: applied
resolves: [the-build-loop-launches-a-component-before-the-component-whose]
---

## Intent

At equal priority, `clm next` lists every component with no pending requirement that waits on other work before any that has one, so the build loop, which launches what `clm next` prints in its order and up to its cap, builds a component others Need (such as `model`) before one that Needs it (such as `apply`), and the one that Needs it is built once instead of twice whenever other work can fill the places meanwhile. `clm build release` also gives up a build the loop printed and the session never launched, instead of refusing it as running. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception.

## Problem

During a wave whose components Need one another's requirements, the loop claimed `impl/apply` and printed `launch build:apply` while `model`, whose pending requirements two of `apply`'s Need, was unbuilt. Read against the specs:

- **The components that weren't offered were ready, and the cap cut them.** `clm next` prints at most the clone's free places, sorted by effective priority and then by ID (`orchestrator/next`). Every component of the wave was at `P5`, so with three places free it printed `apply`, `config` and `context`, the first three by ID; `model`, `vcs`, `machine` and `gate.fingerprint` came later in the same order. `apply` is ready because most of its pending requirements are unblocked (`needs-alone-order`), so nothing in the order kept it behind what it waits on. Priority seeding (`priority-orders-nodes`) pulls waited-on work forward only when the waiting work has a higher priority; at equal priority the ID decides, and the ID knows nothing of the waits.
- **The loop can't fix the order itself.** It launches builds in `clm next`'s order (`orchestrator.loop/plan-order`) and never sees the components the cap cut.
- **`release` refuses a printed run.** `orchestrator.loop/release` refuses a run that is `planned` and not an orphan as it refuses a running one, though nothing the loop knows of is behind a `launch` the session hasn't reported. The session had to report `clm build returned build:apply --result none --cause stopped` for a run that never started, a step no rule states. `pause` (`20261116-the-build-loop-shows`, waiting for the plan's end) already takes a printed run on the human's word, and `release` doesn't refuse a paused build.
- **"Waits for disk" with 95G free** was `machine`'s measure, not this defect: it sets aside the largest build measured (about 21G) for each running build before counting the room. Not changed here.

No change or code fix since 2026-11-12 fixes it: the loop's changes of those days (`20261115-the-build-loop-waits`, `20261115-parallel-builds-fill-the`, `20261116-the-build-loop-shows`, `20261117-the-build-loop-drafts`, `20261119-the-build-loop-keeps` and the rest) touch neither `clm next`'s order nor the release's refusal of a planned run, and no commit since names this item.

## Approaches

- **Taken: a tie-break in `clm next`.** At equal effective priority, a component none of whose pending requirements has a `waits_on` entry comes before one with such a requirement, then by ID. `waits_on` is already in the queue (`check.coverage/pending-report`), so the rule needs no new input (`reuse-first`). It is `clm next`'s, since the cap is applied there and the loop keeps the order it is given (`build-loop-in-clm`, `orchestrator.loop/plan-order`). It only orders, so it changes no readiness and keeps `needs-alone-order`.
- **Taken: `release` takes a planned run.** A printed `launch` the session hasn't reported has no subagent the loop knows of, and the release is the human's word, as a pause is; `release` already takes a paused run and an orphan no take-over holds. The live risk is a session that launched a subagent and hasn't reported it yet, and that session is the one running the release.
- **Not taken: holding a waiting component back while what it waits on is pending.** That makes a wait order readiness at component level, the coarse wait `needs-alone-order` removed, and leaves places idle when nothing else is ready.
- **Not taken: listing the components others wait on first.** Priority seeding already does that across priorities, and once a component that waits sorts last nothing more is settled at equal priority (`yagni`).
- **Not taken: `clm next` naming why a ready component isn't printed.** The components weren't unready: its Contract says it prints at most the free places, and `clm status` and `clm view flight` list every ready component. With the tie-break, the cap falls on the components that wait.
- **Not taken: accepting the defect,** the plan's default answer while the freeze holds. Phase 5's waves are built through this loop and their components Need one another, so each such wave can spend an extra build per component that waits; whether to build it now is the human's, so a question item asks.

## Outcome

- `orchestrator/next` (modified): at equal effective priority, components with no waiting pending requirement come before those with one, then by ID. Its scenario *need against the node graph* now prints `b` then `a`, and a new scenario shows the order with and without the cap.
- `orchestrator`'s Design gains *Ties at equal priority*, with the reasons above.
- `orchestrator.loop/release` (modified): a `planned` run doesn't refuse the release; a new scenario *a printed build released*.
- `backlog/the-fix-for-a-build-ordered-before-what-it-needs-is-outside-the-freeze` (`kind: question`) blocks both requirements until the human answers.

## What the requirements no longer state

- `orchestrator.loop/release`: that it SHALL refuse with exit 1 when the loop file lists `build:<component>` as `planned` and not an orphan. Its test `a_planned_run_is_refused_and_a_returned_one_is_not` is in `deleted-tests.txt`.

## Constraints

- `clm next`'s output format, its cap and its readiness are unchanged; only the order of components at equal effective priority changes.
- The loop's plan, `view`'s queue model and `queue_from_refs` are unchanged.

## Edge cases

- **Nothing else is ready.** A component that waits is still offered for its unblocked requirements when the places reach it, and is then built again once what it waits on is current, as now.
- **A component both waited on and waiting** sorts with those that wait, since building it now still means building it again.
- **Waits on a backlog item** (`blocked_by`) don't count: an answer may be long in coming, and holding the component behind it would only delay the rest.

## Open questions

None.
