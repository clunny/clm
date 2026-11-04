---
component: plugin
base: {}
---

## ADDED Requirements

### Requirement: propose-adds-glossary-terms
`skills/propose/SKILL.md` SHALL tell the session that when a requirement its change adds or modifies introduces a term `clm/glossary-covers-terms` covers (a top-level `clm` command, a requirement state, a component status, a change status, a backlog kind or a key of `change.md`'s frontmatter), the same spec branch adds the term's entry to the guide's glossary, `docs/guide/glossary.md`, with a definition in plain words: the glossary is a spec path, so the code branch that builds the term can't carry it, and without it that branch's land fails `clm/glossary-covers-terms`. A test SHALL read the skill and fail naming the missing text.

#### Scenario: the skill says so
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it holds `docs/guide/glossary.md` and `clm/glossary-covers-terms` in one paragraph, which also names a command, a status, a kind and a frontmatter key

#### Scenario: a skill without it
- GIVEN a `propose` `SKILL.md` that doesn't mention `docs/guide/glossary.md`
- WHEN the check runs on it
- THEN it reports `docs/guide/glossary.md`

#### Serves
- architecture/plain-terms
- capability/propose-intent
