---
status: applied
human_burdens:
  - Every build, fold and archive under Claude Code runs on opus, which costs more than sonnet, for as long as the value stands
---

## Intent

Builds under Claude Code launch on `opus` instead of `sonnet`. The human finds that builds on `sonnet` make too many mistakes (2026-11-15), and asked for the value to be changed at once, with making it a choice left as a backlog idea. It is outside the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims`, whose freeze the human excepted for it that day: every remaining phase is built by these builds.

## Problem

The model a build launches on is one fixed value, `build_model` in `agent-harness`'s abilities, stated by `agent-harness/abilities` as `sonnet` for Claude Code. `clm agent-harness` prints it, and the build loop launches every build, fold and archive on it. No setting changes it, and nothing chooses a model by what the task needs: the human expected the loop to pick a model for each task, and found one value for the whole class.

## Approaches

- **Change the fixed value to `opus` (chosen, for now).** One requirement and one Design row of `agent-harness`. Quick, and it is what the human asked for. Its cost: every build costs more, and an adopter can't choose otherwise once it is released; no release is planned before the plan ends.
- **A setting for the build model.** Each project picks. It needs a requirement of `config`, the guide's settings page, and the loop reading it. Left as `backlog/every-build-runs-on-one-fixed-model-whatever-the-task`.
- **A model chosen for each task.** What the human expected. It needs a rule for what makes a task hard, which nothing measures yet. Left in the same item.

## Outcome

- `clm agent-harness` under Claude Code prints `build-model opus`, and the build loop launches builds, folds and archives on `opus`.
- A build's final review and every draft still run on the session's model, as before.
- The capability test for `right-model-per-task` asserts that a build model is named, not which, so it needs no change.

## What stays on `sonnet`

- The prompts `clm run`, `clm draft` and `clm draft --fold` write name `sonnet` as a literal for the subagents a build or a draft launches in turn, the builder check among them. No requirement states that literal (`prompt-text-is-source`).
- The plugin's three agents, `spec-auditor`, `architecture-auditor` and `spec-slimmer`, set `model: sonnet` in their frontmatter, which `plugin/launch-models` states. This change leaves that requirement alone.

So the builder check still runs on `sonnet` while the builder now runs on `opus`, and its reason in `model-per-launch`, "the model the builder will run on", no longer holds. The backlog item lists all of these, so the work that makes the model a choice takes them too.

## Scenarios that named the value

Four scenarios of other components stated `sonnet` as what the system prints for the build model: `clm/agent-harness-command`, and `orchestrator.loop`'s `step-actions`, `plan-fold` and `plan-archive`. Their requirements say "the build model", and their tests read it from the adapter since `4ce05672`. The scenarios now say `<model>`, the build model `agent-harness/abilities` gives, so they stay true whatever the value. Each requirement's build re-cites its tests and changes no code. `orchestrator.loop/loop-start`'s scenario keeps `sonnet`: there it is the model the loop file holds for a run.

## Decisions

- **`model-per-launch`** (amended): builds, folds and archives launch on `opus`; why, that no measurement backs it, and what still names `sonnet`.
- **`spec-architecture-audit`** (amended): its two sentences that called `sonnet` the build's model.

## Constraints

- No migration: no file `clm init` writes, no settings key and no format changes.
- `right-model-per-task` holds as written: CLM still names the model builds run on.
