---
component: orchestrator.run
base: { contract: "7372cbf537b3b132" }
---

## MODIFIED Contract
`clm run <component>` claims a component through `orchestrator`'s `claim_local`, which records the claim in the local branch `impl/<component>` before pushing it, or picks up its unfinished build, so a launch interrupted at any point is resumed or claimed afresh by the next `clm run <component>`, never held until its claim lapses, and writes the prompt for a session that builds its pending requirements and lands them with `clm land` (`land-locally`) to `.clm/prompts/<component>.md` in the checkout it runs in. On stdout it prints a short launcher naming that file by its absolute path, which the build loop gives to a background subagent of the human's session through the `build` skill in its own Claude Code worktree (`builds-in-subagents`), and which the subagent follows by reading the file (`build-prompt-file`). It fetches `origin` first and takes `--expiry <hours>` (default 4) and `--review-model <model>`, the alias of the model the build's final review runs on, which the build loop sets to the session's model (`model-per-launch`). Each build holds one of the clone's `max_builds` places (`orchestrator`'s `build-places`) from when `clm run` starts it until it lands, is finished or its claim lapses. When it refuses a component, it says why. It exits 0 on success, 1 when the work isn't possible (with the reason), 2 when git fails, and 3 when the clone has no free place. `clm run --finish <component>`, which the build loop runs whenever a build's subagent returns, frees the component's place and tidies the worktree of a landed build, printing `landed impl/<component>` when the branch has landed (`unclosed-build-reads-finish`); a build that hasn't landed keeps its claim and its branch, pushed or not, so the next `clm run <component>` resumes it (`resume-what-push-accepts`). A build compiles into its own worktree's `target/` (`build-places`). `clm superpowers <skill>` prints the path of `skills/<skill>/SKILL.md` in the pinned superpowers copy, fetching it first if needed, so builds and interactive skills follow one version (`design-brainstorming`, `unattended-superpowers`). The pin is moved by hand, on an `impl/orchestrator.run` branch (`cut-evals`). `clm setup` runs the project's `[build] setup` (`config`) in the CLM root of the checkout it runs in, which a build runs before its first test run so that its tests run against its own worktree's code and dependencies (`tests-run-in-their-own-tree`, `build-setup-step`); it exits 0 when the command passes or none is set, and 1 otherwise.

A component whose pending requirements name a file Claude Code protects, such as `.claude/settings.json` or an `.npmrc`, is built like any other: Claude Code asks the human to confirm each such write (`builds-in-subagents`).

## MODIFIED Requirements
### Requirement: background-work-stopped <!-- base: 2b51ea062d384a2a -->
The `clm run` prompt SHALL tell the session to wait for a command it runs in the background, such as `clm land --check-only`, through that command's own completion notice, and never through a loop, watch or monitor that matches processes by name (such as `pgrep -f`), since such a match also finds other builds' processes and outlives the command it waits for; running a command in the background is a setting of the tool call, not a change to the command, so it doesn't break the rule of running the check as one plain command. It SHALL tell the session that a `clm land` it started, and when `clm.toml` sets `[land] check` a `clm land --check-only` it started, is waited for until its completion notice arrives, however long that takes, since lands queue behind other builds' lands and can take many minutes; that it never writes its final message, landed or stopped, while one of those is still running, and never stops one to end the build, since the orchestrating session reads the final line as the build's end and then frees its place and tidies its worktree. It SHALL tell the session that when its turn ends while a command it started in the background still runs, because waiting for that command's completion notice is all that is left, the message ending that turn is not its final message, since the orchestrating session may receive it as the build's result: it carries no closing line, neither `landed <sha>` nor `stopped: <reason>`, and ends with the line `waiting: <command>`, naming the command it waits for; the final message comes in a later turn, once the command has ended. It SHALL tell the session that it hands back one report, its final message (`one-report-per-run`): a tool that delivers its report to the orchestrating session and accepts one report per run, such as Claude Code's `SubagentHandback`, is never called for the message ending such a turn, which the session ends as plain text, and is called once, with the final message, since a report sent while waiting uses up the run's one report and the final message would then never reach the orchestrating session. It SHALL tell the session that before it writes its final message it stops every other background task it started that is still running (background shells, monitors, watches) and waits for every subagent it launched to return, so that no task of a finished build wakes the orchestrating session later. It SHALL name no tool for stopping a task. These instructions SHALL be in every prompt, in the paragraph that tells the session to end its final message with `landed <sha>` or `stopped: <reason>`, and the text SHALL keep to the wording the component's other requirements allow (it names no `cd`, exit code or `clm watch`, and names `clm land --check-only` only when `clm.toml` sets `[land] check`).

#### Scenario: prompt
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`, and once resuming a branch with unlanded commits
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `completion notice`, `pgrep` and `background task`, and says to stop those tasks before the final message

#### Scenario: a land still running
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `clm land`, `queue`, `many minutes` and `never stop one`, and says never to write the final message while a `clm land` the session started is still running

#### Scenario: a turn ends while waiting
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `waiting: <command>` and `not your final message`, and says that the message ending such a turn carries neither `landed <sha>` nor `stopped: <reason>`

#### Scenario: a waiting turn sends no report
- GIVEN a ready component `b`, with and without a `[land] check` in `clm.toml`
- WHEN `clm run b` runs
- THEN in each prompt it writes, the paragraph that contains `stopped: <reason>` also contains `one report`, `SubagentHandback` and `one-report-per-run`, and says that the hand-back tool is never called for the message ending a turn that waits, and is called once, with the final message

#### Serves
- capability/work-concurrently

#### Needs
- orchestrator.loop/return-rules
