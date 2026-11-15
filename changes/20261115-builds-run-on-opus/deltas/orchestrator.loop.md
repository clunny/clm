---
component: orchestrator.loop
---

## MODIFIED Requirements

### Requirement: step-actions <!-- base: c4f86a8014302dc1 -->
`clm build step` SHALL fetch `origin`, do the housekeeping (`step-housekeeping`), read the snapshot, compute the plan, prepare each run the plan launches, record it as `planned`, store the plan, and print the actions, one per line, in the forms the Contract gives, the `launch` lines in the plan's order. The prompt file a `launch` names SHALL be the loop's own, under `.clm/prompts/loop/`, holding the line `loop run <name>` first and then, unchanged, the launcher `clm run`, `clm draft` or `clm draft --fold` printed for it, or the archive prompt (`plan-archive`), so the panel finds which entry a subagent belongs to (`plugin.panel/build-rows`). A run still `planned` SHALL be printed again unchanged by every step until `clm build launched` reports it, and SHALL NOT be prepared again. When the loop has nothing to launch, resume or stop and no `clm build wait` of the holder is running, it SHALL print `wait`. Every `clm build` command SHALL renew the lease.

The build of this requirement updates the capability test `capability/build-from-spec` (`crates/clm/tests/capability/build_from_spec.rs`) to drain its queue through `clm build start`, `step`, `launched` and `returned` rather than `clm next` and `clm run`, keeping its citation; the test of the new scenario *resume in a new session* is `capability`'s own build's (`capability-tests-follow-rules`).

#### Scenario: an unreported launch
- GIVEN a step that printed `launch build:a build <model> <file>`, and no `launched` for it
- WHEN `clm build step` runs again
- THEN it prints the same line, and `clm run a` ran once

#### Scenario: launched
- GIVEN a planned run `build:a`
- WHEN `clm build launched build:a t-17` runs
- THEN the loop file lists `build:a` as running with task ID `t-17`

#### Scenario: the launcher names the run
- GIVEN a step that launches `build:a`
- WHEN its prompt file is read
- THEN its first line is `loop run build:a` and the rest is what `clm run a` printed

#### Needs
- view/queue-view

### Requirement: plan-fold <!-- base: b2d7f17cb4add846 -->
Once the fold list isn't empty and no draft of the loop is running, the plan SHALL launch one fold, `fold:<first-id>`, with the prompt `clm draft --fold` writes for the fold list's IDs in the order their `group` lines came, on the build model, taking no place, also while a drain is requested. Its `proposed <batch> review <url>` SHALL put the batch in the human's queue. After `clm build approved <batch>`, the plan SHALL print `run land:<batch> clm land spec/<batch>`; a land that reports `gate/approval` SHALL put the batch back in the human's queue, and a land that lands SHALL be followed by `clm draft --finish` for each folded draft.

#### Scenario: fold while builds run
- GIVEN drafts `x` and `y` returned `proposed x review <url>` and `proposed y review <url>`, no draft running, and `build:a` running
- WHEN `clm build step` runs
- THEN it prints a `launch fold:x fold <model> <prompt-file>` for `x` then `y`, `<model>` the build model `agent-harness/abilities` gives

#### Scenario: approved batch
- GIVEN a fold batch `b` in the human's queue
- WHEN `clm build approved b` runs
- THEN it prints `run land:b clm land spec/b`

#### Needs
- view/queue-view

### Requirement: plan-archive <!-- base: 9750158238301773 -->
When `clm wrap-up` names complete changes, no `impl/archive/*` branch is on `origin` that a run of the loop doesn't hold, and neither a drain nor a stop is requested, the plan SHALL launch `archive:<id>`, `<id>` the first change it names, on the build model, taking no place, with a prompt that names `impl/archive/<id>` cut from `origin/main`.

#### Scenario: complete change
- GIVEN `clm wrap-up` printing `complete c`, and no `impl/archive/*` on `origin`
- WHEN `clm build step` runs
- THEN it prints `launch archive:c archive <model> <prompt-file>`, `<model>` the build model `agent-harness/abilities` gives,, whose prompt names `git switch -c impl/archive/c origin/main`

#### Needs
- view/queue-view
