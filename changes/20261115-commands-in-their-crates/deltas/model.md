---
component: model
---

## ADDED Requirements

### Requirement: violation-line
`clm-model` SHALL offer the line a violation prints as, `<path>: <rule>: <message>`, so that every crate that prints violations gives the same line whether or not it depends on `check`.

#### Scenario: one violation
- GIVEN a violation with path `specs/a/SPEC.md`, rule `check/x` and message `m`
- WHEN its line is asked for
- THEN it is `specs/a/SPEC.md: check/x: m`
