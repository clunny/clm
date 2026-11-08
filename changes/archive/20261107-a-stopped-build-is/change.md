---
status: done
resolves: [a-stopped-build-is-always-restarted-from-scratch-even-when]
---

## Intent

`/clm:build` resumes a stopped build's own subagent, rather than building the component again from scratch, when the build's work is sound and its stop came from outside the work: a usage or rate limit as now, and also a full disk, a network outage or a tool or host failure that has passed. Which way to go is the orchestrating session's judgement, from why the build stopped, and the skill states its criteria: a build that went wrong in its approach, ran out of context, or stopped again for the reason it was resumed for is built again as now. Decision: `resume-sound-builds`, superseding `resume-limited-builds`. Resolves `backlog/a-stopped-build-is-always-restarted-from-scratch-even-when`.

## Problem

Several builds stopped only because the disk filled during `clm land`. Their code was done and reviewed; only the land had to run again. The build skill allows one exception to "build it again with a new subagent", a usage or rate limit (`resume-limited-builds`), so the session built them all again. Each retry lost the stopped build's context, compiled the workspace again from an empty `target/` (several GB each on a machine already short of disk), and lost files the build hadn't committed, such as a drafted pull request body. The design was too narrow: the limit was one case of "the work is sound, the environment failed".

## Approaches

- **The session judges by the stop's cause, with stated criteria (chosen).** Generalises the existing limit rule, reusing all of its mechanics (no `clm run --finish`, the place held, `SendMessage`, the fall-back to the retry) (`reuse-first`). The `stopped:` reason is prose the build writes, so a judgement with named examples covers causes no list foresaw.
- **Resume every stop.** Rejected: a build that went wrong in its approach or ran out of context carries its mistake or has no room left; a fresh build is the better next step.
- **A list of resumable reasons in `clm`.** Rejected: needs a reason syntax the builds don't have and misses new failures (`yagni`).

## Outcome

- `plugin`: the requirement `build-resumes-limited-builds` becomes `build-resumes-sound-builds`, which states the judgement and its criteria, says a build is resumed at most once for the same reason (its second such stop is step 3's retry's first, so the retry's "twice in a row" still bounds the whole), has the session tell the human what to clear when only they can (freeing disk), and resumes once the cause has cleared. `build-subagents`, `build-finishes-builds` and `build-unclosed-builds` name the new rule where they named the old. The Design's paragraph on finishing builds says the same.
- Decision `resume-sound-builds`; `resume-limited-builds` is marked superseded.

## Edge cases and workarounds

- A cause the session misjudges as passing makes the resumed build stop again for it; the retry takes over.
- The resume needs the stopped build's worktree. When something removed it first (`clm wrap-up` removing a pushed, clean worktree, as `backlog/a-stopped-build-s-worktree-is-removed-before-the-build-can-be` reports), `SendMessage` fails, which counts as a stop and goes to the retry. A resumed build keeps its place and counts as running, so `build` doesn't reach its own `clm wrap-up` while one waits; keeping the worktree against other removals is that item's work, not this change's.
- The retry's fresh build no longer fails on a worktree that still holds `impl/<component>`: `clm run` removes the stopped build's clean worktree when it resumes the branch (`20261107-stopped-build-with-unpushed`).

## Constraints

`builds-in-subagents`: the session still does no build work itself; it only resumes the build's own subagent. `redesign-before-workaround`: this widens the rule rather than adding a recovery step for disk-full stops. `method-in-the-plugin`: the judgement is written in the build skill, where sessions read it.

## Open questions

None.
