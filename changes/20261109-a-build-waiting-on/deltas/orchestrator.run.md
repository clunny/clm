---
component: orchestrator.run
---

## ADDED Requirements
### Requirement: bounded-test-runs
The `clm run` prompt SHALL tell the session that every test or check command it runs itself, other than `clm land` and, when `clm.toml` sets `[land] check`, `clm land --check-only`, which it runs as written, ends within a time limit (`bounded-test-runs`): it runs the command in the foreground, where the tool call's own timeout ends it, or, when it runs it in the background, as `timeout <limit> <command>`, with a limit well past the time the run should take. It SHALL give the reason: a test that hangs, such as a polling wait with no deadline of its own, never ends a run started in the background, so its completion notice never comes, and nothing wakes a session that waits for it, nor the orchestrating session, which counts a waiting build as still running, so the build sits idle with no one told. It SHALL tell the session that a run its limit ends counts as a failing run, whose hanging test it finds as for any failure: it fixes the test when it is in its component's files, and otherwise records it as a backlog item and names that item in its pull request body. These instructions SHALL be in every prompt, fresh or resuming, in the paragraph that tells the session to end its final message with `landed <sha>` or `stopped: <reason>`, and the text SHALL keep to the wording the component's other requirements allow (it names no `cd`, exit code or `clm watch`, and names `clm land --check-only` only when `clm.toml` sets `[land] check`).

#### Scenario: prompt
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`, and once resuming a branch with unlanded commits
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `timeout <limit> <command>`, `foreground`, `hangs`, `bounded-test-runs` and `backlog item`

#### Serves
- capability/build-from-spec
