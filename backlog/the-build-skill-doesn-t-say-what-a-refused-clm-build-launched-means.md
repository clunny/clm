---
title: "The build skill doesn't say what a refused clm build launched means"
kind: deferred
components: [plugin]
origin: 20261117-clm-build-step-prints
---

Once `orchestrator.loop/launched-refuses-an-earlier-task` is built, `clm build launched <name> <task-id>` exits 1 when the task ID belongs to another run or to an earlier run of the name, such as the subagent of a build that landed before it reported, whose component the loop then launched again under the same name. The `build` skill's `launch` and `run` bullets (`plugin/skills/build/SKILL.md`) say only to run `clm build launched`, and nothing about a refusal.

Add beside them, in the skill's words: a `launch` or `run` is always a new run, even when the agent harness still lists a task of the same name as running; start it, and report its own task ID. When `clm build launched` refuses a task ID, the printed action has not been started: start it and report the new task's ID. Once `orchestrator.loop/returned-ignores-an-earlier-task` is built too (`20261117-a-late-report-of`), the skill's reporting sentence ("Report every subagent's or background command's return with `clm build returned <name> --result <last line>`") also says to pass `--task <task-id>`, the ID of the task that returned, so that a late report of an earlier run of the name changes nothing. The text is prompt text, which no requirement states (`prompt-text-is-source`), so a code branch of `plugin` resolves this item with the trailer `Resolves: backlog/the-build-skill-doesn-t-say-what-a-refused-clm-build-launched-means` once both requirements are built.
