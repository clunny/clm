---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: draft-corrects-bug-components <!-- base: 9b7f374a6c2b5c97 -->
The prompt `clm draft` writes SHALL tell the session that a code-only bug whose fix lies in no component the item names has its `components` set on `spec/<id>` to the components whose `paths` hold the fix, is left out of `resolves` with `left <slug>: components corrected to <components>`, and is not fixed by that draft, since `clm claim --fix` takes only a component the item on `origin/main` names. It SHALL tell the session that a `bug` or `deferred` item whose work is test plumbing is treated the same way, as the exception to the branch of its own that `draft-code-fix-written-rule` gives test plumbing, when every file its fix changes belongs to no component the item names, a file belonging to a component whose `paths` hold it or whose requirement its text cites: the session sets the item's `components` on `spec/<id>` to the components those files belong to, leaves it with the same `left` line and opens no branch of its own for it, since the gate passes the deletion of an item by a `Resolves: backlog/<slug>` trailer only when the branch's code belongs to a component the item at the merge base names (`gate/backlog-additions`), and otherwise prints `mixed`. It says so in the paragraph that describes code-only bugs.

#### Scenario: prompt
- GIVEN untaken `bug` item `a` with `components: [check]`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `clm claim <component> --fix backlog/<slug>` also contains `components corrected to <components>`

#### Scenario: test plumbing under another component
- GIVEN untaken `bug` item `a` with `components: [check]`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `components corrected to <components>` also contains `belongs to no component the item names`, `gate/backlog-additions` and `mixed`
