---
component: orchestrator.loop
base: {}
---

## MODIFIED Requirements

### Requirement: draft-of-gone-items-dropped <!-- base: 4dafc4af35047792 -->
After its fetch, and before its housekeeping, every command that prints the actions to take now, save `clm build pause --end`, SHALL finish each draft run the loop file lists as `planned`, or as an orphan in any state, that names at least one item and none of whose items `origin/main`'s tree holds as `backlog/<slug>.md`, while `origin` has no `spec/<change-id>` for it: it SHALL run `clm draft --finish <change-id>`, take the run off the loop file's list, keeping any task ID it has with the run name's others (`launched-refuses-an-earlier-task`), record none of its items among the items drafts of the loop left, pass each `left <path>: ...` line and each line starting `kept ` the finish prints to a `say`, as `return-rules` does for every `clm draft --finish` the loop runs, and print no other action for it. A draft any of whose items `origin/main` still holds, a draft that names no item, a draft with `origin/spec/<change-id>`, and a draft that is `running` or `stopped` and not an orphan SHALL be kept as before.

#### Scenario: a printed draft whose only item a build resolved
- GIVEN a step that printed `launch draft:20261119-x` for a draft of item `b`, no `launched` for it, no `origin/spec/20261119-x`, and then `origin/main` advanced to a commit that deletes `backlog/b.md`
- WHEN `clm build step` runs
- THEN it prints no line naming `draft:20261119-x`, `clm draft --finish 20261119-x` ran, and the loop file lists no `draft:20261119-x`

#### Scenario: one item still open
- GIVEN a step that printed `launch draft:20261119-x` for a draft of items `b` and `c`, no `launched` for it, and `origin/main` holding `backlog/c.md` and no `backlog/b.md`
- WHEN `clm build step` runs
- THEN it prints the same `launch draft:20261119-x` line again

#### Scenario: an orphan with a pushed spec branch
- GIVEN a lease renewed 20 minutes ago, a running `draft:20261119-x` of item `b` with task ID `t-1`, `origin/main` holding no `backlog/b.md`, and `origin/spec/20261119-x`
- WHEN `clm build start --session-model opus` runs
- THEN `clm draft --resume 20261119-x` ran and it prints a `launch draft:20261119-x`
