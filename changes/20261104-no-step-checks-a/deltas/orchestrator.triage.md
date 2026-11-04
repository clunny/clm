---
component: orchestrator.triage
---

## ADDED Requirements

### Requirement: draft-architecture-audit
The prompts `clm draft` and `clm draft --fold` write SHALL tell the session, in the paragraph that names the builder check, to launch the `architecture-auditor` subagent beside it, in the same message, with the Agent tool's `model` set to the alias of its own model, in the words `your own model`, since the audit is a design review (`model-per-launch`), giving it only the change ID and the path of its checkout (for the fold, asking it about the elements where two changes met, as the builder check is asked); to fix in the change each finding it agrees with and to write each one it leaves under `## Constraints` in `change.md`, with why, so the human approving the change or the fold reads it; that the audit advises and never gates (`spec-architecture-audit`); and that when the builder check or the audit changed the change, the session runs the round the paragraph already names after the builder check.

#### Scenario: draft prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that names the builder check contains `architecture-auditor`, `your own model`, `## Constraints` and `spec-architecture-audit`

#### Scenario: fold prompt
- GIVEN `origin/spec/20261027-a` and `origin/spec/20261027-b`, each holding its `changes/<id>/change.md` with `status: applied`
- WHEN `clm draft --fold 20261027-a 20261027-b` runs
- THEN the paragraph of the prompt file that names the builder check contains `architecture-auditor`, `your own model`, `## Constraints` and `spec-architecture-audit`

#### Serves
- capability/triage-unattended
- architecture/right-over-convenient

#### Needs
- plugin/architecture-auditor
