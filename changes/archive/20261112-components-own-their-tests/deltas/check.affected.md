---
component: check.affected
base: {}
---

## MODIFIED Requirements

### Requirement: affected-components <!-- base: 6fdb3e7041d79495 -->
`clm affected` SHALL print a component line (the Contract's) for each component that owns a changed file (`check.coverage/owner`) and for each component that depends on one of those, directly or indirectly, sorted by ID, and for each component whose `test_depends_on` names one of those (`model/test-ownership-keys`), since its tests import it, and for no other component.

#### Scenario: dependants
- GIVEN a root with no dependencies, components `a`, `b` depending on `a`, `c` depending on `b`, and `d` depending on nothing, and a branch that changes only `crates/a/src/lib.rs`, which `a`'s `paths` hold
- WHEN `clm affected --base main` runs on the branch
- THEN it prints the component lines of `a`, `b` and `c`, and no other component line

#### Scenario: tests import a changed component
- GIVEN components `app` depending on `feat`, and `feat` with `test_depends_on: [app]`, and a branch that changes only a file `app`'s `paths` hold
- WHEN `clm affected --base main` runs on the branch
- THEN it prints the component lines of `app` and `feat`

#### Scenario: nothing changed
- GIVEN a branch with no commits beyond `main` and a clean working tree
- WHEN `clm affected --base main` runs
- THEN it prints nothing and exits 0

#### Needs
- check/path-ownership
- check.coverage/owner
- model/test-ownership-keys
