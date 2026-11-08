---
component: orchestrator.triage
---

## MODIFIED Requirements
### Requirement: background-work-stopped <!-- base: 7151c40d65b4e6b4 -->
The `clm draft` prompt SHALL tell the session to wait for a command it runs in the background, such as `clm land spec/<id>` or the checks of the code-fix path, through that command's own completion notice, and never through a loop, watch or monitor that matches processes by name (such as `pgrep -f`), since such a match also finds other drafts' and builds' processes and outlives the command it waits for; running a command in the background is a setting of the tool call, not a change to the command. It SHALL tell the session that a `clm land` it started is waited for until its completion notice arrives, however long that takes, since lands queue behind other drafts' and builds' lands and can take many minutes, and that it never writes its final message while one is still running, since the triage session reads the final line as the draft's end and then frees its place and removes its worktree. It SHALL tell the session that when its turn ends while a command it started in the background still runs, because waiting for that command's completion notice is all that is left, the message ending that turn is not its final message, since the triage session may receive it as the draft's result: it carries no closing line, neither `landed <sha>`, `proposed <id> review` nor `stopped: <reason>`, and ends with the line `waiting: <command>`, naming the command it waits for; the final message comes in a later turn, once the command has ended. It SHALL tell the session that it hands back one report, its final message (`one-report-per-run`): a tool that delivers its report to the triage session and accepts one report per run, such as Claude Code's `SubagentHandback`, is never called for the message ending such a turn, which the session ends as plain text, and is called once, with the final message, since a report sent while waiting uses up the run's one report and the final message would then never reach the triage session. It SHALL tell the session that before it writes its final message, landed, proposed or stopped, it stops every background task it started that is still running (background shells, monitors, watches) and waits for every subagent it launched, the builder check among them, to return, so that no task of a finished draft wakes the triage session later. It SHALL name no tool for stopping a task. These instructions SHALL be in every prompt `clm draft <slug>...` writes, in the paragraph that tells the session to end its final message with `landed <sha>`, `proposed <id> review` or `stopped: <reason>`, and the text SHALL keep to the wording `draft` allows (it names no `clm watch`, `exits 0` or `CARGO_TARGET_DIR`).

#### Scenario: prompt
- GIVEN untaken `bug` item `a`, once alone and once with item `b` claimed by `origin/propose/b` committed 1 hour ago
- WHEN `clm draft a` and `clm draft a b` run
- THEN in each prompt file it writes, the paragraph that contains `stopped: <reason>` also contains `completion notice`, `pgrep` and `background task`, and says to stop those tasks before the final message

#### Scenario: a turn ends while waiting
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN in the prompt file it writes, the paragraph that contains `stopped: <reason>` also contains `many minutes`, `waiting: <command>` and `not your final message`, and says that the message ending such a turn carries none of `landed <sha>`, `proposed <id> review` and `stopped: <reason>`

#### Scenario: a waiting turn sends no report
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN in the prompt file it writes, the paragraph that contains `stopped: <reason>` also contains `one report`, `SubagentHandback` and `one-report-per-run`, and says that the hand-back tool is never called for the message ending a turn that waits, and is called once, with the final message

#### Needs
- plugin/triage-waiting-drafts
