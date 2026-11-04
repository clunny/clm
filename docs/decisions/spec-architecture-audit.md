---
title: "A fresh subagent audits each spec change against ARCHITECTURE.md and the decisions it touches, beside the builder check; it advises, never gates"
date: 2026-11-04
status: accepted
---

Resolves `backlog/no-step-checks-a-spec-change-against-architecture-md-and-the` (`changes/20261104-no-step-checks-a/`). Refines `advisory-code-review` and `blocked-by-backlog`, applying `right-over-convenient` and `deterministic-enforcement`.

**What happened.** A session designed and landed spec changes whose design broke `path-ownership` (a build would have had to edit another component's code for an addition) and left gaps against `expand-contract`, `method-in-the-plugin` and `traced-rules`. `clm check`, `clm rebase`, the gate and the land's tests passed: they prove specs well-formed and consistent. The builder check asks whether a change can be built, not whether it fits the constraints, and `spec-auditor` audits code against specs. Nothing read a spec change against the constraints, and the session, which had read their names but not their text, found the problems only when the human asked.

**Now.** The plugin ships a read-only `architecture-auditor` agent. `propose`, `contribution`, every draft and the fold launch it beside the builder check, in the same message, with only the change ID and the checkout. It reads `ARCHITECTURE.md` whole, the branch's diff, `clm context` for the touched components and the decisions the diff or the touched elements name, and reports each constraint the design breaks, each gap a constraint calls for, and each decision it contradicts, quoting the sentence. The session fixes what it agrees with and writes each finding it leaves under `## Constraints` in `change.md`, with why, which is what the human approving the change reads. It never gates: whether a design keeps a judgement constraint is model judgement (`deterministic-enforcement`).

**The model** is the session's own (`model-per-launch`): the audit reviews a design, as a build's final review does, not the work a `sonnet` build will do. The fold runs on `sonnet`, so its audit does too; each folded draft was already audited on the main model.

**Why not more, or less.**
- **Ask the builder check to also judge the constraints.** One subagent fewer (`reuse-first`), but the builder reads as the person who builds the change, and a reviewer asked two questions answers the one it was framed for; the builder check also runs on `sonnet`, the build's model, for a reason that doesn't hold for a design review.
- **Tell the session to read `ARCHITECTURE.md` more carefully.** `propose`'s step 1 already says to read it; the session did and still missed the text. A fresh context not anchored on the design is what `advisory-code-review` found works for code.
- **A program that checks each constraint.** The ones a program can check already are (`#### Enforced by`); the misses were in `#### Untested` judgement or in how a design would be built, which no check reads.
- **A gate on the findings.** `deterministic-enforcement` rules it out.

**Would change if** audits keep reporting the same broken constraint: that is a sign the constraint can be checked by a program, and the check belongs in `clm check`.
