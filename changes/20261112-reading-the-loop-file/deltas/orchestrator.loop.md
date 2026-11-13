---
component: orchestrator.loop
---

## MODIFIED Requirements
### Requirement: plan-snapshot <!-- base: 675c1a3ae9038a80 -->
`clm-loop` SHALL offer `plan(snapshot)`, a pure function returning `view`'s queue model with the sections the view shows, in the order and with the reasons the steps act on, and `stored_plan(root)`, which reads the plan the last step stored and the runs as the loop file holds them now, without fetching and without creating or writing any file or directory, or none when the clone has no loop file. Only a command that writes the loop file SHALL create `<git common dir>/clm/`. `clm build step` SHALL act on, and store, `plan` of the snapshot it read after its fetch.

#### Scenario: view and step agree
- GIVEN a snapshot with `a` ready at `P2` and a `group` line at `P4`, `max_builds` 1
- WHEN `plan` is computed and `clm build step` runs on the same snapshot
- THEN the plan's next section starts with `build:a`, the step's first `launch` is `build:a`, and `stored_plan` returns that plan

#### Scenario: no loop has run
- GIVEN a clone with no `<git common dir>/clm/` directory
- WHEN `stored_plan` reads it
- THEN it returns none, and `<git common dir>/clm/` still doesn't exist

#### Needs
- view/queue-view
