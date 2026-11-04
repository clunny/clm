---
component: orchestrator.run
---

## MODIFIED Requirements

### Requirement: run-finish <!-- base: df34cd4ab02473a8 -->
`clm run --finish <component>` SHALL fetch `origin`, free the place `<component>`, and, when `impl/<component>` has landed (at least one of `origin/impl/<component>` and the local `impl/<component>` exists, and the tip of each that exists is an ancestor of `origin/main`), print `landed impl/<component>` before any other line about worktrees, then remove each worktree under the main worktree's `.claude/worktrees/` that has `impl/<component>` checked out and no uncommitted changes or untracked files that aren't ignored, unlocking it first, printing `removed <path>`, and print `kept <path>: uncommitted changes` for one that has them. When the branch hasn't landed it SHALL print no `landed` line, remove no worktree and push or delete no branch. The `landed impl/<component>` line is how `build` learns that a build whose subagent ended without a closing line landed (`unclosed-build-reads-finish`). It SHALL never remove a worktree outside `.claude/worktrees/`, and SHALL exit 0, also when there was no place, and 2 when git fails.

#### Scenario: landed build tidied
- GIVEN a place `b`, and a clean, locked worktree at `.claude/worktrees/agent-1` with local `impl/b` checked out, whose tip is the second parent of a merge on `origin/main`, and no `origin/impl/b`
- WHEN `clm run --finish b` runs
- THEN it exits 0, the clone has no place `b`, the worktree is gone, and it printed `landed impl/b` and, after it, `removed` with its path

#### Scenario: stopped build kept
- GIVEN a place `b`, `origin/impl/b` with a commit `origin/main` lacks, and a clean worktree at `.claude/worktrees/agent-1` with `impl/b` checked out at that commit
- WHEN `clm run --finish b` runs
- THEN it exits 0, printed no line starting `landed`, the clone has no place `b`, the worktree and `origin/impl/b` are unchanged; and a following `clm run b` resumes the branch without claiming

#### Scenario: dirty worktree kept
- GIVEN the landed build of "landed build tidied" with an untracked file `notes.txt` in its worktree
- WHEN `clm run --finish b` runs
- THEN it exits 0, the worktree and `notes.txt` are still there, and it printed `landed impl/b` and `kept` with its path

#### Scenario: checkout outside the worktrees directory
- GIVEN local `impl/b`, landed, checked out in a clean linked worktree at `../other` outside `.claude/worktrees/`
- WHEN `clm run --finish b` runs
- THEN that worktree is still there

#### Scenario: nothing to finish
- GIVEN no place `b` and no branch `impl/b`
- WHEN `clm run --finish b` runs
- THEN it exits 0 and prints no line starting `landed`
