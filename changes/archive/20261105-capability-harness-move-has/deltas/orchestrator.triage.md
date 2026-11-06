---
component: orchestrator.triage
---

## ADDED Requirements

### Requirement: draft-capability-test-fixes
The prompts `clm draft` and `clm draft --fold` write SHALL tell the session, in the paragraph that names the builder check, to ask its subagent which capability tests each requirement the change adds or modifies changes what they observe at the product's boundary, and that such work is neither a requirement of `capability`, which holds none of its own, nor a `#### Needs`, nor a backlog item: the requirement's own build updates those tests (`capability-tests-follow-rules`), so the session names them in that requirement's text; and that a requirement under which a `CAPABILITIES.md` entry as written no longer holds edits that entry in the change instead, a hand edit.

#### Scenario: draft prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `capability-tests-follow-rules` and `CAPABILITIES.md`

#### Scenario: fold prompt
- GIVEN `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN the paragraph of the prompt file that names the builder check contains `capability-tests-follow-rules` and `CAPABILITIES.md`

#### Serves
- capability/triage-unattended
- architecture/path-ownership
