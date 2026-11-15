---
status: done
resolves:
  - the-project-rationale-still-says-builds-run-on-sonnet
---

## Intent

`docs/the-clm-project.md`, the page `AGENTS.md` names as the reasoning behind CLM, states the model policy as `model-per-launch` now holds it: a build runs on the build model `agent-harness` names, `opus` under Claude Code since `20261115-builds-run-on-opus`, and the cheaper model is for the other well-specified work (`backlog/the-project-rationale-still-says-builds-run-on-sonnet`).

On the backlog, the change deletes that item. It has no delta: no requirement, scenario or Design changes.

## Problem

Under **The right model for the task**, the page listed "building a pending rule test-first" as work for "a faster, cheaper model (Sonnet, or Haiku for mechanical steps)", and said a build "that keeps failing on the cheaper model steps up". `20261115-builds-run-on-opus` set Claude Code's build model to `opus` and amended `docs/decisions/model-per-launch.md`, but left this page as it was. `model-per-launch` also says nothing runs on `haiku`. A reader took the wrong build policy from the page every session is told to read first.

## Approaches

- **Reword the two sentences to match `model-per-launch` (chosen).** A hand edit of the page; the design isn't wrong, only the page is behind it.
- **Remove the model names from the page and point to the decision.** Rejected: the page is the reasoning a human reads without opening decisions, and the other bullet names its model too.

## Outcome

- The page's second bullet under **The right model for the task** says builds run on the build model the harness names (`opus` under Claude Code today, set by the human on 2026-11-15, with no measurement behind it yet), and that a cheaper model (Sonnet) is for the other well-specified work CLM launches: the subagents a build launches, the builder check, spec slimming and per-component drafting from code. It names no Haiku, since nothing runs on it.
- The paragraph after it says a build that keeps failing steps up to the main session's model, with no "cheaper model".
- `backlog/the-project-rationale-still-says-builds-run-on-sonnet.md` is deleted.

## Constraints

- `model-per-launch` is the source: the page restates it and adds no policy.
- The appendix's capability 7 ("a cheaper one for well-specified building") is left as written: the appendix is headed a first draft of the capabilities and is a record of that draft, as its "new, not built" notes are.

## Edge cases and workarounds

None.
