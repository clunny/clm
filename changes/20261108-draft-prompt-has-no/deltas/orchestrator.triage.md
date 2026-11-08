---
component: orchestrator.triage
---

## MODIFIED Requirements

### Requirement: draft-code-fix-written-rule <!-- base: f48e94427237a7cd -->
The prompt `clm draft` writes SHALL tell the session that a `bug` is code-only, and fixed by component on `impl/<component>`, only when a requirement or scenario as written already states the behaviour the fix restores and the fix's test cites it; a bug whose behaviour no requirement states goes into the change, as a requirement or scenario the change adds, and its code is fixed when the component is built (`code-fix-needs-a-written-rule`). It SHALL say that an `ARCHITECTURE.md` entry, carried or the project's own, is not such a requirement, since a test never cites an entry (`human-layer-proof`): a bug whose behaviour only an architecture entry states is a bug no requirement states, unless its work is test plumbing. It SHALL tell the session too that a `bug` or `deferred` item whose work is test plumbing only is never a requirement of any component: harness-only work, changing only the plumbing of the capability suite and nothing a capability test observes, or the plumbing of a component's own test suite (its runner settings, the ports and servers its tests start, the paths they write), changing nothing any of its tests asserts. When the Design that holds that plumbing, if any, stays true as written, the session fixes it in the draft on a branch of its own from `origin/main`, named outside `impl/`, `spec/`, `propose/` and `autosync/`, with no claim and a test that cites no requirement, deleting the item in a commit whose trailer block holds `Resolves: backlog/<slug>`, lands it with `clm land` once `clm gate --base origin/main` prints `code`, deletes the pushed branch with `clm branch delete`, and takes the item out of `resolves` (`small-change-no-claim`); when that Design must change, the change edits the Design, keeps the item in `resolves` and adds a `kind: deferred` item for the code. The prompt says it in the paragraph that already describes code-only bugs, not in a new one.

#### Scenario: prompt
- GIVEN untaken `bug` item `a`
- WHEN `clm draft a` runs
- THEN the paragraph of the prompt file that contains `code-fix-needs-a-written-rule` also contains `as written`, `harness-only`, `test plumbing`, `own test suite` and `only an architecture entry states`

#### Serves
- capability/triage-unattended
- architecture/fold-small-defer-large
