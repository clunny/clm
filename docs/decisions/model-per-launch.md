---
title: "Every agent CLM launches names its model: builds and mechanical work on sonnet, design and the final review on the main session's"
date: 2026-10-30
status: accepted
---

Serves `capability/right-model-per-task` and applies the re-founding's "the right model for the task" (`docs/the-clm-project.md`, Restraint), which reuses superpowers' Model Selection rules (`subagent-driven-development`) rather than inventing a policy. An agent launched without a model inherits its launcher's, so before this every build, draft, audit and builder check ran on the human's session model unseen.

**The policy.** Each launch names its model explicitly, by the Agent tool's `model` parameter, an agent file's `model` frontmatter or `claude --model`:

| Launch | Launched by | Model |
| --- | --- | --- |
| a build | `implement` | `sonnet` |
| a rebuild after a build stopped (not on a backlog item or spec change) | `implement` | the session's own model (stepping up) |
| a build's `spec-auditor`, its final review | the build | the `--review-model` `implement` passes to `clm run`: the session's own model |
| any other subagent a build launches | the build | `sonnet` |
| a draft | `triage`, `build` | the session's own model (design) |
| the fold | `triage`, `build` | `sonnet` |
| a builder check | `propose`, a draft, the fold | `sonnet`, the model the builder will run on |
| `architecture-auditor`, a design review (`spec-architecture-audit`) | `propose`, `contribution`, a draft, the fold | the session's own model (`sonnet` in the fold) |
| `spec-slimmer` | `health` | `sonnet` |
| per-node spec drafting from code | `init` | `sonnet` |

"The session's own model" is the main session's, whatever it is, never a fixed top tier. `sonnet` is the floor superpowers sets for implementers working from prose and for reviewers ("turn count beats token price"), and CLM's builds work from prose requirements; nothing runs on `haiku`.

**How the review model reaches a build.** The build runs on `sonnet`, so it can't name the main session's model by inheritance. `implement` knows its own model and passes its alias with `clm run <node> --review-model <alias>`, which `clm run` writes into the prompt unchecked (the Agent tool refuses an unknown alias). Without the flag, as from an older skill, the prompt says to name the build's own model, so the review is still named, on the cheaper model.

**Edge cases accepted.**
- `/code-review`, which a build runs as a skill, launches its own reviewers as Claude Code decides; CLM names no model for them.
- A main session whose model has no alias in the Agent tool's list names the nearest alias.
- `sonnet` appears as a constant in the skills, `clm-run`'s prompt and `clm-triage`'s prompts; moving it is a code change in each, as moving the superpowers pin is.

**Rejected.** A `[models]` setting in `clm.toml` (YAGNI: no adopter asked to choose, and the main session's model can't be configured ahead); naming `opus` for reviews (a fixed top tier the principle rules out); having `implement` run the final review itself after the build returns (the build lands before it returns, and moving the review out would redesign the build flow).

**Would change if** effort accounting shows builds on `sonnet` stopping and stepping up often enough that they cost more than building on the main model.
