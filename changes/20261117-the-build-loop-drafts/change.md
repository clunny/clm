---
status: applied
resolves: [the-build-loop-drafts-a-component-s-bug-fix-while-it-builds-that]
---

## Intent

The build loop no longer launches the draft of a bug group beside a build of the same component. A draft whose group names a component the loop is building, or is about to build in the same step, waits in next up with `waits for build:<component>`, takes no place, and is launched once that build has returned. So the draft's `clm claim <component> --fix` is never refused by the loop's own build, and a draft refused that way is never relaunched into the same refusal. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception.

## Problem

In one `clm build step` the loop printed `launch build:<component>` and, beside it, a `launch draft:` for a `kind: bug` group on that component. `clm run` claims the component before the step prints its `launch`, so the draft's `clm claim <component> --fix backlog/<slug>` was refused (`origin/impl/<component>` has a live commit) and the draft stopped with nothing landed. Reported with `--cause blocked`, which relaunches nothing, the run returned; but the next step's plan offered the same `group` line again, since nothing ties a draft to a component's build, and printed the same draft while the build still held the claim.

The design is wrong, not the code: `plan-order` treats a bug draft and a build as independent entries, though a code-only bug is fixed by component (`code-fix-resolves-bug`) and needs the claim the build holds. No requirement states that a draft waits for its component's build, so the fix is a requirement of this change (`code-fix-needs-a-written-rule`).

No change or code fix since 2026-11-10 fixes it: `20261116-the-build-loop-shows` adds the pause and the queue's printed state, and nothing on `main` reads a `group` line's components in the plan.

## Approaches

- **Taken: the plan holds such a draft back.** The plan already holds the loop's runs and reads each `group` line's components from `clm triage`, so the rule needs no new input and no new command (`reuse-first`): a draft whose components include one a build run of the loop holds (any state but `returned`), or one the same plan lists as a build, is listed with `waits for build:<component>` and takes no place. It covers both halves of the report: the draft is never launched beside the build, and after a refusal from any other cause the next step can't relaunch it while the build is held. It is the loop's rule, in `clm` (`build-loop-in-clm`).
- **Taken: the build goes first.** When a component is both ready to build and named by a bug group, the build is launched (or waits for a place) and the draft waits, whatever their priorities. Builds release requirements, a build may fold small bugs of its component itself (`fold-small-defer-large`), and letting the draft go first would leave a window between the `launch` and the draft's own claim in which the next step's `clm next` still offers the component, so the race would come back the other way.
- **Not taken: waiting on any live claim of the component, another clone's included.** The loop would have to judge claims' expiry, `orchestrator`'s rule, for a case the draft already handles: a draft whose fix claim is refused leaves its bugs with `left` lines (`code-fix-resolves-bug`), and the loop then drafts them no more. The loop's own build is the case the report shows and the loop knows for certain (`yagni`).
- **Not taken: holding the build back while a draft of its component runs.** A run records its items, not their components, so it would need a new field in the loop file, and the draft may land a spec change that changes what the build must do anyway; the case is rarer than the one reported (see Edge cases).
- **Not taken: accepting the defect,** the plan's default answer while the freeze holds. Each occurrence spends a whole draft session for nothing, and drafts are launched by the loop that builds every remaining phase; whether to build it now is the human's, so a question item asks.

## Outcome

- `orchestrator.loop/draft-waits-for-its-build` (new): the plan launches no draft of a `group` line naming a component whose build the loop holds or lists in the same plan; it lists it with `waits for build:<component>` (or `waits for stopped runs to resume` while those resume), it takes no place and no disk room, and it is launched as any other once no such build is held or listed. A line with no components never waits so.
- `orchestrator.loop/plan-order` (modified): its `group` lines exclude a draft that waits for its component's build.
- `backlog/the-fix-for-a-bug-draft-beside-its-component-s-build-is-outside-the-freeze` (`kind: question`) blocks the two requirements until the human answers.
- `docs/decisions/build-loop-in-clm.md`'s *Priority across drafts and builds* names the exception to a draft going first at equal priority, and why (the architecture audit's finding).
- No capability test sees a difference: `capability/build-from-spec` drains components, not `group` lines.

## Constraints

- `clm triage`'s `group` line and `clm draft` are unchanged; the loop reads the components the line already prints.
- `view`'s queue model is unchanged: the new reason is a string of the next section's.

## Edge cases

- **A high-priority bug waits behind a low-priority build of its component.** The build goes first whatever the priorities; the draft is launched once it returns.
- **A draft launched before its component became ready.** When a draft of the loop runs and a spec change elsewhere makes its component pending, the plan may launch the build while the draft runs, and the draft's fix claim is then refused; it leaves its bugs with `left` lines, as its prompt says. Accepted: rarer than the case reported, and the bugs reach the next loop.
- **An escalated build** isn't listed in the plan, so a draft of its component isn't held for it.

## Open questions

None.
