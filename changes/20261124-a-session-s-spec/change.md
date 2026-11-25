---
status: applied
resolves: [a-session-s-spec-auditor-on-a-code-fix-runs-on-the-agent-file-s]
---

## Intent

A session that fixes code itself launches the `spec-auditor` on its own model, as a build's final review does. `AGENTS.md`'s Prove step names the model, the decision `model-per-launch` gains the launch in its table, and the bug `a-session-s-spec-auditor-on-a-code-fix-runs-on-the-agent-file-s` is deleted. No requirement, scenario or Design changes, and nothing is built.

## Problem

A code fix a session makes itself (`clm claim <component> --fix`, or a small code change) is reviewed before landing as `AGENTS.md`'s Prove step says: `/code-review` and the `spec-auditor` subagent. The step named no model, so the auditor ran on its agent file's `model: sonnet`, while the same review in a build runs on the model `clm run --review-model` writes into the prompt, the session's own (`orchestrator.run`). `model-per-launch` lists every launch with its model and had no line for this one.

## Approaches

- **Name the model where the session is told to launch the auditor (taken).** The only such place outside a `clm run` prompt is this repository's `AGENTS.md`, Prove step: it now says to set `model` to the model a `clm run` prompt names or, in a session's own fix, to the session's own model's alias. The decision's table gains the row "a session's `spec-auditor` on its own code fix", on the session's own model, beside the build's final review, with a dated paragraph.
- **Edit the `propose` and `contribution` skills too (rejected).** The item names both, but neither launches the auditor itself: the `propose` skill's Code fixes runs "the repository's checks and reviews as `CLAUDE.md` asks", which in this repository is `AGENTS.md`'s Prove step, now naming the model; the `contribution` skill takes in intent only and launches only the `architecture-auditor`, whose model it already names. The `AGENTS.md` `clm init` writes for an adopter asks for no `spec-auditor`, so no adopter session is told to launch one outside a build. Restating the model in the skills would hold the rule twice (`reuse-first`).
- **Change the agent file's `model: sonnet` (rejected).** `plugin.agents` states it, and it is the model for launches that name none; the fault was the launch naming none, which `model-per-launch` forbids.

## Outcome

`AGENTS.md` and `docs/decisions/model-per-launch.md` are edited by hand, and the item is deleted. Nothing in `specs/` changes, so no component becomes pending.

## Constraints

- `model-per-launch`: every launch names its model; a review runs on the session's own model.
- `prompt-text-is-source`: no requirement states the instruction's wording.
- `reuse-first`, `yagni`: the rule is stated once, where the launch is asked for.
