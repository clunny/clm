---
status: applied
resolves: [the-draft-prompt-gives-no-route-for-a-bug-in-a-code-comment-s]
---

## Intent

Say where a bug in a code comment's wording goes. `code-fix-needs-a-written-rule` gains a third exception beside test plumbing and prompt text: a code or doc comment that contradicts a Contract, Design or decision is fixed by component, with `clm claim <component> --fix`, a `Resolves:` trailer and no test. `AGENTS.md`'s defect paragraph says the same. The draft prompt's and the propose skill's wording is folded into the existing deferred item that already asks them to name the prompt-text case, and the bug is deleted.

## Problem

A draft for `backlog/clm-coverage-doc-still-says-clm-check-re-exports-it` met a crate doc comment that contradicted `check.coverage`'s Contract. The decision sends every bug no requirement states to a spec change that adds one. A comment's wording restores no behaviour a test can observe, so nothing can cite a requirement, and a requirement stating a comment's text would be a rule about source wording (`yagni`, and the reasoning of `prompt-text-is-source`). The decision's exceptions named test plumbing, the capability harness and prompt text, not comments, so the session worked out the route itself and took the prompt-text one by analogy (`93bd10cf`).

## Approaches

- **Taken: name comments in the decision, and fold the prompt wording into the deferred item.** The decision is where the route is stated, and `AGENTS.md` repeats it for this repo's sessions. The draft prompt (`crates/clm-triage/src/draft.rs`) and the propose skill (`plugin/skills/propose/SKILL.md`) are source, and their fixed text can't be edited on a spec branch. `backlog/the-draft-prompt-and-propose-skill-give-no-route-for-a-bug-in-prompt-wording` already asks those same two paragraphs to name the prompt-text exception, so the comment case joins it (`reuse-first`), as `20261116-a-build-whose-only` folded its bug into a deferred item.
- **Rejected: a requirement of each component stating its comments' text.** A scan of source wording, which `prompt-text-is-source` removed for prompts; it proves nothing about the product.
- **Rejected: fixing the prompt now on `impl/orchestrator.triage`.** The freeze answers defects with "accept" by default (`backlog/a-passing-check-proves-less-than-the-spec-claims`), the deferred item already holds this paragraph, and a draft meeting such a bug until then reads the route in the decision.
- **Rejected: accepting with no edit.** The next draft would have to work the route out again, which is the defect.

## Outcome

- `docs/decisions/code-fix-needs-a-written-rule.md` has an "Outside the rule: the wording of a code comment" paragraph. A comment wrong because its code is wrong stays under the rule: the code's bug needs a written rule.
- `AGENTS.md`'s CLM-defects paragraph names the comment case beside prompt text.
- `docs/guide/how-it-works.md`, which names test plumbing as the bug route's one exception, names prompt text and comments too, so the guide `clm guide` prints says it (`method-in-the-plugin`), as the architecture audit found.
- The deferred item's two bullets now ask for the comment case beside the prompt-text one.
- The bug's item is deleted.

## Constraints

- No requirement changes, so nothing is pending; the prompt and skill wording waits on the deferred item, which a code branch resolves with `Resolves: backlog/the-draft-prompt-and-propose-skill-give-no-route-for-a-bug-in-prompt-wording` (`deferred-work-resolves-in-code`).

## Edge cases and workarounds

None.

## Open questions

- The test in `crates/clm-triage/tests/draft.rs` that reads the paragraph the deferred item rewrites: whether the build keeps its assertions or drops wording assertions is the build's to settle under `prompt-text-is-source`, as the item already says the test follows the paragraph.
