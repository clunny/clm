---
status: applied
from: backlog/clm-s-cli-tests-belong-to-the-root
---

## Intent

Let a component's files be exempt from one named code rule for a time, each exemption naming the backlog item whose work ends it, added only through a reviewed change and kept in sight by `clm health`. Recovery uses exemptions for what doesn't meet a rule at adoption, instead of accommodating it. And withdraw `test_depends_on`, which `20261112-components-own-their-tests` added and no project has used yet: it legalised tests that import the product above their component, which breaks the build's context, test selection and blame. A component's tests now follow the same dependency rule as its code.

## Problem

`test_depends_on` let a component's tests import the product's entry point, so an existing suite built around the whole app could be adopted as it was. That declares an upward edge instead of removing it. A test that runs the whole product depends on code its build never sees (`ancestor-scoped-context`), can be broken by a change anywhere, which test selection assumes it can't (`land-checks-stay-fast`), and blames no one component when it fails. The right shape is that a component proves its requirements through its own surface, and a test of the whole product belongs to the component that holds the entry point, or to the capability layer.

Adopting an existing app still meets code that is owned and specified but doesn't meet a rule yet. CLM had only `outside` for it, which means "no claims at all" and would hide adopted code from every rule. Projects also meet such cases later, when a fix can't happen yet. Neither has a way to say "this owned code is exempt from that rule, temporarily, until this work is done".

## Approaches

- **Keep `test_depends_on` as a declared allowance.** It's convenient for adoption, but it makes a breach of the layering legal and needs `affected` and `context` to follow it. Rejected.
- **Put refactoring debt in `outside`.** `outside` means CLM makes no claims; exempt code is adopted and owned. Rejected.
- **A blanket "awaiting refactoring" marker over an area.** It switches off every rule there, so a file can get worse unseen. Rejected.
- **Chosen: exemptions, one rule each** (`temporary-exemptions`). Each names its rule, its component's own files, the item that ends it and why. Adding or widening one needs review, and `clm health` lists each with what it still covers. Builds never add one.

## Outcome

- `model/exemptions`: the `exemptions` frontmatter key. `model/test-ownership-keys` no longer describes `test_depends_on`, and `model/test-dependencies-retired` makes it an unknown key once `check`, `check.affected`, `context` and `init` no longer read it.
- `check`: `undeclared-imports`, `tests-cite-their-owner` and `skipped-tests` skip the files an exemption names for them; test files are judged against `depends_on` alone.
- `check.affected/affected-components` and `context/bundle-contents`: the `test_depends_on` additions are reverted.
- `init/survey-components`: the component map takes `exemptions` and refuses `test_depends_on`.
- `gate/needs-review`: adding or widening an exemption needs review. `gate/exemption-names-its-item`: a new exemption's item must exist.
- `check.health/exemptions-signal`: each exemption, its file count, and whether its item or files are gone.
- `orchestrator.run/stays-in-its-component`: builds never add an exemption, and add nothing to an exempt file that the exemption would excuse.
- `plugin.init/init-skill-gives-tests-their-component`: recovery gives whole-product tests to the component that holds the entry point, has features prove their requirements through their own surface, and gives what doesn't meet a rule at adoption an exemption with an item, never `outside`.
- `ARCHITECTURE.md`: a new carried constraint, `exemptions-end`; `path-ownership`'s shared-suite scenario no longer imports the entry point.
- Decision `temporary-exemptions`; `components-own-their-tests` and `incremental-adoption` amended.
- Guide: the glossary gains Exemption and loses `test_depends_on`; `ci-and-deployment` and `spec-format` follow.
- Backlog: `clm-s-cli-tests-belong-to-the-root` reworded: each crate offers a library entry point its own tests call, and the binary keeps only tests of dispatch.

## Constraints

- `outside` is unchanged, and so are projects that use it.
- `test_depends_on` was never in a release a project adopted, so withdrawing it needs no `upgrade` migration.
- The withdrawal is sequenced: `check`, `check.affected`, `context` and `init` stop reading `test_depends_on` before `model` removes it (`coupled-changes-sequence`).
