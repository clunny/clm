---
component: clm
---

## MODIFIED Requirements
### Requirement: architecture-term-retired <!-- base: 7e35623671e4b149 -->
`clm/retired-terms`'s list SHALL hold the old name of an entry of `ARCHITECTURE.md`, the word `architecture` followed by the word `requirement`, in the singular and in the plural, so neither comes back (`plain-terms`). `ARCHITECTURE.md` SHALL hold its entries as `### Constraint: <slug>` blocks inside `## Constraints` (`model/human-layers`), and no `clm:remove-by 20261102-rename-architecture-constraints-contract` marker SHALL be left.

#### Scenario: the old term retired
- GIVEN the repository
- WHEN the retired-terms test runs
- THEN its list holds the old term in the singular and the plural, and it finds neither

#### Serves
- architecture/plain-terms

#### Needs
- model/human-layers
- rename/requirement-rename
- init/init-files
- init/template-says-architectural-constraint
- plugin.init/init-skill
- plugin/skills-write-enforcement
- plugin/says-architectural-constraint
- effort/architecture-list
- effort/architecture-page
- effort/architecture-decide-command
- orchestrator.run/prompt-serves
- plugin.hooks.guard/approve-guard
