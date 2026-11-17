---
component: orchestrator.loop
base: {}
---

## ADDED Requirements

### Requirement: launched-refuses-an-earlier-task
`clm build launched <name> <task-id>` SHALL refuse with exit 1, changing nothing in the loop file (its lease included), when `<task-id>` is a task ID the loop file records other than the one recorded for the run now listed as `<name>`: another run's, in any state, or one kept as an earlier run's. The loop file SHALL keep, with each run, the task IDs it reported before: whenever the loop clears a run's task ID or replaces a listed run with a new run of its name (a rebuild, `stop-causes`; an orphan's relaunch, `loop-start`; a step planning the name of a run that returned or that `claim-gone-ends-a-build` finished), the cleared or replaced run's task ID and the IDs it kept SHALL stay with the run of that name. The field SHALL be optional, so a loop file an earlier `clm` wrote loads unchanged and its `version` stays the same. Its stderr SHALL name the run that task belonged to and, for an earlier run of `<name>`, say that the loop has finished that run and that a late `landed` report of it changes nothing (`return-rules`), and that the action printed for `<name>` is a new run: for a `launch`, to be started as a new subagent, and for a `run`, as a new command, each reported with its own task ID. The build of this requirement updates the capability test `capability/build-from-spec` (`crates/clm/tests/capability/build_from_spec.rs`), whose `drain_the_queue` and `resume_in_a_new_session` report one task ID for two runs, to report a distinct ID for each run, keeping its citations (`capability-tests-follow-rules`).

#### Scenario: a landed build's task reported for its successor
- GIVEN a running `build:a` with task ID `t-1` and no `origin/impl/a`, a fake `clm run --finish a` that prints `landed impl/a`, and component `a` still ready
- WHEN `clm build step` runs, printing `launch build:a`, and then `clm build launched build:a t-1`
- THEN `launched` exits 1, its stderr names `t-1` as the task of the run of `build:a` the loop finished and says the printed `launch` is a new run, and the loop file lists `build:a` as planned with no task ID; `clm build launched build:a t-2` then records it running with `t-2`

#### Scenario: a rebuilt run's old task
- GIVEN `build:a` with task ID `t-1`, finished and rebuilt after a `context` stop, its `launch` printed again
- WHEN `clm build launched build:a t-1` runs
- THEN it exits 1 and the loop file is unchanged

#### Scenario: a relaunched orphan's old task
- GIVEN a loop whose lease lapsed, listing `build:a` as running with task ID `t-1` and a live claim `origin/impl/a`
- WHEN `clm build start --session-model opus` runs, printing `launch build:a`, and then `clm build launched build:a t-1`
- THEN it exits 1, and the loop file lists `build:a` as planned with no task ID

#### Scenario: another run's task
- GIVEN a running `build:b` with task ID `t-1`, and a planned `build:a`
- WHEN `clm build launched build:a t-1` runs
- THEN it exits 1 naming `build:b`, and the loop file is unchanged
