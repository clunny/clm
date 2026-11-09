---
component: init
---

## MODIFIED Requirements
### Requirement: claude-md-names-the-skills <!-- base: c18c16d773ec14cc -->
The `CLAUDE.md` that `clm init` writes SHALL name `/clm:propose`, `/clm:build` and `/clm:help`, and SHALL NOT name `/clm:implement`.

#### Scenario: a new repository
- GIVEN a repository with no `CLAUDE.md`
- WHEN `clm init` runs
- THEN its `CLAUDE.md` holds `/clm:propose`, `/clm:build` and `/clm:help`, and not `/clm:implement`

#### Needs
- plugin.build/skill-named-build
- plugin/help-skill
