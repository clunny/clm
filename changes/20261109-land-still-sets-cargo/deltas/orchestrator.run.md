---
component: orchestrator.run
---

## ADDED Requirements
### Requirement: template-migration-fix
The `clm run` prompt SHALL tell the session that when a requirement it builds changes the text of a file `clm init` writes, and a requirement of `upgrade`, in the specs as applied, states the migration that carries that change and needs the requirement it builds, it writes that migration in its own branch, under `upgrade`'s `paths`, exactly as that requirement states it (its ID, `old`, `new`, `done` and note), since the upgrade corpus test fails a land of either half alone (`path-ownership`, `template-change-ships-its-migration`); that it adds no test citing that `upgrade` requirement, which `upgrade`'s own build adds, but brings any existing test of `upgrade`'s that the new migration makes fail (such as one expecting an earlier migration's rewrite to be the last) to what `upgrade`'s requirements as applied state; and that it names the migration under `## Fixes in other components` in its pull request body. The text SHALL be in every prompt, beside `removal-breaks-dependant`'s and `capability-tests-follow-rule`'s, which it doesn't change.

#### Scenario: prompt
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says that a requirement changing a file `clm init` writes, whose migration an `upgrade` requirement states and needing it, writes that migration in its own branch as stated, cites no `upgrade` requirement, fixes `upgrade`'s tests the migration makes fail, names it under `## Fixes in other components`, and contains `template-change-ships-its-migration`

#### Serves
- architecture/path-ownership
- architecture/upgrades-carry-migrations
