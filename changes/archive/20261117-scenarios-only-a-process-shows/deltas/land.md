---
component: land
---

## MODIFIED Requirements



### Requirement: land-binary-copy <!-- base: 7e2d6c4e9e96b4ff -->
Before running its first subcommand, `clm land` SHALL copy its own binary into a fresh directory under the system's temporary directory and run the gate and `clm check` from that copy, unless `CLM_GATE` or `CLM_CHECK` names the command; it SHALL remove the copy on every exit path. When the copy can't be made it SHALL exit 1 naming the path, pushing and recording nothing. That the land runs from the copy and leaves none behind needs the `clm` binary's own file, which only a `clm` process has, so the root states and proves it (`clm/land-as-a-process`).

#### Scenario: the copy can't be made
- GIVEN a `TMPDIR` that is a file, so no directory can be made in it
- WHEN a land of a branch runs
- THEN it is refused naming that path, the gate never ran, `main` is unchanged and no record is written

### Requirement: land-target-lock <!-- base: 2dadb710cf7e89e2 -->
A `clm land` that will run `[land] check` SHALL hold an exclusive advisory lock on `.clm/land.lock` beside the main worktree's `.clm/land` from before it checks the head out into the land tree until the push of the merge commit has gone through and the target's cap has been applied (`land-target-cap`), or the attempt ends sooner, so that the gate on the merge commit, `clm check`, the push and the cap run under it, releasing it on every exit path, whatever `CARGO_TARGET_DIR` the caller set. A land that waits for the lock SHALL say so on stderr, naming `.clm/land`. A land that waited for the lock SHALL merge its branch with `main` as `origin` holds it once the lock is its own, never with a `main` it fetched before it waited, whichever check it then runs. A land that runs no `[land] check`, because its code key was already checked or no check is set, SHALL NOT take this lock. That two `clm land` processes never run their checks at once needs two processes, so the root states and proves it (`clm/land-as-a-process`).

#### Scenario: checked code takes no lock
- GIVEN a branch that changes only `specs/x/SPEC.md`, a `checked` record for `origin/main`'s code key, and another process holding the lock
- WHEN `clm land` runs
- THEN it lands without waiting and prints no `waiting` line

#### Scenario: a land that waited checks the new main
- GIVEN a branch `impl/a` with no `checked` record, a branch `spec/x` whose merged tree's code key has a `checked` record and whose readers select a test, and a `[land] check` that appends to a log the files of the tree it runs in and sleeps 2 seconds
- WHEN `clm land impl/a` starts, and `clm land spec/x` starts once `impl/a` holds the lock
- THEN both land, `spec/x`'s check ran in a tree holding `impl/a`'s file, and `spec/x`'s stderr holds no `main moved` line

#### Scenario: lock released after a failing check
- GIVEN a `[land] check` that exits 1
- WHEN `clm land` runs and then a second `clm land` runs
- THEN the first exits 1 and the second doesn't print `waiting`

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- init/starter-check-land-dir
- upgrade/migration-starter-land-dir
