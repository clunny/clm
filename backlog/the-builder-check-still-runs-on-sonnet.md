---
title: "The builder check still runs on sonnet, though builds run on opus"
kind: deferred
components: [plugin, orchestrator.triage]
origin: 20261115-commands-in-their-crates
---

The human set the builder check to the build model on 2026-11-15 (`model-per-launch`): it reads a change as the session that will build it, and builds run on `opus`. Three places still launch it with `model: "sonnet"`:

- `plugin/skills/propose/SKILL.md`, step 9: "Launch a fresh subagent with the `Agent` tool (general-purpose, `model: \"sonnet\"`, `model-per-launch`)". This is `plugin`'s.
- `crates/clm-triage/src/draft.rs`, the draft's prompt: "`model` set to `sonnet` (`model: \"sonnet\"`), the model builds run on". This is `orchestrator.triage`'s.
- `crates/clm-triage/src/fold.rs`, the fold's prompt, in two places. This is `orchestrator.triage`'s.

Each names `opus` instead, or the build model `clm agent-harness` prints as `build-model`, so that the value has one source. The `architecture-auditor` launches beside them are unchanged. A test that asserts the old text follows its prompt. One component at a time, each under its own `clm claim <component> --fix`, the last deleting this item.

Due now, an exception to the freeze by the human. It changes nothing a requirement states.
