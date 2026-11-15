---
title: "The project rationale still says builds run on a cheaper model, though builds now launch on opus"
kind: bug
components: [clm]
---

`docs/the-clm-project.md`, under **The right model for the task**, lists "building a pending rule test-first" as work for "a faster, cheaper model (Sonnet, or Haiku for mechanical steps)". Change `20261115-builds-run-on-opus` changed `agent-harness/abilities` so that Claude Code's `build_model` is `opus`, and amended `docs/decisions/model-per-launch.md`, but left this page unchanged. `AGENTS.md` names this page as the reasoning behind CLM, so a reader takes the wrong build policy from it. A docs-only edit fixes it (rewording the bullet to match `model-per-launch`), landing on checks.

Found by `/code-review` on the `impl/clm` build that re-cited `clm/agent-harness-command`.
