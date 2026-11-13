---
title: "A component's files may be exempt from one named code rule for a time, each exemption naming the backlog item that ends it"
date: 2026-11-13
status: accepted
---

`components-own-their-tests` gave a component's tests a way to import the product's entry point above them, `test_depends_on`, so an existing suite built around the whole app could be adopted as it was. That declared an upward edge instead of removing it. A test that runs the whole product depends on code its build never sees (`ancestor-scoped-context`), can be broken by a change anywhere, which test selection assumes it can't (`land-checks-stay-fast`), and blames no one component when it fails. A component's tests should follow the same dependency rule as its code: its requirements are proven through its own surface, and a test of the whole product belongs to the component that holds the entry point, or to the capability layer. Refactoring an existing app toward that is better than accommodating the pattern, so `test_depends_on` goes, before any project has used it.

What adoption, or any project at any time, still needs is a way to say "this owned code doesn't meet that rule yet" without switching the rule off, and without pretending the code is unowned. So:

- **An exemption** in a component's frontmatter names one rule (`undeclared-imports`, `tests-cite-their-owner` or `skipped-tests`), the component's own files it covers, the backlog item whose work ends it (`until`), and optionally why. Those rules skip those files, and every other rule still applies to them.
- **Adding or widening one needs review** (`gate/needs-review`, and `humans-review-intent` and `yes-no-review` list it); removing or narrowing one, or rewording its `why`, doesn't. A new exemption's item must exist when it's added (`gate/exemption-names-its-item`).
- **It stays in sight**: `clm health` lists every exemption with how many files it still covers, and flags one whose item is gone or whose files are gone, for a change to remove. Neither is a `clm check` failure, so the build that fixes the code lands, and the exemption is removed by a spec change after it.
- **Builds never add one** (`orchestrator.run/stays-in-its-component`); a build that would need one leaves its requirement unbuilt with a `question` item.
- **Recovery uses them** (`plugin.init/init-skill-gives-tests-their-component`): what doesn't meet a rule at adoption gets an exemption and an item, never `outside`.

**Exemption, not `outside`.** `outside` is code CLM makes no claims about: no owner, no requirements, no rules, whether not adopted yet or never meant to be, like vendored code (`incremental-adoption`, unchanged). Exempt code is adopted, owned and specified; it just doesn't meet one rule yet. Putting it in `outside` would have CLM look away from code it is responsible for.

**Why the tests still run, and on every land.** Until a feature's own tests replace them, exempt tests are the only thing guarding its behaviour (`proven-by-tests`). A file exempt from `undeclared-imports` imports code the graph doesn't connect it to, so any change can break it; test selection runs it whenever any component is affected (`check.affected/affected-files`), keeping `land-checks-stay-fast`'s guarantee. That makes the debt cost land time, which is a reason to pay it down.

**Withdrawing `test_depends_on` follows `expand-contract`.** `check`, `check.affected` and its `why` chain, `context` and `init` stop reading it first; `model/test-dependencies-retired` removes it after them, the field marked `clm:remove-by` this change meanwhile.

Rejected:
- **`test_depends_on`**, above. Withdrawn.
- **A blanket "awaiting refactoring" marker over an area.** It switches off every rule, so a file under it can get worse unseen. One rule per exemption keeps the rest working.
- **Exemptions in `clm.toml`.** It's a code path a build can edit with no review, so a build could exempt itself (as `incremental-adoption` found for `outside`). The detector allowances there stay, since they configure tools rather than the spec graph's rules (`unreferenced-allow-then-narrow`).
- **`clm check` failing an exemption whose item is gone or whose files are gone.** The code fix would then fail its own land and need a spec edit beside it, which the gate refuses as `mixed`.
- **A deadline date per exemption.** When the work is due is the human's call, made by its item's priority; `clm health` shows what is left.
- **Names:** *waiver* (legalistic), *allowance* (sounds permanent, and the detector allowances already use it), *deferral* (too close to the `deferred` backlog kind).

## The rule in full

`ARCHITECTURE.md` stated this as the entry `exemptions-end` until 2026-11-13, when the entry was cut to its rule (`human-layer-entries-stay-short`). The text still holds:

A rule `clm check` judges code by SHALL hold for every file a component owns, except a file that one of that component's exemptions names for that rule; each exemption SHALL name its rule, its files and the backlog item whose work ends it, SHALL be added only by a change a human reviews, and SHALL stay in sight until a change removes it (`temporary-exemptions`). Code CLM makes no claims about at all, such as code not adopted yet or vendored code, is the root's `outside` instead (`incremental-adoption`).

**Scenario: adopting a suite that imports the app**
- GIVEN an existing test suite, adopted into feature components, whose tests import the app's entry point above them
- WHEN the first spec graph is written
- THEN each feature that owns such tests holds an exemption from `undeclared-imports` for them, naming the item that has its builds prove its requirements through its own surface, and `clm check` passes

**Scenario: an exemption a build would add**
- GIVEN a build whose requirement needs an import its component's `depends_on` doesn't allow
- WHEN it builds
- THEN it adds no exemption, leaves the requirement unbuilt, and adds a `question` item whose `blocks` names it

**Untested**
When an exemption's work is done is the human's call, made by ordering its item in the backlog; `clm health` shows each exemption, how much it still covers and whether its item is gone.
