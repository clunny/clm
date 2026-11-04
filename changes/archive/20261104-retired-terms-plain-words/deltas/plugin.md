---
component: plugin
---

## MODIFIED Requirements

### Requirement: skills-say-new-words <!-- base: 0989267b501a76ea -->
No file under `plugin/` SHALL hold, except in a line holding `clm:remove-by`, the word `plain-terms-renames` replaced with `unbuilt` as a word of its own; the word it replaced with `workaround` after `clm` and a space or as a backlog kind (written `kind: <word>`, or in backticks after the word kind), the plain English word keeping it; the word it replaced with `human-task` as a backlog kind (written `kind: <word>`, or in backticks after the word kind; the triage line `human` and the English word keep it); or the key it replaced with `human_burdens` inside backticks or before `: [`: the build moves every mention, not only those a requirement quotes. In particular `skills/init/SKILL.md` writes a new root component with `status: unbuilt`, and `skills/health/SKILL.md` names the `workaround` signal `clm health` prints. A test reads every file under `plugin/` and fails naming the file, the line and the old word.

#### Scenario: the shipped plugin
- GIVEN the repository
- WHEN the test reads `plugin/`
- THEN it finds none of the old words in those forms, a line using the old name of `workaround` as a plain English word passes, `skills/init/SKILL.md` holds `status: unbuilt`, and `skills/health/SKILL.md` names `workaround`

#### Serves
- architecture/plain-terms

#### Needs
- model/valid-frontmatter
- model/parse-changes
- model/parse-backlog
- check.health/requirement-signals
- check.health/workaround-command
- check.coverage/state-names
