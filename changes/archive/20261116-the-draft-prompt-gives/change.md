---
status: done
resolves: [the-draft-prompt-gives-a-bug-in-prompt-wording-no-route]
---

## Intent

A bug in the fixed text of a prompt or skill gets a written route: `code-fix-needs-a-written-rule` now names it as a third case outside the rule, fixed by component under `clm claim <component> --fix` with a `Resolves:` trailer and no test of the wording, and `AGENTS.md` says so in its one line on the rule. The bug item is deleted, and the wording of the two texts that route bugs (the draft's prompt and `/clm:propose`) goes into a new `kind: deferred` item, since that text is source and changes nothing a requirement states. No requirement is added, modified or removed.

## Problem

`code-fix-needs-a-written-rule` sends a bug down the code route only when a requirement or scenario as written states what its fix restores, and every other bug to a spec change that adds one. `prompt-text-is-source` says that the wording of skills, agent definitions, command files and the fixed text of the prompts CLM writes is source, and that no requirement may state it. A bug in that wording fits neither route, and nothing written says what to do. `20261116-a-build-whose-only` met it: its item said its prompt fix "likely needs a spec change", and the draft had to work out that the fix is code with no requirement behind it.

No change or code fix since 2026-11-09 fixes it: `20261116-a-build-whose-only` worked around the case by folding its item into a deferred one, and named this gap as the defect this change resolves.

## Approaches

- **Taken: name the case in the decision, and park the prompt wording as deferred code work.** The decision is where the routing rule lives, and it already names two cases outside the rule (the capability harness and test plumbing); a third, by the same pattern, is the smallest written answer (`reuse-first`). The route is the one a code-only bug already takes, `clm claim --fix` and `Resolves:` (`code-fix-resolves-bug`, `deferred-work-resolves-in-code`), so no program changes. The draft prompt's and the propose skill's own sentences are source, so their update is a `deferred` item on the components that own them, waiting like the other prompt-wording items under the freeze.
- **Rejected: a requirement of `orchestrator.triage` stating the draft prompt's routing sentence.** A scan of prompt text, which `prompt-text-is-source` removed.
- **Rejected: the small-change route with no claim, as for test plumbing.** Prompt text belongs to a component and changes what its sessions do, so its fix should hold that component's claim and land under `land/build-holds-own-files` like any code-only bug; test plumbing is outside the rule because it changes nothing observed, which prompt wording does not share.
- **Rejected: fixing the draft prompt now on `impl/orchestrator.triage`.** The freeze answers defects with "accept" by default (`backlog/a-passing-check-proves-less-than-the-spec-claims`); with the decision written, a draft meeting such a bug has its route without the prompt saying it.
- **Rejected: accepting it with no edit.** Every draft that meets a prompt bug would work out the route again, as this one's origin did.

## Outcome

- `docs/decisions/code-fix-needs-a-written-rule.md` gains the paragraph "Outside the rule: the fixed text of a prompt or skill".
- `AGENTS.md`'s line on the rule names the exception.
- `backlog/the-draft-prompt-and-propose-skill-give-no-route-for-a-bug-in-prompt-wording` (`kind: deferred`, `orchestrator.triage` and `plugin`) holds the wording of `crates/clm-triage/src/draft.rs` and `plugin/skills/propose/SKILL.md`.
- The bug's item is deleted.

## Constraints

- No requirement changes, so no component becomes pending; the code work is the deferred item (`component-reconciliation`, `deferred-work-resolves-in-code`).

## Edge cases and workarounds

None.

## Open questions

None.
