---
title: "Every build runs on one fixed model, whatever the task"
kind: idea
components: [agent-harness, config, orchestrator.loop, orchestrator.run, orchestrator.triage, plugin]
---

## Intent

The model a build launches on should fit the work, and a project should be able to choose it. Today it is one value fixed in `agent-harness` (`build_model`, `opus` for Claude Code since `20261115-builds-run-on-opus`, `sonnet` before), the same for every build, fold and archive of every project.

## What the human said (2026-11-15)

Builds on `sonnet` made too many mistakes, so the value was changed to `opus` at once. They had expected the loop, or some layer above it, to choose the model for the specific task, not one model for the whole class of tasks; a fixed `sonnet` for builds was the wrong value. Making it configurable was asked for as this item.

## Approaches

- **A setting**, in `clm.toml` or `clm.local.toml`, naming the build model, with the adapter's value as the default. A requirement of `config`, a section of the guide's settings page, and the loop and `clm agent-harness` reading it.
- **A model for each task.** The loop picks by what the build holds: the size of the component and of what is pending, whether the component is over budget, whether an earlier round failed (a build that stops twice is already rebuilt on the session's model). It needs evidence of which builds go wrong on the cheaper model.

## Open questions

- No measurement backs the choice either way. The loop file and the ledger could show how many builds on each model were rebuilt, escalated or refused at the land; nobody has read them.
- What `opus` builds cost against `sonnet` ones (`effort-accounted` records it).
- The literal `sonnet` elsewhere, which this should take with it: the prompts written by `crates/clm-run/src/lib.rs`, `crates/clm-triage/src/draft.rs` and `crates/clm-triage/src/fold.rs` for the subagents a build or draft launches, the builder check among them, which no requirement states; and the frontmatter of `plugin/agents/architecture-auditor.md`, `spec-auditor.md` and `spec-slimmer.md`, which `plugin/launch-models` states. The builder check runs on `sonnet` while the builder runs on `opus`, against the reason `model-per-launch` gives for it.
- Whether a capability, `right-model-per-task`, should say the person can choose.
