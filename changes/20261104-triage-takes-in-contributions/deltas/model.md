---
component: model
---

## MODIFIED Requirements

### Requirement: parse-backlog <!-- base: 043adb6595864b3e -->
The model SHALL load every `backlog/<slug>.md` as a backlog item with frontmatter `title` (required), `kind` (`idea|deferred|question|bug|human-task|workaround`, required), `components` (component IDs, default empty, which an item may still write under the old key, `component-keys`), `origin` (a change ID, optional), `priority` (`P1` to `P5`, default `P5`, `priority-caps`) `blocks` (requirement IDs `<component>/<slug>`, or human-layer entry IDs `architecture/<slug>` and `capability/<slug>`, default empty: the requirements that can't be built until the item is resolved, `requirement-blocks`) and `requirement` (`<component>/<slug>@<version>`, required on a `workaround` item and rejected on any other kind: the requirement a build found hard to build, at the version it built, `workaround-flags`) and `issues` (a list of positive whole numbers, default empty: the host's issues the item was taken in from, `issues-enter-through-the-backlog`), rejecting unknown keys, any other `priority` value and an `issues` entry that isn't a positive whole number. A `blocks` entry, or a workaround item's `requirement`, MAY name a human-layer entry that exists (`human-layers`), whose version is its block's. It SHALL reject a `workaround` item whose `requirement` names a requirement that doesn't exist, or whose version isn't the first 8 hex digits of that requirement's current content hash, so the spec change that removes or rewrites a requirement deletes its flags or re-records them at the new version. It SHALL reject an item naming an origin change that doesn't exist, a component that neither exists nor is created by a delta with `create: true` of a change outside the archive, or a `blocks` entry naming a requirement that neither exists nor is added, under `## ADDED Requirements`, by a delta for its component of a change outside the archive, so the spec PR that removes a requirement edits the items that block it, and the spec PR that adds a requirement may file the item that blocks it. A change counts whatever its `status`, as for `components`; a delta that failed to load adds nothing. A `kind` that is the word `plain-terms-renames` replaced with `human-task` or `workaround` SHALL load as that kind, so an item from before that rename, in the tree or in a commit whose history `effort` reads, still loads (`renamed-words-still-load`). A `human-task` item is a step only a person can take, such as creating an account or setting a secret. A `workaround` item is a build's flag that a requirement was hard to build; `clm health` counts them (`check.health`).

#### Scenario: unknown node
- GIVEN `backlog/x.md` with `components: [ghost]`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `ghost`

#### Scenario: node a live change creates
- GIVEN `backlog/x.md` with `components: [n]`, no `specs/n/SPEC.md`, and `changes/c/` with `status: proposed` and a delta for `n` with `create: true`
- WHEN the model loads
- THEN item `x` is loaded with components `[n]`, and loading reports nothing

#### Scenario: node an archived change created
- GIVEN `backlog/x.md` with `components: [n]`, no `specs/n/SPEC.md`, and a delta for `n` with `create: true` only under `changes/archive/`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `n`

#### Scenario: human step
- GIVEN `backlog/x.md` with `kind: human-task`
- WHEN the model loads
- THEN item `x` is loaded with kind `human-task`

#### Scenario: the old kind names
- GIVEN `backlog/h.md` whose `kind` is the word `plain-terms-renames` replaced with `human-task`, and `backlog/workaround-a-x-1.md` whose `kind` is the word `plain-terms-renames` replaced with `workaround`, with `requirement: a/x@3fa2c9d1` at `a/x`'s version
- WHEN the model loads
- THEN `h` is loaded with kind `human-task` and `workaround-a-x-1` with kind `workaround`, and loading reports nothing

#### Scenario: item priority
- GIVEN `backlog/x.md` with `priority: P1`, `backlog/y.md` with no `priority`, and `backlog/z.md` with `priority: high`
- WHEN the model loads
- THEN `x` is loaded with priority `P1`, `y` with priority `P5`, and loading reports `backlog/z.md` and `high`

#### Scenario: blocks
- GIVEN `backlog/x.md` with `blocks: [a/x, a/y]` where both requirements exist, and `backlog/z.md` with `blocks: [a/ghost]`
- WHEN the model loads
- THEN `x` is loaded blocking `a/x` and `a/y`, and loading reports `backlog/z.md` and `a/ghost`

#### Scenario: blocks a requirement a change outside the archive adds
- GIVEN `backlog/x.md` with `blocks: [a/new]`, component `a` with no requirement `new`, and `changes/c/` with `status: proposed` and a delta for `a` whose `## ADDED Requirements` holds `### Requirement: new`
- WHEN the model loads
- THEN item `x` is loaded blocking `a/new`, and loading reports nothing

#### Scenario: blocks a requirement of a node a change outside the archive creates
- GIVEN `backlog/x.md` with `blocks: [n/y]`, no `specs/n/SPEC.md`, and `changes/c/` with `status: proposed` and a delta for `n` with `create: true` whose `## ADDED Requirements` holds `### Requirement: y`
- WHEN the model loads
- THEN item `x` is loaded blocking `n/y`, and loading reports nothing

#### Scenario: blocks a requirement only an archived change added
- GIVEN `backlog/x.md` with `blocks: [a/old]`, component `a` with no requirement `old`, and a delta for `a` adding `### Requirement: old` only under `changes/archive/`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `a/old`

#### Scenario: blocks a requirement a change only modifies
- GIVEN `backlog/x.md` with `blocks: [a/ghost]`, component `a` with no requirement `ghost`, and `changes/c/` with a delta for `a` whose `## MODIFIED Requirements` holds `### Requirement: ghost`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `a/ghost`

#### Scenario: blocks a requirement a rejected delta adds
- GIVEN `backlog/x.md` with `blocks: [a/new]`, component `a` with no requirement `new`, and `changes/c/` with a delta for `a` whose `## ADDED Requirements` holds `### Requirement: new` and whose `## MODIFIED Design` holds `### Requirement: z`
- WHEN the model loads
- THEN loading reports the delta's `model/misplaced-delta-text`, and reports `backlog/x.md` and `a/new`

#### Scenario: blocks a human-layer entry
- GIVEN `backlog/x.md` with `blocks: [capability/approve]`, and `CAPABILITIES.md` whose `## Capabilities` holds `### Capability: approve`
- WHEN the model loads
- THEN loading reports nothing

#### Scenario: workaround flag
- GIVEN `backlog/workaround-a-x-1.md` with `kind: workaround` and `requirement: a/x@3fa2c9d1`, where `a/x` exists and its version is `3fa2c9d1`
- WHEN the model loads
- THEN item `workaround-a-x-1` is loaded with kind `workaround` and requirement `a/x` at `3fa2c9d1`, and loading reports nothing

#### Scenario: flag at an old version
- GIVEN `backlog/workaround-a-x-1.md` with `kind: workaround` and `requirement: a/x@00000000`, where `a/x`'s version is `3fa2c9d1`
- WHEN the model loads
- THEN loading reports `backlog/workaround-a-x-1.md`, `a/x` and `3fa2c9d1`

#### Scenario: flag on a removed requirement
- GIVEN `backlog/workaround-a-x-1.md` with `requirement: a/x@3fa2c9d1` and component `a` with no requirement `x`
- WHEN the model loads
- THEN loading reports `backlog/workaround-a-x-1.md` and `a/x`

#### Scenario: flag without a requirement
- GIVEN `backlog/f.md` with `kind: workaround` and no `requirement`
- WHEN the model loads
- THEN loading reports `backlog/f.md` and `requirement`

#### Scenario: requirement on another kind
- GIVEN `backlog/i.md` with `kind: idea` and `requirement: a/x@3fa2c9d1`
- WHEN the model loads
- THEN loading reports `backlog/i.md` and `requirement`

#### Scenario: taken in from issues
- GIVEN `backlog/x.md` with `kind: bug` and `issues: [73, 91]`
- WHEN the model loads
- THEN item `x` is loaded with issues `[73, 91]`, and loading reports nothing

#### Scenario: an issue that isn't a number
- GIVEN `backlog/x.md` with `kind: bug` and `issues: ["#73"]`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `issues`

#### Serves
- capability/keep-a-backlog
