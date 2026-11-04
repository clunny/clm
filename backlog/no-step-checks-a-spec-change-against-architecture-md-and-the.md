---
title: "No step checks a spec change against ARCHITECTURE.md and the decisions it touches"
kind: bug
components: [plugin]
---

Step: /clm:contribution and /clm:propose, from designing a change to landing it.

What happened: a session designed and landed spec changes whose design broke an architectural constraint (a build would have had to edit another component's code for an addition, which path-ownership allows only for a removal) and missed others (an expand-contract marker, a guide entry method-in-the-plugin calls for, a capability a rule serves). Every check passed: clm check, clm rebase, the gate and the land's tests prove that specs are well-formed and consistent, and the builder check asks whether a change can be built, not whether it fits the constraints. The session found the problems only when the human asked whether it had followed CLM's philosophy; it had read the constraints' names but not their text, although propose's step 1 says to read ARCHITECTURE.md.

Expected: propose (and so contribution, triage's drafts and the fold) should run an architecture review beside the builder check: a fresh subagent that reads the change against ARCHITECTURE.md's constraints and the decisions the change names or touches, and lists each constraint the design breaks or leaves a gap against, as advice the session settles before the pull request (deterministic-enforcement: it advises, never gates). The spec-auditor agent audits code against specs; nothing audits specs against the constraints.

Rules involved: right-over-convenient, ask-the-architect, deterministic-enforcement, advisory-code-review.
