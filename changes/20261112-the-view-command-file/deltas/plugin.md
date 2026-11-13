---
component: plugin
---

## MODIFIED Requirements

### Requirement: view-fallback-wording <!-- base: ac3807bc3e2f11c6 -->
`plugin/commands/view.md`, the text the model receives when no `clm-panel` hooks module answers `/clm:view`, SHALL NOT say that `clm-panel` isn't installed. It SHALL say that the panel either isn't installed or didn't load, and name `/plugin install clm-panel@clm` for the first case and `claude plugin validate` and `claude --debug` for the second, then go on to the same `clm status` and `clm view <name>` fallbacks as before, its names and argument hint being `status`, `queue`, `spec`, `flight` and `blockers`. With no name it SHALL have the model show `clm view queue`, the view the panel opens on, and SHALL NOT name `clm view flight`.

#### Scenario: installed but refused
- GIVEN `plugin/commands/view.md`
- WHEN a test reads it
- THEN it doesn't contain `isn't installed here`, and it contains `/plugin install clm-panel@clm`, `claude plugin validate`, `claude --debug` and `queue`

#### Scenario: no name shows the queue
- GIVEN `plugin/commands/view.md`
- WHEN a test reads it
- THEN it contains `clm view queue` and doesn't contain `clm view flight`

#### Serves
- capability/watch-beside-chat

#### Needs
- view.repo/view-queue
