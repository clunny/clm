---
component: orchestrator.triage
---

## ADDED Requirements
### Requirement: triage-web-answers
`clm triage` SHALL print, right after each `human <slug> ...` line, one line `answered <slug> <writer> <at> <answer>` per open answer of the ledger fold over `origin`'s ledger refs and the clone's own (`ledger/answer-records`) that names that item, oldest first, and none for an item no open answer names. It SHALL print no `answered` line for an item that has no `human` line. Its other lines, their order and its exit code SHALL be unchanged by it.

#### Scenario: answered in the web app
- GIVEN an `origin` whose backlog holds question `q` and `human-task` `h`, and `origin/clm/ledger/alice/web` holding an `answered` record naming `q` with answer `yes` at `2026-11-12T09:00:00Z`
- WHEN `clm triage` runs
- THEN the line after `human q ...` is `answered q alice/web 2026-11-12T09:00:00Z yes`, and no `answered` line follows `h`'s line

#### Serves
- capability/follow-from-browser
- capability/triage-unattended

#### Needs
- ledger/answer-records
