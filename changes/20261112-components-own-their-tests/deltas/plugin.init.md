---
component: plugin.init
base: {}
---

## ADDED Requirements

### Requirement: init-skill-gives-tests-their-component
`skills/init/SKILL.md` SHALL tell Claude, when it recovers an existing codebase's graph, to give each component the tests that prove it with `tests` globs, by directory or by a file-name pattern, never as a list of files, so that a test file added later is owned (`components-own-their-tests`); to give a component whose tests import the product's entry point or a shared test harness a `test_depends_on` naming the component that holds it, rather than giving the whole suite to one component; to keep the shared harness (fixtures, helpers, a test app factory) in a component of its own; to prefer directory globs in `paths` too, so a file added later beside a component's files is owned; and, where one existing test file tests several components, to give it to one of them and record a `kind: deferred` backlog item for splitting it, naming the file and the components.

#### Scenario: skill text
- GIVEN `plugin/skills/init/SKILL.md`
- WHEN a test reads it
- THEN it names `tests`, `test_depends_on` and `components-own-their-tests`, and says never to list test files one by one

#### Serves
- architecture/path-ownership
- capability/adopt-clm

#### Needs
- init/survey-components
