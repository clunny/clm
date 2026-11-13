---
component: view.repo
base: { frontmatter: "f2a4cbee3816c75f" }
---

## MODIFIED Frontmatter
`clm view queue` reads the plan through `orchestrator.loop`'s `snapshot` and `plan`, so this crate depends on it (`view` below `orchestrator.loop` below `view.repo`).

```yaml
depends_on: [view, model, check, orchestrator, orchestrator.loop, vcs]
```

## ADDED Requirements
### Requirement: view-queue
`clm view queue [--at <rev>]` SHALL render `orchestrator.loop`'s `plan` of the snapshot `orchestrator.loop`'s `snapshot` reads from the clone (the local refs, the places and the loop file), never fetching, the queue and the backlog read from the working tree or, with `--at <rev>`, from that commit's tree, with the checkout line as the other views have it (`views-name-their-commit`). With no loop file it SHALL render the plan of a loop that isn't held, whose `next up` is what a step would launch.

#### Scenario: no loop yet
- GIVEN a clone with no `<git common dir>/clm/loop`, component `a` ready and `max_builds` 3
- WHEN `clm view queue` runs
- THEN it exits 0, shows `loop not held`, lists `build:a` under `next up`, and writes no file

#### Scenario: same plan as the loop
- GIVEN a loop running `build:a`, and a fold batch `x` waiting for approval
- WHEN `clm view queue --at origin/main --format json` runs
- THEN its `running` holds `build:a` and its `waiting_on_you` holds `fold:x`

#### Serves
- capability/watch-beside-chat
- capability/build-from-spec

#### Needs
- orchestrator.loop/plan-snapshot
- view/queue-view
