---
component: orchestrator.run
base: {}
---

## MODIFIED Requirements

### Requirement: stays-in-its-component <!-- base: a0d6cc53026aa024 -->
The `clm run` prompt SHALL tell the session that its build changes only its own component's files: those its component's `tests` match, wherever they sit, and those its `paths` match that no other component's `tests` match (`path-ownership`, `components-own-their-tests`), apart from the two exceptions `capability-tests-follow-rule` and `template-migration-fix` state and the re-citing `pending-prompt` asks for in a file no component's `tests` match. It SHALL tell the session to put each new test in a file its component's `tests` match, or its `paths` when it sets no `tests`, and never to add a test to a file another component owns. It SHALL tell the session that when a requirement it builds can't be built within those files (it needs a file no glob of its component matches, an import its `depends_on` doesn't allow, or an edit to another component's code or tests, as when what it removes from or adds to its Contract breaks a component that still uses it), the session keeps every component building as it stands, leaves the requirement unbuilt, and adds a `question` backlog item whose `components` name the components concerned, with `blocks` naming the requirement and saying what spec change it waits on: a glob, a dependency, or another component's requirement that moves that component first (`coupled-changes-sequence`); and that it never moves an import, hides a file, edits a spec path or asks for a history rewrite to get past a check, and never adds or widens an exemption, which only a change a human reviews does (`temporary-exemptions`). It SHALL tell the session that a file of its component that an exemption names still holds every other rule, and that a build adds nothing to such a file that the exemption would excuse. For an `obsolete` requirement whose code another component still uses, it SHALL tell the session to remove the markers and tests, keep that code, with no `clm:remove-by` marker, and add a `deferred` item without `blocks`, whose `components` name both components and whose body names the kept code, the component that uses it, and that the code goes once that component no longer uses it (`contract-removal-needs`, `deferred-work-resolves-in-code`). The text SHALL be in every prompt, beside `pending-prompt`'s instruction to remove an obsolete requirement's code, which it qualifies and leaves in place.

#### Scenario: own files
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says the build changes only the files its component's `tests` and `paths` give it, names `components-own-their-tests`, `capability-tests-follow-rule` and `template-migration-fix`, and says never to add a test to a file another component owns

#### Scenario: no legal home
- GIVEN the prompt `clm run` builds for a component
- WHEN the tests read it
- THEN it says a requirement that needs a file, an import or an edit outside the component's files is left unbuilt with a `question` item whose `blocks` names it, names `coupled-changes-sequence`, and says never to move an import, hide a file, edit a spec path, ask for a history rewrite or add an exemption to get past a check, naming `temporary-exemptions`

#### Scenario: obsolete requirement
- GIVEN the prompt `clm run` builds for a component with no `obsolete` requirement
- WHEN the tests read it
- THEN it says to keep code another component still uses and to add a `deferred` item without `blocks`

#### Serves
- architecture/path-ownership
- capability/no-dead-weight
