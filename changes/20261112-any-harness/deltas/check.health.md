---
component: check.health
---

## MODIFIED Requirements

### Requirement: uncited-decisions <!-- base: 43e37f912a96bf09 -->
`clm health` SHALL report each accepted decision whose name appears, in a code span outside fenced code blocks, in no component's `SPEC.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md`, page under `docs/guide/`, file of a change outside the archive, or other decision, as `docs/decisions/<name>.md: health/uncited-decision: nothing names it`. A superseded decision SHALL NOT be reported.

#### Scenario: cited and uncited
- GIVEN decisions `kept` (named as `` `kept` `` in `specs/a/SPEC.md`), `lone` (named nowhere) and `old` (`status: superseded`, named nowhere)
- WHEN `clm health` runs
- THEN it prints `docs/decisions/lone.md: health/uncited-decision: nothing names it` and no `uncited-decision` line for `kept` or `old`

#### Scenario: named only in a fence
- GIVEN decision `fenced`, whose only mention is `` `fenced` `` inside a ```` ``` ```` block of `specs/a/SPEC.md`
- WHEN `clm health` runs
- THEN it prints an `uncited-decision` line for `fenced`
