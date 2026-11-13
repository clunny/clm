---
component: plugin.triage
---

## ADDED Requirements
### Requirement: triage-web-answers
`skills/triage/SKILL.md` SHALL tell Claude that an `answered <slug> <writer> <at> <answer>` line under a `human` line is the human's answer given in the web app (`web-answers-as-ledger-records`), and `done` under a `human-task` item's line their word that they did the step: to show it with the item rather than ask for it again, unless two such lines for one item disagree, when it asks the human which holds; to add it to `.clm/answers.md` as `<slug>: <answer> (answered in the web app by <writer> at <at>)`; and to draft the item as it drafts a `human` line answered in the session.

#### Scenario: the skill takes web answers
- GIVEN the plugin's `skills/triage/SKILL.md`
- WHEN a test reads it
- THEN it contains `answered <slug> <writer> <at> <answer>` and `answered in the web app by <writer> at <at>`

#### Serves
- capability/follow-from-browser
- capability/triage-unattended

#### Needs
- orchestrator.triage/triage-web-answers
