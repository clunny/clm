---
title: "A component's tests are not selected when a component it lists in test_depends_on changes"
kind: bug
components: [check.affected]
---

`clm context` now prints the contract of each `test_depends_on` component, so a change to that contract changes the dependent's bundle. `clm affected` (`crates/clm-affected/src/select.rs`, `why.rs`) selects dependents by `depends_on` only, so the dependent's tests may not run when that contract changes.
