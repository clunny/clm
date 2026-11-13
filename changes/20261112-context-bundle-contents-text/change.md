---
status: applied
resolves: [context-bundle-contents-text-is-garbled]
---

## Intent

Repair the text of the `context` spec so it says what `clm context` does. `bundle-contents` gets its statement back in one piece, with the `test dependency's contract` scenario moved out of it to stand beside the others. The Design's list of the bundle's sections is completed, and the Contract names `test_depends_on` beside `depends_on`. The behaviour stays as it is: the change only states, whole, what the code already does.

## Problem

`20261112-components-own-their-tests` wrote its new scenario into the middle of `bundle-contents`' first sentence, where the sentence mentions `#### Serves` blocks, so the rule reads cut off and holds a scenario inside its statement (`backlog/context-bundle-contents-text-is-garbled`). The Design has been cut off since the first public commit, at "before its contract, `#": it never says where the target's requirements and assets go, and has no entry for the `# CLM's constraints` or `# Dependencies` sections. The Contract still says the bundle holds the contract of "each component it depends on directly", with no mention of `test_depends_on`, which the requirement and the code (`dependencies` in `crates/clm-context/src/lib.rs`) both include.

## Approaches

- **Rewrite the three texts from the code and the requirement as they stand (chosen).** The requirement's statement is the one before the scenario was inserted, with the `test_depends_on` words that change added; the scenario becomes its own block. The Design's list follows `bundle_from` section by section. No behaviour changes.
- **Accept the text as it is.** It leaves a rule whose statement no reader can follow, which a build then has to guess at. Rejected.

## Outcome

- `context/bundle-contents` states its rule in one sentence pair, followed by the `test dependency's contract` scenario as a scenario of its own. The rule's version moves, so the tests in `crates/clm-context/tests/served.rs` and `crates/clm-context/tests/test_dependency.rs` that cite it are `behind` and are re-cited when `context` is next built; no code changes.
- The Contract names `depends_on` and `test_depends_on`.
- The Design lists the bundle's five sections in order: `# ARCHITECTURE.md`, `# CLM's constraints`, `# Capabilities served`, `# Ancestors` (with the target's frontmatter, requirements and assets) and `# Dependencies`.

## Constraints

- No behaviour changes: every scenario held before still holds, and the build only re-cites.
- Edge cases and workarounds: none.
