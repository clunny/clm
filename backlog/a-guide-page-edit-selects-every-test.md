---
title: "A guide page edit selects every test"
kind: bug
components: [init, check.affected]
---

Step: `clm land` of a spec branch, whose `[land] check` is handed `clm affected`'s selection.

What happened: the branch edited a page under `docs/guide/`, as the propose skill requires when a change adds a settings key, a glossary term or guidance. `clm affected` printed `all docs/guide/<page>.md`, so the land ran every test: the full Rust suite and every browser and capability suite, about 20 minutes on that land. `docs/guide/` counts as code for the ledger's code key, because `clm-init` compiles the guide into `clm guide`, but no component's `paths` hold it, and `check.affected` selects everything for a changed file with no owner.

Expected: a guide edit selects the tests of the component that builds the guide (`init`) and its dependents, as any owned file does, not every test.

Rules involved: `check.affected`'s `all` for an unowned file, `ledger`'s `doc_only` (which keeps `docs/guide/` as code), and the propose skill's rule that the guide's glossary and settings pages are spec paths a spec branch carries. The fix likely gives `init` ownership of `docs/guide/**`, which needs a spec change and a check of how the gate treats a component path that is also a spec path.
