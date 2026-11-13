---
title: "A component's files may be exempt from one named code rule for a time, each exemption naming the backlog item that ends it"
date: 2026-11-13
status: accepted
---

`components-own-their-tests` gave a component's tests a way to import the product's entry point above them, `test_depends_on`, so an existing suite built around the whole app could be adopted as it was. That declared an upward edge instead of removing it. A test that runs the whole product depends on code its build never sees (`ancestor-scoped-context`), can be broken by a change anywhere, which test selection assumes it can't (`land-checks-stay-fast`), and blames no one component when it fails. A component's tests should follow the same dependency rule as its code: its requirements are proven through its own surface, and a test of the whole product belongs to the component that holds the entry point, or to the capability layer. Refactoring an existing app toward that is better than accommodating the pattern, so `test_depends_on` goes, before any project has used it.

What adoption, or any project at any time, still needs is a way to say "this owned code doesn't meet that rule yet" without switching the rule off, and without pretending the code is unowned. So:

- **An exemption** in a component's frontmatter names one rule (`undeclared-imports`, `tests-cite-their-owner` or `skipped-tests`), the component's own files it covers, the backlog item whose work ends it (`until`), and optionally why. Those rules skip those files, and every other rule still applies to them.
- **Adding or widening one needs review** (`gate/needs-review`); removing or narrowing one doesn't. A new exemption's item must exist when it's added (`gate/exemption-names-its-item`).
- **It stays in sight**: `clm health` lists every exemption with how many files it still covers, and flags one whose item is gone or whose files are gone, for a change to remove. Neither is a `clm check` failure, so the build that fixes the code lands, and the exemption is removed by a spec change after it.
- **Builds never add one** (`orchestrator.run/stays-in-its-component`); a build that would need one leaves its requirement unbuilt with a `question` item.
- **Recovery uses them** (`plugin.init/init-skill-gives-tests-their-component`): what doesn't meet a rule at adoption gets an exemption and an item, never `outside`.

**Exemption, not `outside`.** `outside` is code CLM makes no claims about: no owner, no requirements, no rules, whether not adopted yet or never meant to be, like vendored code (`incremental-adoption`, unchanged). Exempt code is adopted, owned and specified; it just doesn't meet one rule yet. Putting it in `outside` would have CLM look away from code it is responsible for.

**Why the tests still run.** Exempt tests are selected by their owner as any test is. Until a feature's own tests replace them, they are the only thing guarding its behaviour (`proven-by-tests`).

Rejected:
- **`test_depends_on`**, above. Withdrawn.
- **A blanket "awaiting refactoring" marker over an area.** It switches off every rule, so a file under it can get worse unseen. One rule per exemption keeps the rest working.
- **Exemptions in `clm.toml`.** It's a code path a build can edit with no review, so a build could exempt itself (as `incremental-adoption` found for `outside`). The detector allowances there stay, since they configure tools rather than the spec graph's rules (`unreferenced-allow-then-narrow`).
- **`clm check` failing an exemption whose item is gone or whose files are gone.** The code fix would then fail its own land and need a spec edit beside it, which the gate refuses as `mixed`.
- **A deadline date per exemption.** When the work is due is the human's call, made by its item's priority; `clm health` shows what is left.
- **Names:** *waiver* (legalistic), *allowance* (sounds permanent, and the detector allowances already use it), *deferral* (too close to the `deferred` backlog kind).
