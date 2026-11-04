---
component: orchestrator.triage
base: {}
---

## ADDED Requirements

### Requirement: draft-adds-glossary-terms
The prompt `clm draft` writes SHALL tell the session that when a requirement its change adds or modifies introduces a term `clm/glossary-covers-terms` covers (a top-level `clm` command, a requirement state, a component status, a change status, a backlog kind or a key of `change.md`'s frontmatter), it adds the term's entry to `docs/guide/glossary.md` on `spec/<id>`, since the code branch that builds the term can't carry a spec path and its land fails without the entry. It says so in the paragraph that tells the session to design the rest as one change, and keeps to the wording `draft-prompt-file` allows.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `Design the rest as one change` also contains `docs/guide/glossary.md` and `clm/glossary-covers-terms`

#### Serves
- architecture/plain-terms
