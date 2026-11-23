---
title: "The propose skill gives no route for a bug in prompt wording"
kind: deferred
components: [plugin]
origin: 20261116-the-draft-prompt-gives
---

`code-fix-needs-a-written-rule` now says that a bug in the fixed text of a prompt or skill, which no requirement may state (`prompt-text-is-source`), is fixed by component with `clm claim <component> --fix backlog/<slug>` and a `Resolves:` trailer, adding no test of the wording; so is a code or doc comment that contradicts a Contract, Design or decision. The draft's prompt in `crates/clm-triage/src/draft.rs` names both cases beside test plumbing since `927b9d80` (`orchestrator.triage`'s half). One text that routes bugs still sends every bug no requirement states to a spec change:

- `plugin/skills/propose/SKILL.md`, `## Code fixes`, the paragraph that begins "A bug takes this route only when": it should name the same two cases beside test plumbing, each fixed by component with no test. This is `plugin`'s.

Code-only work: it changes nothing a requirement states. Fixed under `clm claim plugin --fix`, whose commit deletes this item with a `Resolves:` trailer. A draft meeting such a bug before then takes the route the decision states.
