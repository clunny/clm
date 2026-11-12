---
component: plugin.build
---

## MODIFIED Requirements

### Requirement: skill-named-build <!-- base: 80390f85c145aaa9 -->
The plugin SHALL ship the skill that builds the approved specs as `skills/build/SKILL.md` and no `skills/implement/` directory, its heading and its description saying build. The description SHALL contain `approved specs` and `not compile`, so that nobody takes `/clm:build` for a compiler. No file under `plugin/` or `docs/guide/`, nor `CLAUDE.md`, nor `AGENTS.md` when it exists, nor `README.md`, SHALL name `/clm:implement` or `skills/implement/`, so the rule still covers this repository's instructions once `upgrade/migration-agents-md` has moved them from `CLAUDE.md` to `AGENTS.md`.

#### Scenario: the skill and its description
- GIVEN the plugin
- WHEN the tests run
- THEN `skills/build/SKILL.md` exists with a `description` holding `approved specs` and `not compile`, and `skills/implement/` doesn't exist

#### Scenario: an old mention left behind
- GIVEN a guide page or a skill that still says `/clm:implement`
- WHEN the tests run
- THEN the test fails, naming the file and the line

#### Scenario: the repository's instructions are read
- GIVEN the repository
- WHEN the tests run
- THEN the files they check for `/clm:implement` include `CLAUDE.md`, `README.md`, and `AGENTS.md` when it exists
