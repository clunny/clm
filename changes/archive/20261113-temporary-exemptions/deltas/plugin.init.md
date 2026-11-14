---
component: plugin.init
base: {}
---

## MODIFIED Requirements

### Requirement: init-skill-gives-tests-their-component <!-- base: 342b4d5f6931c51f -->
`skills/init/SKILL.md` SHALL tell Claude, when it recovers an existing codebase's graph, to give each component the tests that prove it with `tests` globs, by directory or by a file-name pattern, never as a list of files, so that a test file added later is owned (`components-own-their-tests`); to prefer directory globs in `paths` too, so a file added later beside a component's files is owned; that a component's tests import only what its code may (`depends_on`), so a test that runs the whole product through its entry point belongs to the component that holds the entry point, as a test of the product, and a feature's requirements are proven through the feature's own surface by tests its builds write; to keep shared test helpers that features' tests use in a component of their own below those features, never importing the entry point; and, for owned code or tests that break a rule `clm check` judges and can't be put right at adoption (an upward import, a test file that tests several components, a skipped test), to give the owning component an exemption naming the rule, the files and a backlog item that ends it, with a line saying why (`model/exemptions`, `temporary-exemptions`), adding that item, and never to put adopted code in the root's `outside`, which is for code CLM makes no claims about (`incremental-adoption`). It SHALL NOT name `test_depends_on`.

#### Scenario: skill text
- GIVEN `plugin/skills/init/SKILL.md`
- WHEN a test reads it
- THEN it names `tests`, `exemptions`, `until`, `components-own-their-tests` and `temporary-exemptions`, says never to list test files one by one and that a test running the whole product belongs to the component holding the entry point, and doesn't contain `test_depends_on`

#### Serves
- architecture/path-ownership
- capability/adopt-clm

#### Needs
- init/survey-components
- model/exemptions
