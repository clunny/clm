---
status: applied
resolves: [propose-nests-splits-forbids-the-guide-pages-propose-names]
---

## Intent

Drop the sentence "It SHALL NOT name `docs/guide/`" from `plugin/propose-nests-splits`, and the matching clause of its scenario, so the requirement stops contradicting `plugin/method-pointers`, `plugin/propose-adds-glossary-terms` and `plugin/propose-adds-settings-sections`, which allow or require `skills/propose/SKILL.md` to name `docs/guide/glossary.md` and `docs/guide/settings.md`. `method-pointers` stays the only rule about which guide paths a skill names.

## Problem

`propose-nests-splits` forbids `skills/propose/SKILL.md` to name `docs/guide/` at all, while three other requirements of the same component have it name two guide pages by path. The build followed the three, and the test for `propose-nests-splits` strips both paths before checking: a test working around its own requirement's text.

## Approaches

- *Reword the sentence* as "names no guide page by path except the two `method-pointers` allows". Keeps the rule in two places that must agree, and the next guide page `propose` needs would have to change both. Rejected.
- *Chosen: drop the sentence* and the scenario's clause. `method-pointers` already states, for every skill and agent, which guide paths may be named; a second copy in `propose-nests-splits` only adds a place to disagree (`reuse-first`).

## Outcome

`propose-nests-splits` says what the split guidance must be and nothing about guide paths. Its build re-cites the test at the new version and drops the `docs/guide/` check, with the stripping of the two pages, from that test; `method-pointers`' test keeps checking the guide paths.

## Edge cases and workarounds

None.

## Settled by the builder check and the audit

- The builder check found no requirement this one relies on that isn't current, and no capability test that sees a difference; `skills/propose/SKILL.md` stays as it is.
- The audit noted that `plugin`'s Design ("**The method**") said a skill names `docs/guide/...` never, which `method-pointers` already contradicted. The change rewords it to name the two pages `method-pointers` allows.

## The human's answers

> propose-nests-splits-forbids-the-guide-pages-propose-names — Human's answer: "Drop the sentence (Recommended)" — Remove 'It SHALL NOT name docs/guide/' and the matching scenario clause. method-pointers stays the only rule about guide paths.
