---
component: effort
---

## ADDED Requirements
### Requirement: entry-text
`report` SHALL give each `Cost`, and `architecture` each `Constraint`, a `text`: its entry's Markdown in `CAPABILITIES.md` or `ARCHITECTURE.md` from the line after its `### ` heading to the next heading of level 3 or above or the end of the file, with leading and trailing blank lines removed, its `####` headings and blocks included. The text pages `clm capabilities <slug>` and `clm architecture <slug>` SHALL be unchanged by it, and `clm capabilities --format json` SHALL carry it.

#### Scenario: a capability's whole entry
- GIVEN `CAPABILITIES.md` holding `### Capability: x`, its statement with `` `clm land` `` in it, a `#### Why` block and a `#### Scenario: s` block, then `### Capability: y`
- WHEN `report` runs
- THEN `x`'s `text` starts with its statement, holds `` `clm land` ``, `#### Why` and `#### Scenario: s`, and holds nothing of `y`

#### Scenario: a constraint's whole entry
- GIVEN `ARCHITECTURE.md` holding entry `e` with `#### Enforced by` naming `a/x` and an `#### Untested` block
- WHEN `architecture` runs
- THEN `e`'s `text` holds its statement and both blocks

#### Serves
- capability/follow-from-browser
- capability/review-capability-cost
