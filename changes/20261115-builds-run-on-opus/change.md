---
status: applied
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
- Nothing else changes: the tests of `orchestrator.loop` and the root that named `sonnet` read the value from the adapter since `4ce05672`, and the capability test for `right-model-per-task` asserts that a build model is named, not which.

## What stays on `sonnet`

The prompts `clm run`, `clm draft` and `clm draft --fold` write name `sonnet` as a literal for the subagents a build or a draft launches in turn, the builder check among them, and the plugin's three auditor agents name it in their frontmatter. Those are source text no requirement states (`prompt-text-is-source`); the backlog item lists them, so the work that makes the model a choice takes them too.

## Constraints

- No migration: no file `clm init` writes, no settings key and no format changes.
- `right-model-per-task` holds as written: CLM still names the model builds run on.
