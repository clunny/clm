---
title: "Every agent CLM launches names its model: builds on opus, other mechanical work on sonnet, design and the final review on the main session's"
date: 2026-10-30
status: accepted
---

Serves `capability/right-model-per-task` and applies the re-founding's "the right model for the task" (`docs/the-clm-project.md`, Restraint), which reuses superpowers' Model Selection rules (`subagent-driven-development`) rather than inventing a policy. An agent launched without a model inherits its launcher's, so before this every build, draft, audit and builder check ran on the human's session model unseen.

**The policy.** Each launch names its model explicitly, by the Agent tool's `model` parameter, an agent file's `model` frontmatter or `claude --model`:

| Launch | Launched by | Model |
| --- | --- | --- |
| a build | `implement` | `opus`, the build model `agent-harness` names |
| a rebuild after a build stopped (not on a backlog item or spec change) | `implement` | the session's own model (stepping up) |
| a build's `spec-auditor`, its final review | the build | the `--review-model` `implement` passes to `clm run`: the session's own model |
| a session's `spec-auditor` on its own code fix (`AGENTS.md`'s Prove step) | the session | the session's own model |
| any other subagent a build launches | the build | `sonnet` |
| a draft | `triage`, `build` | the session's own model (design) |
| the fold | `triage`, `build` | the build model |
| a builder check | `propose`, a draft, the fold | the build model, `opus` under Claude Code: it reads a change as the session that will build it |
| `architecture-auditor`, a design review (`spec-architecture-audit`) | `propose`, `contribution`, a draft, the fold | the session's own model (`sonnet` in the fold) |
| `spec-slimmer` | `health` | `sonnet` |
| per-node spec drafting from code | `init` | `sonnet` |

"The session's own model" is the main session's, whatever it is, never a fixed top tier. `sonnet` is the floor superpowers sets for implementers working from prose and for reviewers ("turn count beats token price"), and CLM's builds work from prose requirements; nothing runs on `haiku`.

**How the review model reaches a build.** The build runs on the build model, so it can't name the main session's model by inheritance. `implement` knows its own model and passes its alias with `clm run <node> --review-model <alias>`, which `clm run` writes into the prompt unchecked (the Agent tool refuses an unknown alias). Without the flag, as from an older skill, the prompt says to name the build's own model, so the review is still named, on the cheaper model.

**Edge cases accepted.**
- `/code-review`, which a build runs as a skill, launches its own reviewers as Claude Code decides; CLM names no model for them.
- A main session whose model has no alias in the Agent tool's list names the nearest alias.
- `sonnet` appears as a constant in the skills, `clm-run`'s prompt and `clm-triage`'s prompts; moving it is a code change in each, as moving the superpowers pin is.

**Rejected.** A `[models]` setting in `clm.toml` (YAGNI: no adopter asked to choose, and the main session's model can't be configured ahead); naming `opus` for reviews (a fixed top tier the principle rules out); having `implement` run the final review itself after the build returns (the build lands before it returns, and moving the review out would redesign the build flow).

**Changed on 2026-11-15** (`20261115-builds-run-on-opus`). The human found builds on `sonnet` made too many mistakes and set the build model to `opus`, which the loop also launches folds and archives on. No measurement backs it: nobody has read from the loop file or the ledger how often `sonnet` builds were rebuilt, escalated or refused at the land, which was the condition this decision first named for changing. The human also expected a model chosen for each task, not one for the class; that, and a setting, are `backlog/every-build-runs-on-one-fixed-model-whatever-the-task`. Left as they were: the subagents a build launches, the builder check and the plugin's agents still name `sonnet`, so the builder check no longer runs on the builder's model.

**The builder check follows the build model (2026-11-15, by the human, with `20261115-commands-in-their-crates`).** The check asks whether a change can be built, as the person who will build it, and builds run on `opus` since that day; the check was still launched on `sonnet`, the model builds had run on. It now runs on the build model wherever it is launched: by `propose`, by a draft and by the fold. No measurement backs it either. The code is `backlog/the-builder-check-still-runs-on-sonnet`. The subagents a build launches and the plugin's agents still name `sonnet`.

**A session's own code fix (2026-11-24, `20261124-a-session-s-spec`).** A session that fixes code itself, with `clm claim <component> --fix` or as a small code change, reviews it before landing as `AGENTS.md`'s Prove step says, with `/code-review` and the `spec-auditor`. The step named no model, so the auditor ran on its agent file's `sonnet`, a weaker model than the same review in a build. It now runs on the session's own model, as a build's final review does, and `AGENTS.md`'s Prove step says so. The `propose` skill's Code fixes runs the reviews "as `CLAUDE.md` asks" and the `contribution` skill launches no `spec-auditor`, so neither skill's text changes.
