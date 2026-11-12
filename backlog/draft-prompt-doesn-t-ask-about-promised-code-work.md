---
title: "The clm draft prompt doesn't ask whether a design-only change's outcome promises code work nothing builds"
kind: bug
components: [orchestrator.triage]
origin: 20261112-propose-skill-doesn-t
---

Step: the prompt `clm draft` prints, its design and builder-check steps, for a change whose deltas add, modify and remove no requirement.
What happened: `20261112-propose-skill-doesn-t` has `/clm:propose` record the code work such a change relies on (a requirement when one would have to state it, otherwise a `kind: deferred` item naming the code) and has its builder check ask whether the outcome promises code work no requirement the change adds or modifies makes pending. The draft prompt, the other spec-writing path (`dependency-work-is-a-requirement`), says neither, so a design-only draft can still promise code that no build does.
Expected: the draft prompt says the same as `plugin.propose/propose-archives-design-only` on recording promised code work, and its builder check asks the same question.
Rules involved: node-reconciliation, dependency-work-is-a-requirement, deferred-work-resolves-in-code.
