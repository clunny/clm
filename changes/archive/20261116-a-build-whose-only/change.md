---
status: done
resolves: [a-build-whose-only-work-is-deleting-an-unproven-item-is-told-its]
---

## Intent

This change only edits the backlog. It folds the bug into `backlog/the-build-prompt-and-skills-still-tell-sessions-what-programs-now-refuse`, the `deferred` item that already asks `clm run`'s prompt to say what an `unproven` item asks of a build, and deletes the bug.

## Problem

`clm run`'s ship step tells every build to confirm that `clm gate --base origin/main` prints `code`. A build whose only work is deleting an `unproven` item, because its tests are sound, changes no code, so its gate prints `spec`. The branch is well formed and lands on checks, but the prompt's step can't be met as written.

## Approaches

- **Taken: fold it into the existing deferred item.** The fix is a change to the fixed text of `clm run`'s prompt. That text is source, and no requirement may state it (`prompt-text-is-source`), so `code-fix-needs-a-written-rule` can't apply here: a requirement stating the gate step is exactly what that decision rules out. The deferred item on `orchestrator.run` already holds the prompt's wording about `unproven` items, so this case belongs in that bullet (`reuse-first`), and a code branch resolves it with a `Resolves:` trailer when the item comes due.
- **Rejected: a requirement of `orchestrator.run` stating the gate step.** That would be a scan of prompt text, which `prompt-text-is-source` removed.
- **Rejected: having the gate print `code` for a branch that only deletes an item.** No code file changed, so no component the item names is touched, and loosening `gate/backlog-additions` there would let any branch delete any bug.
- **Rejected: fixing it now on `impl/orchestrator.run`.** The freeze answers defects with "accept" by default (`backlog/a-passing-check-proves-less-than-the-spec-claims`), and the cost is small: builds that met this case read the gate's `spec` correctly and landed (`0dff6fcd`, `504f965b`, `4d9b6aa9`, `0bf15406`).

## Outcome

- The deferred item's bullet on `unproven` items now also says that a build that only deletes the item sees its gate print `spec`, and the ship step should accept that, not lead the build to change a sound test.
- The bug's item is deleted.
- A CLM defect met while drafting is reported: `backlog/the-draft-prompt-gives-a-bug-in-prompt-wording-no-route`.

## Accepted

- **The draft's fix-branch prompt (`crates/clm-triage/src/draft.rs`).** Its `prints code` step can't meet the same case. `clm triage` never prints a `bug` item that holds `unproven` entries (`orchestrator.triage`'s triage requirement), so a draft never takes one. A draft that finds a bug already fixed deletes it on its spec branch, not on a fix branch, so a draft's fix branch always changes code.
- **A requirement whose rule lives in data may be filed `unproven` again.** That happens only when its tests change, because a pass never judges an unchanged test twice, and deleting the item settles it. The plan already gives such requirements, ones whose behaviour is another file's or another component's, to phase 5: rewrite the requirement, or delete the item (`backlog/a-passing-check-proves-less-than-the-spec-claims`, phase 3's "A changed test is judged again").

## Edge cases and workarounds

None.

## Open questions

None.
