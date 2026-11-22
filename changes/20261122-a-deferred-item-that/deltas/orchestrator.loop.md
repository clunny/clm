---
component: orchestrator.loop
base: {}
---

## MODIFIED Requirements

### Requirement: humans-queue <!-- base: b869c258d549c19e -->
The plan SHALL hold the human's queue, each entry with what the human does about it and since when, oldest first: the fold batches waiting for approval (`say "approve <batch>"`); the `question` and `human-task` items on `origin/main` that block requirements, with what they block (`answer backlog/<slug>`), since the commit that added them; the `deferred` items on `origin/main` that block requirements, with what they block (`judge with /clm:triage`), since the commit that added them, since no run of the loop takes a deferred item and only the human judges its condition (`deferred-work-resolves-in-code`); the escalations the loop couldn't land; the red results of the loop's verify runs no item has taken in (`take in with /clm:triage`); the work `clm wrap-up` reports isn't safe and its ledger findings; the migrations `clm upgrade` lists (`run /clm:upgrade`); the conditions only the human can clear (`free disk, then say so`); each run the human paused (`pause`), since it was paused (`say "go on with <name>"`); and, when the disk holds back ready work (`plan-order`) while no run of the loop, of any kind, is planned or running, so no land will free space, since the step's time, the disk (`free disk; the loop goes on once a build fits`). It SHALL NOT call the host, so intake isn't in it.

#### Scenario: escalated build in the queue
- GIVEN an item `backlog/a-stops-on-work.md` the loop filed, blocking `a/x`
- WHEN the plan is computed
- THEN the human's queue lists `a-stops-on-work` with `answer backlog/a-stops-on-work` and `a/x`

#### Scenario: a deferred item that blocks a requirement
- GIVEN a `kind: deferred` item `backlog/move-x-first.md` on `origin/main` blocking `a/x`, and a `kind: deferred` item `backlog/later.md` that blocks nothing
- WHEN the plan is computed
- THEN the human's queue lists `move-x-first` with `judge with /clm:triage` and `a/x`, and doesn't list `later`

#### Scenario: the disk holds work with nothing running
- GIVEN component `a` ready, no run of the loop planned or running, and a snapshot whose disk gives `room` 0
- WHEN the plan is computed
- THEN the human's queue lists the disk with `free disk; the loop goes on once a build fits`, and with `build:b` running instead it doesn't

#### Needs
- view/queue-view
- machine/disk-room
