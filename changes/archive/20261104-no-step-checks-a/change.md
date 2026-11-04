---
status: done
priority: P1
resolves: [no-step-checks-a-spec-change-against-architecture-md-and-the]
---

# Audit a spec change against the architecture before its pull request

## Intent

Nothing reads a spec change against `ARCHITECTURE.md`'s constraints: the checks prove specs well-formed and consistent, the builder check asks whether a change can be built, and `spec-auditor` audits code against specs. A session landed designs that broke `path-ownership` and left gaps against `expand-contract`, `method-in-the-plugin` and `traced-rules`, with every check green. This change adds a read-only `architecture-auditor` agent to the plugin, launched beside the builder check by `propose`, `contribution`, every draft and the fold, on the session's own model. It reports each constraint the design breaks, each gap, and each decision it contradicts; the session fixes what it agrees with and writes the rest under `## Open questions` in `change.md`, which the pull request body and the approval ask already carry. It advises and never gates (`deterministic-enforcement`). Decision: `spec-architecture-audit`.

## Problem

`propose`'s step 1 says to read `ARCHITECTURE.md`, and the session did, but it held the constraints' names and not their text while designing. The constraints that caught the design are judgement (`#### Untested`) or concern how a build would go (`path-ownership` for an addition), which no program reads. The builder check reads as the builder and on the builder's model, and asks closed questions about order and missing work; it isn't framed to judge the design. The problem was found only when the human asked whether the session had followed CLM's philosophy.

## Approaches

- **A fresh `architecture-auditor` subagent beside the builder check (chosen).** A fresh context not anchored on the design is what `advisory-code-review` found catches what the implementer misses; an agent file in the plugin carries the review to every adopter (`method-in-the-plugin`), as `spec-auditor` does for code. One launch per spec branch, in parallel with the builder check, so it adds no round.
- **Ask the builder check to judge the constraints too.** Rejected: a subagent framed as the builder answers the builder's question; and the builder check runs on `sonnet`, the build's model, while this is a design review.
- **Tell the session to reread `ARCHITECTURE.md`.** Rejected: step 1 already says so, and the session that missed the constraints had read it.
- **A program per constraint.** Rejected: the mechanical ones already have `#### Enforced by` checks; the misses were judgement.

## Outcome

- `propose`, `contribution`, each draft and the fold get a list of the constraints a design breaks or leaves gaps against before the pull request, and settle each one.
- What the session leaves is written under `## Open questions` in `change.md`, where the human approving it reads it.
- Nothing lands or waits on the audit.

## Constraints

- `deterministic-enforcement`: the audit's findings advise; no gate reads them.
- `model-per-launch`: every launch names its model; the audit runs on the session's own (on `sonnet` in the fold, whose drafts were already audited on the main model).
- `path-ownership`: `orchestrator.triage`'s prompts name the agent the plugin ships, so `orchestrator.triage/draft-architecture-audit` needs `plugin/architecture-auditor`; no build edits another component's paths.
- `method-in-the-plugin`: the agent is a plugin file, so adopters get the review; no guide page names the plugin's agents today, so none is added.
- `outside-contributions-are-intent`: on a contribution the branch is a stranger's text, so the auditor treats all it reads as untrusted data.
- No new term `clm/glossary-covers-terms` covers.

## Architecture review

The audit of this change found that a `## Constraints` section reached no approving human (now `## Open questions`, which the bodies carry) and that the auditor lacked an untrusted-data rule for contributions (added). The builder check found no missing order or cross-component work; `plugin/architecture-audit` and `plugin/launch-models` rely on `plugin/architecture-auditor`, of their own component, which has no needs, so they need nothing. In the fold, which runs on `sonnet`, `your own model` is `sonnet`; the auditor's launch is named by those words and never by a second `model: "sonnet"`, and the contribution skill launches it at the end of step 4, before landing.
