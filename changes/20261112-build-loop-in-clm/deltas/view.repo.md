---
component: view.repo
base: { frontmatter: "f2a4cbee3816c75f" }
---

## MODIFIED Frontmatter
`clm view queue` reads the plan through `orchestrator.loop`'s `stored_plan`, so this crate depends on it (`view` below `orchestrator.loop` below `view.repo`).

```yaml
depends_on: [view, model, check, orchestrator, orchestrator.loop, vcs]
```

## ADDED Requirements
### Requirement: view-queue
`clm view queue` SHALL render the plan `orchestrator.loop`'s `stored_plan` reads, never fetching and writing no file, with the checkout line as the other views have it (`views-name-their-commit`) and the time and `origin/main` commit the plan was computed at. With no loop file it SHALL render a loop that isn't held, whose next section lists the ready, unclaimed components of the queue (`queue_from_refs`) read from the working tree, or with `--at <rev>` from that commit's tree, in priority order, and no drafts, saying that no loop has run on this clone; with a loop file, `--at` SHALL change only the checkout line.

#### Scenario: no loop yet
- GIVEN a clone with no `<git common dir>/clm/loop` and component `a` ready
- WHEN `clm view queue` runs
- THEN it exits 0, shows `loop not held`, lists `build:a` under `next up`, and writes no file

#### Scenario: the loop's plan
- GIVEN a loop running `build:a`, whose stored plan holds a fold batch `x` waiting for approval
- WHEN `clm view queue --at origin/main --format json` runs
- THEN its `running` holds `build:a` and its `waiting_on_you` holds `fold:x`

#### Serves
- capability/watch-beside-chat
- capability/build-from-spec

#### Needs
- orchestrator.loop/plan-snapshot
- view/queue-view
- view/view-command
