---
component: orchestrator.loop
base: {}
---

## ADDED Requirements

### Requirement: launched-refuses-an-earlier-task
`clm build launched <name> <task-id>` SHALL refuse with exit 1, the loop file unchanged, when `<task-id>` is a task ID the loop file records for another run, or for an earlier run of `<name>`: a run of that name the loop finished (by its return, by `claim-gone-ends-a-build` or by a refused relaunch, `loop-start`) or rebuilt with a new subagent (`stop-causes`) before a step planned the run now listed under `<name>`. Its stderr SHALL name the run that task belonged to, say that the loop has finished that run and that a late report of it changes nothing (`return-rules`), and say that the `launch` printed for `<name>` is a new run, whose prompt is launched as a new subagent and reported with that subagent's task ID. So that it can tell, when a step plans or relaunches a run under the name of a run the loop file lists, the new run SHALL keep the task IDs reported for that run and for the runs of its name before it.

#### Scenario: a landed build's task reported for its successor
- GIVEN a running `build:a` with task ID `t-1` and no `origin/impl/a`, a fake `clm run --finish a` that prints `landed impl/a`, and component `a` still ready
- WHEN `clm build step` runs, printing `launch build:a`, and then `clm build launched build:a t-1`
- THEN `launched` exits 1, its stderr names `t-1` as the task of the run of `build:a` the loop finished and says the printed `launch` is a new run, and the loop file lists `build:a` as planned with no task ID; `clm build launched build:a t-2` then records it running with `t-2`

#### Scenario: a rebuilt run's old task
- GIVEN `build:a` with task ID `t-1`, finished and rebuilt after a `context` stop, its `launch` printed again
- WHEN `clm build launched build:a t-1` runs
- THEN it exits 1 and the loop file is unchanged

#### Scenario: another run's task
- GIVEN a running `build:b` with task ID `t-1`, and a planned `build:a`
- WHEN `clm build launched build:a t-1` runs
- THEN it exits 1 naming `build:b`, and the loop file is unchanged
