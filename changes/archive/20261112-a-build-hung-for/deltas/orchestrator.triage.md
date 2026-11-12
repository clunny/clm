---
component: orchestrator.triage
---

## MODIFIED Requirements
### Requirement: bounded-test-runs <!-- base: 9a53e9d5e4dbac51 -->
The `clm draft` prompt SHALL tell the session that every command it runs itself, such as the checks of the code-fix path, other than `clm land` in any form, which it runs as written, ends within a time limit (`bounded-test-runs`): it runs the command in the foreground, where the tool call's own timeout ends it, or, when it runs it in the background, as `timeout <limit> <command>`, with a limit well past the time the run should take, whatever the command is, a test or check run or any other, and for each command of a chain it starts in the background. It SHALL tell the session that no command it runs reads standard input that nothing feeds it, such as `cat` with no file to read or a `cat >> <file>` with no input given, since nothing ever types into a command it runs, and that it writes a file with its editing tools instead; a command given its input by a heredoc or a pipe, such as `clm defect` or `clm workaround` with the report on stdin, is fine, since its input ends. It SHALL give the reason: a command that never ends by itself, such as a test that hangs on a polling wait with no deadline of its own or a command waiting on standard input, never ends a run started in the background, so its completion notice never comes, and nothing wakes a session that waits for it, nor the triage session, which counts a waiting draft as still running, so the draft sits idle with no one told. It SHALL tell the session that a test or check run its limit ends counts as a failing run, whose hanging test it finds as for any failure: it fixes the test when it is in the files of the component it is fixing, and otherwise records it as a backlog item and names that item in its pull request body. These instructions SHALL be in every prompt, in the paragraph that tells the session to end its final message with `landed <sha>`, `proposed <id> review` or `stopped: <reason>`.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft` runs
- THEN the paragraph of the prompt it writes that contains `stopped: <reason>` also contains `timeout <limit> <command>`, `foreground`, `hangs`, `standard input`, `editing tools`, `bounded-test-runs` and `backlog item`
