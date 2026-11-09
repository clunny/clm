---
component: plugin.propose
---

## MODIFIED Requirements

### Requirement: propose-adds-glossary-terms <!-- base: fd6a2597f159faa3 -->
`skills/propose/SKILL.md` SHALL tell the session that when a requirement its change adds or modifies introduces a term `clm/glossary-covers-terms` covers (a top-level `clm` command, a requirement state, a component status, a change status, a backlog kind or a key of `change.md`'s frontmatter), the same spec branch adds the term's entry to the guide's glossary, `docs/guide/glossary.md`, with a definition in plain words: the glossary is a spec path, so the code branch that builds the term can't carry it, and without it that branch's land fails `clm/glossary-covers-terms`. It says so in one paragraph of the skill's design steps, without moving an existing step. A test, a function over the skill's text, SHALL read the skill and fail naming the missing text.

#### Scenario: the skill says so
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it holds `docs/guide/glossary.md` and `clm/glossary-covers-terms` in one paragraph, which also holds the words `command`, `status`, `kind` and `frontmatter`

#### Scenario: a skill without it
- GIVEN a `propose` `SKILL.md` that doesn't mention `docs/guide/glossary.md`
- WHEN the check runs on it
- THEN it reports `docs/guide/glossary.md`

#### Serves
- architecture/plain-terms
- capability/propose-intent

### Requirement: propose-adds-settings-sections <!-- base: ae00a3f6e3e7dc28 -->
`skills/propose/SKILL.md` SHALL tell the session that when a requirement its change adds or modifies adds a settings key, or a field or value of one, the same spec branch adds or edits the key's section in the guide's settings page, `docs/guide/settings.md`, saying what it does, what each choice trades off and where it may be set: the page is a spec path, so the code branch that builds the key can't carry it, and without the section that branch's land fails `config/settings-guide-covers-keys`. It says so in one paragraph of the skill's design steps, beside the glossary's, without moving an existing step. A test, a function over the skill's text, SHALL read the skill and fail naming the missing text.

#### Scenario: the skill says so
- GIVEN the plugin's `skills/propose/SKILL.md`
- WHEN a test reads it
- THEN it holds `docs/guide/settings.md` and `config/settings-guide-covers-keys` in one paragraph

#### Scenario: a skill without it
- GIVEN a `propose` `SKILL.md` that doesn't mention `docs/guide/settings.md`
- WHEN the check runs on it
- THEN it reports `docs/guide/settings.md`

#### Serves
- capability/configure-clm
- capability/propose-intent
