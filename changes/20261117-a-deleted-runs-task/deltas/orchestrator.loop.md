---
component: orchestrator.loop
base: {}
---

## MODIFIED Requirements

### Requirement: launched-refuses-an-earlier-task <!-- base: 10580e691793e418 -->
`clm build launched <name> <task-id>` SHALL refuse with exit 1, changing nothing in the loop file (its lease included), when `<task-id>` is a task ID the loop file records other than the one recorded for the run now listed as `<name>`: another run's, in any state, or one kept as an earlier run's. The loop file SHALL keep the task IDs of the runs it has finished with by run name, apart from its list of runs, so that they outlast the runs: whenever the loop clears a run's task ID (a rebuild, `stop-causes`; an orphan's relaunch, `loop-start`; a task reported over another), replaces a listed run with a new run of its name (a step planning the name of a run that returned or that `claim-gone-ends-a-build` finished), or removes a run from its list by any path (`clm build start` dropping the runs that returned, `loop-start`; `clm build release`, `release`; a step dropping a fold or archive orphan for the plan to compute again, or a run whose relaunch was refused, `loop-start`), that run's task ID SHALL stay kept under its name, whether or not a run of the name is listed then or later. No step SHALL remove a kept ID; they go only with the loop file. The field SHALL be optional, so a loop file an earlier `clm` wrote loads unchanged and its `version` stays the same, and the task IDs such a file kept with a run SHALL be kept under that run's name. Its stderr SHALL name the run that task belonged to and, for an earlier run of `<name>`, say that the loop has finished that run and that a late `landed` report of it changes nothing (`return-rules`), and that the action printed for `<name>` is a new run: for a `launch`, to be started as a new subagent, and for a `run`, as a new command, each reported with its own task ID. The build of this requirement updates the capability test `capability/build-from-spec` (`crates/clm/tests/capability/build_from_spec.rs`), whose `drain_the_queue` and `resume_in_a_new_session` report one task ID for two runs, to report a distinct ID for each run, keeping its citations (`capability-tests-follow-rules`).

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

#### Scenario: a landed build dropped by a new session
- GIVEN a loop whose lease lapsed, listing `build:a` as returned and landed with task ID `t-1`, and component `a` ready again
- WHEN `clm build start --session-model opus` runs, printing `launch build:a`, and then `clm build launched build:a t-1`
- THEN `launched` exits 1, its stderr names `t-1` as the task of the run of `build:a` the loop finished, and the loop file lists `build:a` as planned with no task ID

#### Scenario: a released build's task
- GIVEN `build:a` stopped with task ID `t-1`, released with `clm build release a`, and a later step printing `launch build:a`
- WHEN `clm build launched build:a t-1` runs
- THEN it exits 1, and the loop file is unchanged

### Requirement: returned-ignores-an-earlier-task <!-- base: 15f5b84054022cfb -->
`clm build returned <name> --task <task-id> --result <line>`, whatever its `--cause`, SHALL record nothing of the return (no state, task ID, stop, condition or resume of any run), run no `clm run --finish` or `clm draft --finish` for it, print no `stop` or `resume` for it, and print the actions to take now as `clm build step` does, a planned run's `launch` printed again unchanged among them, when `<task-id>` is one the loop file keeps under `<name>` (`launched-refuses-an-earlier-task`) and isn't the task ID recorded for the run now listed as `<name>`, if one is listed; its stderr SHALL say that the return is a late report of an earlier run of `<name>`, which the loop has finished, and that it changed nothing. This SHALL hold after the refusals of an invalid `--cause` or `--until`, which stay as they are, and before every rule of `return-rules` and `stop-causes`, whatever the state of the run now listed as `<name>`, and also when the loop file lists no run of `<name>`, which `return-rules` would refuse. A return with no `--task`, or with a `<task-id>` the loop file doesn't keep as an earlier run's of `<name>`, SHALL be handled as `return-rules` handles it, so a session that doesn't pass `--task` gets what it got before.

#### Scenario: a late context stop of a landed build
- GIVEN a running `build:a` with task ID `t-2`, whose loop file keeps `t-1` as an earlier run's of `build:a`
- WHEN `clm build returned build:a --task t-1 --result none --cause context` runs
- THEN it exits 0, `clm run --finish a` didn't run, stderr names `build:a` as an earlier run's late report, no line printed names `build:a`, and the loop file lists `build:a` as running with `t-2` and no stop

#### Scenario: a late work stop before the new run is launched
- GIVEN a planned `build:a` with no task ID, whose loop file keeps `t-1` as an earlier run's of `build:a`, and the `launch build:a` line a step printed for it
- WHEN `clm build returned build:a --task t-1 --result none --cause work` runs
- THEN it prints that `launch build:a` line unchanged, and the loop file lists `build:a` as planned with no stop

#### Scenario: the run's own task
- GIVEN a running `build:a` with task ID `t-2`, whose loop file keeps `t-1` as an earlier run's of `build:a`
- WHEN `clm build returned build:a --task t-2 --result none --cause work` runs
- THEN it prints `resume build:a t-2 <message-file>`, and the loop file counts one `work` stop for `build:a`

-now-at-its-path

#### Scenario: a late report with no run of its name listed
- GIVEN a loop file that lists no `build:a` and keeps `t-1` under `build:a`, as after `clm build release a` released a stopped `build:a` with task ID `t-1`
- WHEN `clm build returned build:a --task t-1 --result none --cause work` runs
- THEN it exits 0, `clm run --finish a` didn't run, stderr names `build:a` as an earlier run's late report, and the loop file is unchanged but for the lease
