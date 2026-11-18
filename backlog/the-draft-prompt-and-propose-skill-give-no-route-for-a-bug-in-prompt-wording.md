---
title: "The draft prompt and the propose skill give no route for a bug in prompt wording"
kind: deferred
components: [orchestrator.triage, plugin]
origin: 20261116-the-draft-prompt-gives
---

`code-fix-needs-a-written-rule` now says that a bug in the fixed text of a prompt or skill, which no requirement may state (`prompt-text-is-source`), is fixed by component with `clm claim <component> --fix backlog/<slug>` and a `Resolves:` trailer, adding no test of the wording. Two texts that route bugs still send every bug no requirement states to a spec change:

- `crates/clm-triage/src/draft.rs`, the draft's prompt, the paragraph ending `(code-fix-needs-a-written-rule)` that says a bug is code-only only when a requirement as written states what its fix restores. It should name the prompt-text case and the code-comment case (a code or doc comment that contradicts a Contract, Design or decision, added to the decision by `20261118-the-draft-prompt-gives`) beside the test-plumbing one, both fixed by component with no test. This is `orchestrator.triage`'s; the test in `crates/clm-triage/tests/draft.rs` that reads that paragraph follows it.
- `plugin/skills/propose/SKILL.md`, `## Code fixes`, the paragraph that begins "A bug takes this route only when": the same two cases. This is `plugin`'s.

Code-only work: it changes nothing a requirement states. One component at a time, each under its own `clm claim <component> --fix`, the last deleting this item. A draft meeting such a bug before then takes the route the decision states.
