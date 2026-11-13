---
component: orchestrator.run
---

## ADDED Requirements
### Requirement: prompt-question-choices
The `clm run` prompt SHALL tell the session that when it adds a `question` item that a short answer settles, it lists the answers that settle it under `choices` (two to five, each a short line, `web-answers-as-ledger-records`), so the human can answer it with a tap in the web app, and lists none for a question that needs explaining.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it names `choices` beside the `question` kind and says to list none for a question that needs explaining

#### Serves
- capability/follow-from-browser

#### Needs
- model/backlog-choices
