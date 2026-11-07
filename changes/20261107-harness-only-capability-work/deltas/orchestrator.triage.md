---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: draft-code-fix-written-rule <!-- base: fb76285c130776b6 -->
The prompt `clm draft` writes SHALL tell the session that a `bug` is code-only, and fixed by component on `impl/<component>`, only when a requirement or scenario as written already states the behaviour the fix restores and the fix's test cites it; a bug whose behaviour no requirement states goes into the change, as a requirement or scenario the change adds, and its code is fixed when the component is built (`code-fix-needs-a-written-rule`). It SHALL tell the session too that a `bug` or `deferred` item whose work is harness-only, changing only the plumbing of the capability suite and nothing a capability test observes, is never a requirement of any component (`human-layer-proof`): when the Design that holds the harness stays true as written, the session fixes it in the draft on a branch of its own from `origin/main`, named outside `impl/`, `spec/`, `propose/` and `autosync/`, deleting the item in a commit whose trailer block holds `Resolves: backlog/<slug>`, lands it with `clm land` once `clm gate --base origin/main` prints `code`, deletes the pushed branch with `clm branch delete`, and takes the item out of `resolves` (`small-change-no-claim`); when that Design must change, the change edits the Design, keeps the item in `resolves` and adds a `kind: deferred` item for the code. The prompt says it in the paragraph that already describes code-only bugs, not in a new one.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the prompt file it writes contains `code-fix-needs-a-written-rule`, `as written` and `harness-only`

#### Serves
- capability/triage-unattended
- architecture/fold-small-defer-large
