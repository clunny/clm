---
component: ci
---

## MODIFIED Requirements

### Requirement: pull-request-template <!-- base: 4dbdf4287a5367dc -->
`.github/PULL_REQUEST_TEMPLATE.md` SHALL tell the author that a pull request changing any file outside the spec paths, which it SHALL list, is closed without review, SHALL point at `CONTRIBUTING.md`, and SHALL ask for the issue the work comes from.

#### Scenario: an author reads it
- GIVEN this repository's `.github/PULL_REQUEST_TEMPLATE.md`
- WHEN a test reads it
- THEN it names `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md` and `closed`

#### Serves
- capability/contribute-from-outside
