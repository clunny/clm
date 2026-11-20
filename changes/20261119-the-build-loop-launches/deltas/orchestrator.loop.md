---
component: orchestrator.loop
base: {}
---

## MODIFIED Requirements

### Requirement: release <!-- base: 4d5201e5c2757655 -->
`clm build release <component>` SHALL fetch `origin` and, after refreshing the take-over holds as a step does, refuse with exit 1, changing nothing and naming the run, when the loop file lists `build:<component>` as `running` and not an orphan, or as an orphan a take-over holds (`take-over-holds-moving-orphans`); a run that is `planned`, `stopped` or `returned`, or an orphan no take-over holds, doesn't refuse it, since a `launch` the session hasn't reported with `clm build launched` has no subagent the loop knows of, and the release is the human's word, as a pause is (`pause`). Otherwise it SHALL run `clm branch delete impl/<component>` and, when that exits other than 0, print its stderr and exit 1 when it exited 1 and 2 otherwise, changing nothing more. Then, when the local `impl/<component>` exists, it SHALL rename it to `released/<component>/<time>`, `<time>` the UTC time as `YYYYMMDDTHHMMSSZ`, with `git branch -m`, also when a worktree has it checked out, and print `kept released/<component>/<time>`; it SHALL remove no worktree and delete no local branch. It SHALL then run `clm run --finish <component>`, remove `build:<component>` from the loop file, and print the actions to take now as `clm build step` does. A component with no branch on `origin`, no local branch and no run SHALL be released with exit 0.

#### Scenario: a dead session's build released
- GIVEN the loop file listing `build:a` as an orphan no take-over holds, `origin/impl/a` pushed from this clone, and a worktree at `.claude/worktrees/agent-1` with the local `impl/a` checked out at a commit `c` that `origin/impl/a` lacks, and an untracked file `notes.txt`
- WHEN `clm build release a` runs
- THEN it exits 0, `origin` has no `impl/a`, the clone has no local `impl/a`, it printed `kept released/a/` with the time, that branch is at `c` and the worktree has it checked out with `notes.txt` still there, `clm run --finish a` ran, and the loop file lists no `build:a`

#### Scenario: a live run
- GIVEN the loop file listing `build:a` as running with task ID `t-1`, not an orphan, and `origin/impl/a` pushed from this clone
- WHEN `clm build release a` runs
- THEN it exits 1 naming `build:a`, `clm branch delete` didn't run, and the local `impl/a` and the loop file are unchanged

#### Scenario: a printed build released
- GIVEN a loop file listing `build:a` as planned, not an orphan, with no task ID, as a step that printed `launch build:a build <model> <file>` leaves it, and `origin/impl/a` pushed from this clone
- WHEN `clm build release a` runs, and again in a fresh fixture whose loop file lists `build:a` as returned
- THEN each exits 0, `clm branch delete impl/a` and `clm run --finish a` ran, and the loop file lists no `build:a`

#### Scenario: another clone's claim
- GIVEN a `clm branch delete impl/a` that exits 1 saying this clone didn't push the tip, and a local `impl/a`
- WHEN `clm build release a` runs
- THEN it exits 1 with that reason on stderr, the local `impl/a` is unchanged, and `clm run --finish a` didn't run

#### Scenario: nothing to release
- GIVEN no `origin/impl/a`, no local `impl/a` and no run `build:a`
- WHEN `clm build release a` runs
- THEN it exits 0 and prints no `kept` line
