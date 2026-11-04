---
title: "The clm plugin's skills and hooks have eval cases, run by claude plugin eval at the lands that can have changed what they exercise"
date: 2026-10-28
status: superseded
superseded_by: cut-evals
---

Extends `continuous-evals-at-land`, whose cases load superpowers alone: nothing ran the `clm` plugin's skills (`propose`, `implement`, `triage`, `init`, `health`) or its hooks, so an edit to a skill's text landed on its tests' word lists alone, which prove that a sentence is there, not that a session follows it.

**The runner.** Claude Code's `claude plugin eval` (`reuse-first`): it loads only the plugin under test, with its hooks, skills and agents, runs each case's prompt as a fresh `claude -p` child in a temporary working directory and `HOME`, scaffolds the workspace with a script the case names, and scores the run with graders. It repeats runs, takes a cost ceiling (`--max-cost-usd`, checked before each run, exit 2 when hit), a threshold (exit 1 below it) and tag filters, and writes `result.json` and a self-contained `report.html`. The cases are `plugin/evals/<case>/`, beside `superpowers-unattended`, which stays a plain script because its plugin under test is superpowers, not ours.

**What decides.** Only the runner's deterministic graders: `tool_used`, `tool_order`, `file_exists` and `regex` over the last message, the trace or a file's contents (`deterministic-enforcement`). The `llm` and `baseline` graders are a model judging a model and are never used, which a test of the case files enforces. What a case can observe is what the model did and what it left behind: the commands it ran and their order, the files in the workspace, its last message. A hook's block is not in that list, so a hook case observes the block's effect: the scaffolded bare `origin` gets an `update` hook that writes a file when `main` is pushed, and the grader checks the file was never created.

**When.** At land, from the check script, when the selection names a file under `plugin/` other than the superpowers cases: the skills, hooks, shim, agents, command, manifest and the cases themselves are the only inputs the runs have, and every one of them reaches the sessions. The run is filtered to what changed, by tags: a skill's cases for its `SKILL.md`, one case for its own directory, every case for the hooks, the shim or anything else all cases share. A run with no selection never runs them.

**Budget.** One run per case (`--runs 1`) at the default threshold, no baseline arm (`--ablation none`), and a hard ceiling per land that the human sets as the script's `skill_eval_budget_usd` (`backlog/skill-eval-budget`, a `human` item that blocks the requirement until they do). The cases are small: a scaffolded repository with one node and one item, tasks bounded by `max_turns` and `timeout_seconds` in each case, and the two that launch a subagent (`implement`, `triage`) are given fixtures whose nested work is short. Runs use the human's own Claude Code login, as `continuous-evals-at-land` does; no API key is stored.

**Chance failures.** One run per case means a stochastic failure fails a land. The report names the case and the grader, the output directory is kept, and landing again runs it again; a failure that repeats is a defect in the skill or the case, filed with `clm defect`. Three runs with a lower threshold would cost three times as much to hide the same information.

**No model pin, and no scheduled rerun.** The runner's docs suggest pinning `--model` so a model rollout isn't read as a plugin regression. Here the cases run on Claude Code's default model, the one the skills run on in every session, because a skill that stops working on that model is broken for its users whatever the cause. The item asked whether a scheduled job should rerun the cases with unchanged inputs to catch such a rollout: no. Files under `plugin/` land several times a week in this repository, so the next such land runs the cases on the new model within days, and the failure's report says which model ran; a scheduled job would need its own trigger, place and cost for the same signal (`continuous-evals-at-land` rejected it for the superpowers cases for the same reason). If plugin lands become rare, that changes, and the job belongs beside `clm superpowers upgrade`'s schedule.

**Why a wrapper.** `plugin/evals/skills.sh` runs the runner with the fixed flags. The cases' build, and the check script's, run as worktree-isolated subagents, where Claude Code refuses any Bash command holding the runner's subcommand as a bare word (`ci`'s `eval-at-land` records the same for `clm superpowers eval`); a script path holds no such word, so a build can run the cases to see them pass, and the check script's tests fake one file instead of Claude Code.

Rejected:
- **Extending `superpowers-unattended`'s `run.sh`** with arms that load the `clm` plugin: a second home-made runner and judge, when Claude Code ships one with the graders, the repeats and the ceiling (`reuse-first`).
- **Every case at every plugin land:** pays for `triage` when `health` changed. Tags cost nothing.
- **A model-judged "did it follow the skill" grader:** advises at best, never gates.
- **Pinning `--model`:** keeps lands green through a rollout the users feel.
