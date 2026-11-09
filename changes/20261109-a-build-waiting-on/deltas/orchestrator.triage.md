---
component: orchestrator.triage
---

## ADDED Requirements
### Requirement: bounded-test-runs
The `clm draft` prompt SHALL tell the session that every test or check command it runs itself, such as the checks of the code-fix path, other than `clm land` in any form, which it runs as written, ends within a time limit (`bounded-test-runs`): it runs the command in the foreground, where the tool call's own timeout ends it, or, when it runs it in the background, as `timeout <limit> <command>`, with a limit well past the time the run should take. It SHALL give the reason: a test that hangs, such as a polling wait with no deadline of its own, never ends a run started in the background, so its completion notice never comes, and nothing wakes a session that waits for it, nor the triage session, which counts a waiting draft as still running, so the draft sits idle with no one told. It SHALL tell the session that a run its limit ends counts as a failing run, whose hanging test it finds as for any failure: it fixes the test when it is in the files of the component it is fixing, and otherwise records it as a backlog item and names that item in its pull request body. These instructions SHALL be in every prompt, in the paragraph that tells the session to end its final message with `landed <sha>`, `proposed <id> review` or `stopped: <reason>`.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft` runs
- THEN the paragraph of the prompt it writes that contains `stopped: <reason>` also contains `timeout <limit> <command>`, `foreground`, `hangs`, `bounded-test-runs` and `backlog item`

#### Serves
- capability/triage-unattended
