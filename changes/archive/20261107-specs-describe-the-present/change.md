---
status: done
---

## Intent

Have specs describe the system as it is now, not how it got there. `clm health` flags 19 Design and Contract lines in 13 components that narrate history ("as before", "no longer", "is gone", "predates", "Today's"). This change rewords each of them to state the present. Where a sentence only described a past state or a migration, it deletes the sentence. It also fixes `orchestrator.runner`'s Design, which said both that a component with an undeletable leftover worktree comes back "once the directory is gone" and that it comes back "when the claim lapses". `clm run --finish` keeps the claim, so the claim lapsing is what brings it back.

## Approaches

- **Reword only the flagged lines (chosen).** This is a light pass that clears `health/history-words`. The other slimming the spec-slimmer proposed is left for its own changes, because it moves code or tests: removing requirements, merging scenarios, and trimming Design text that repeats a requirement.
- **Slim each flagged component fully.** This would be a larger change that touches requirements and tests, so it would need builds.

## Outcome

Only Design and Contract text changes, and no requirement is touched. Nothing needs building, so the change is archived on its spec branch. `web.worker`'s "Moving them into the replica waits until they show up over budget" is dropped because `backlog/replica-spec-reviews` already holds that idea. The **Size** paragraph's note that the budget is due a review once the view code left goes to `backlog/web-worker-budget-above-its-code` (`fold-small-defer-large`).
