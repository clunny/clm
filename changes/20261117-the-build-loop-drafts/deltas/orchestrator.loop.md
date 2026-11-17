---
component: orchestrator.loop
base: {}
---

## MODIFIED Requirements

### Requirement: plan-order <!-- base: d3316d65daf711f6 -->
The plan SHALL launch, up to the clone's free places (`max_builds` less the places that count, as `orchestrator`'s `places` reads them), in one list sorted by priority (`P1` first): each component `clm next` prints, at the priority `clm view flight --at origin/main --format json` gives it, and each `group` line `clm triage` prints none of whose items a draft of the loop left, at the line's priority, save a draft that waits for a build of its component (`draft-waits-for-its-build`); at equal priority a draft SHALL come before a build, and otherwise each keeps its own command's order. A build SHALL launch on the build model `clm agent-harness` names, with `--review-model` the session's model, and a draft on the session's model (`model-per-launch`). It SHALL launch no new build or draft while a drain or a stop is requested. It SHALL also hold new builds and drafts back on the disk, as `clm machine --format json` read in the step's snapshot gives it: no more of them in a step than its `disk.room` when that is a number; none while `disk.room` is `null` and `disk.state` is `critical`; and none while a `disk` condition holds. A `clm machine` that exits non-zero or prints output that doesn't parse, or a `disk.state` of `unknown`, SHALL hold nothing back. An orphan's relaunch SHALL NOT be held back on the disk. The disk SHALL hold back only the entries the free places would launch, the room counting new launches alone: an entry beyond the free places SHALL keep the reason `waits for a place`, one the places would launch but the disk holds back SHALL be listed in the plan's next section with the reason `waits for disk`, and while stopped runs are being resumed one at a time every entry SHALL keep the reason `waits for stopped runs to resume`. When `clm run` or `clm draft` exits 3 because another session took the place meanwhile, the step SHALL launch nothing more that step and print no action for that run.

#### Scenario: a P1 build before P4 drafts
- GIVEN `max_builds` 3, three `group` lines at `P4`, and component `a` ready at `P1`
- WHEN `clm build step` runs
- THEN its first `launch` is `build:a`, followed by two drafts

#### Scenario: a draft before a build at equal priority
- GIVEN `max_builds` 2, component `a` ready at `P3` and a `group` line at `P3`
- WHEN `clm build step` runs
- THEN it launches the draft first and then `build:a`

#### Scenario: an item a draft left
- GIVEN a draft of the loop that returned `left b: needs the human`, and `clm triage` printing a `group` line holding `b`
- WHEN `clm build step` runs
- THEN it launches no draft of that line

#### Scenario: room for one build
- GIVEN `max_builds` 3, no places held, components `a`, `b` and `c` ready at `P2`, and `clm machine --format json` giving `disk.room` 1 and `disk.state` `warn`
- WHEN `clm build step` runs
- THEN it prints `launch build:a` and no other `launch`, and the stored plan lists `build:b` and `build:c` with `waits for disk`

#### Scenario: a disk stop holds new builds
- GIVEN `max_builds` 3, `build:b` stopped with cause `disk`, component `a` ready, and `clm machine --format json` giving `disk.room` 2
- WHEN `clm build step` runs
- THEN it prints no `launch build:a`, and the stored plan lists `build:a` with `waits for disk`

#### Scenario: no measurement yet
- GIVEN `max_builds` 2, component `a` ready, and `clm machine --format json` giving `disk.room` `null`
- WHEN `clm build step` runs with `disk.state` `critical`, and again in a fresh fixture with `disk.state` `warn`
- THEN the first prints no `launch`, and the second prints `launch build:a`

#### Scenario: no reading
- GIVEN `max_builds` 2, component `a` ready, and a `clm machine` that exits 1
- WHEN `clm build step` runs
- THEN it prints `launch build:a`

#### Needs
- view/queue-view
- machine/disk-room

## ADDED Requirements

### Requirement: draft-waits-for-its-build
The plan SHALL NOT launch the draft of a `group` line any of whose `<components>` is the component of a build the loop holds, a run `build:<component>` in any state but `returned` (an orphan and a paused run included), or of a build the same plan lists in its next section. It SHALL list that draft in the next section with the reason `waits for build:<component>`, naming the first such component in the line's order, except while stopped runs are being resumed one at a time, when it keeps `waits for stopped runs to resume` (`plan-order`); and the draft SHALL take none of the free places and none of the disk's room, so the entries after it are launched as though it weren't listed. Once no such build is held or listed, the plan SHALL launch the draft as `plan-order` launches any other. A `group` line with no components SHALL never wait so.

#### Scenario: a build and a bug draft of one component
- GIVEN `max_builds` 2, no places held, component `a` ready at `P3`, and `clm triage` printing `group bug:a [a] P1 0 x` and `group bug:b [b] P4 0 y`
- WHEN `clm build step` runs
- THEN it prints `launch build:a` and one `launch draft:`, for `y`, and the stored plan lists `draft:bug:a` with `waits for build:a`

#### Scenario: a draft waits until the build lands
- GIVEN `max_builds` 3, a running `build:a` with a claim `origin/impl/a` that `origin/main` lacks, and `clm triage` printing `group bug:a [a] P5 0 x`
- WHEN `clm build step` runs, and again after `clm build returned build:a --result 'landed <sha>'` finishes `build:a` with nothing of `a` ready
- THEN the first prints no `launch` and lists `draft:bug:a` with `waits for build:a`, and the second prints a `launch draft:` for `x`

#### Needs
- view/queue-view
