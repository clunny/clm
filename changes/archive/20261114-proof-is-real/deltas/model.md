---
component: model
base: { contract: "00f408d32cfb9587" }
---

## MODIFIED Contract

Parses `specs/` and `changes/` (see docs/spec-format.md) into a validated spec graph of components, requirements, assets, ownership tree and dependency DAG, with a content hash per element. A component's assets are the files under `specs/<component path>/assets/` (`node-assets`). A requirement exposes the requirements of other components it needs built first, from its `#### Needs` block (`requirement-needs`). The graph exposes the component frontmatter keys that live changes declare in `frontmatter_keys`, and the check a component's frontmatter must pass given them, so `apply` validates a delta's frontmatter by the same rule (`declared-frontmatter-keys`). The model exposes its component parser and its check of the rules across components, both of which loading runs, so `apply` checks the graph a change would leave by the same rules (`graph-check`). A component may name its test files with `tests`, globs it owns wherever they sit (`test-ownership-keys`), and the rules its files are temporarily exempt from with `exemptions`, each naming the backlog item that ends it (`exemptions`). The model exposes its check of an `exemptions` value, which loading runs, so `init` judges the entries of a proposed component map by the same rule (`exemptions`). The graph exposes the root's `outside` globs, the code no component owns yet, absent when the root doesn't set the key; `check`'s path ownership reads them, and runs only when the key is set (`incremental-adoption`). It exposes the root's `wiring` globs the same way, the files every build may edit, which the gate's ownership rule reads, and which runs only when that key is set (`wiring-globs`, `gate/own-files`). The graph also holds the two human layers, the entries of `ARCHITECTURE.md` and `CAPABILITIES.md` (`human-layers`), as requirements with IDs `architecture/<slug>` and `capability/<slug>`, which `Graph::requirement` finds and `Graph::requirements` lists after every component's, and the heading each layer's file writes its entries under, which `rename` rewrites; `capability` and `architecture` are reserved component IDs (`reserved-ids`). A component's `serves`, and a requirement's `#### Serves` block, name the entries a rule serves; `Graph::effective_serves` gives a rule's (`serves`). The model loads from a directory on disk or from an in-memory set of files, with the same result, and builds for `wasm32-unknown-unknown` so the `web` Worker can use it.


## ADDED Requirements

### Requirement: wiring-globs
The root component's frontmatter MAY set `wiring` to a list of globs, written as a component's `paths` are: the files that wire every component into one product, such as a dispatch file, a workspace manifest and its lock file, which every component's build may edit whoever owns them (`own-files-at-the-gate`). Setting the key, even to `[]`, turns on the gate's ownership rule for the repository (`gate/own-files`). The model SHALL expose the root's `wiring` as absent when the key is absent, and otherwise as the list in the order written, empty for `[]`. It SHALL reject, as `model/wiring-globs` naming the file and `wiring`, a `wiring` key on any component but the root (judged by component ID when the graph loads), and a `wiring` whose value isn't a list of non-empty strings, a YAML null included. The model's frontmatter check (`optional-nodes`), which loading runs and `apply` runs on a delta, SHALL reject `wiring` on any component but `clm`, as loading does, and a `wiring` of the wrong shape on any component, the root included.

#### Scenario: globs on the root
- GIVEN `specs/SPEC.md` whose frontmatter has `wiring: ["Cargo.toml", "src/main.rs"]`
- WHEN the model loads `specs/`
- THEN the root's `wiring` is `Cargo.toml` then `src/main.rs`

#### Scenario: empty list
- GIVEN `specs/SPEC.md` whose frontmatter has `wiring: []`
- WHEN the model loads `specs/`
- THEN the root's `wiring` is present and empty, and loading reports nothing

#### Scenario: no key
- GIVEN `specs/SPEC.md` whose frontmatter has no `wiring`
- WHEN the model loads `specs/`
- THEN the root's `wiring` is absent and loading reports nothing

#### Scenario: on another component
- GIVEN `specs/a/SPEC.md` whose frontmatter has `wiring: ["src/**"]`
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/a/SPEC.md` and `wiring`

#### Scenario: bad value
- GIVEN `specs/SPEC.md` whose frontmatter has `wiring: "src/**"`, `wiring: [""]` or `wiring:` with no value
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/SPEC.md` and `wiring`

#### Serves
- architecture/path-ownership

## MODIFIED Requirements

### Requirement: parse-backlog <!-- base: fa4b09c88f9f0264 -->
The model SHALL load every `backlog/<slug>.md` as a backlog item with frontmatter `title` (required), `kind` (`idea|deferred|question|bug|human-task|workaround`, required), `components` (component IDs, default empty, which an item may still write under the old key, `component-keys`), `origin` (a change ID, optional), `priority` (`P1` to `P5`, default `P5`, `priority-caps`) `blocks` (requirement IDs `<component>/<slug>`, or human-layer entry IDs `architecture/<slug>` and `capability/<slug>`, default empty: the requirements that can't be built until the item is resolved, `requirement-blocks`) and `requirement` (`<component>/<slug>@<version>`, required on a `workaround` item and rejected on any other kind: the requirement a build found hard to build, at the version it built, `workaround-flags`), `issues` (a list of positive whole numbers, default empty: the host's issues the item was taken in from, `issues-enter-through-the-backlog`) and `alerts` (a list of positive whole numbers, default empty: the host's dependency alerts the item was taken in from, `dependency-alerts-through-intake`) and `verify_failures` (a list of commit IDs, each 40 lower-case hex digits, default empty: the red `verified` results on `main` the item was taken in from, `verify-after-land`) and `unproven` (a list of `<component>/<slug>@<version>`, the version 8 lower-case hex digits, default empty, accepted on a `bug` item alone: the requirements a mutation pass found unproven at that version, `mutation-after-land`), rejecting unknown keys, any other `priority` value, an `issues` or `alerts` entry that isn't a positive whole number, a `verify_failures` entry that isn't 40 lower-case hex digits, an `unproven` entry of another shape and an `unproven` on an item of another kind. An `unproven` entry MAY name a requirement that no longer exists or a version that is no longer current: it then says nothing, so the spec change that rewords or removes the requirement edits no item. The model SHALL expose each item's `unproven` in the order written. A `blocks` entry, or a workaround item's `requirement`, MAY name a human-layer entry that exists (`human-layers`), whose version is its block's. It SHALL reject a `workaround` item whose `requirement` names a requirement that doesn't exist, or whose version isn't the first 8 hex digits of that requirement's current content hash, so the spec change that removes or rewrites a requirement deletes its flags or re-records them at the new version. It SHALL reject an item naming an origin change that doesn't exist, a component that neither exists nor is created by a delta with `create: true` of a change outside the archive, or a `blocks` entry naming a requirement that neither exists nor is added, under `## ADDED Requirements`, by a delta for its component of a change outside the archive, so the spec PR that removes a requirement edits the items that block it, and the spec PR that adds a requirement may file the item that blocks it. A change counts whatever its `status`, as for `components`; a delta that failed to load adds nothing. A `kind` that is the word `plain-terms-renames` replaced with `human-task` or `workaround` SHALL load as that kind, so an item from before that rename, in the tree or in a commit whose history `effort` reads, still loads (`renamed-words-still-load`). A `human-task` item is a step only a person can take, such as creating an account or setting a secret. A `workaround` item is a build's flag that a requirement was hard to build; `clm health` counts them (`check.health`).

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

#### Scenario: taken in from a red run
- GIVEN `backlog/x.md` with `kind: bug` and `verify_failures` naming one 40-hex commit ID
- WHEN the backlog loads
- THEN item `x` is loaded with that commit in its verify failures, and loading reports nothing

#### Scenario: short commit refused
- GIVEN `backlog/x.md` with `kind: bug` and `verify_failures: ["abc123"]`
- WHEN the backlog loads
- THEN loading reports `backlog/x.md` and `verify_failures`

#### Scenario: taken in from alerts
- GIVEN `backlog/x.md` with `kind: bug` and `alerts: [4]`
- WHEN the model loads
- THEN item `x` is loaded with alerts `[4]` and no issues, and loading reports nothing

#### Scenario: an alert that isn't a number
- GIVEN `backlog/x.md` with `kind: bug` and `alerts: [0]`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `alerts`

#### Scenario: an issue that isn't a number
- GIVEN `backlog/x.md` with `kind: bug` and `issues: ["#73"]`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `issues`

#### Scenario: named unproven
- GIVEN `backlog/u.md` with `kind: bug` and `unproven: [a/x@0123abcd]`, `a/x` existing at another version
- WHEN the model loads it
- THEN the item holds that entry, and loading reports nothing

#### Scenario: unproven on another kind
- GIVEN `backlog/u.md` with `kind: idea` and `unproven: [a/x@0123abcd]`
- WHEN the model loads it
- THEN loading fails naming the file and `unproven`

#### Scenario: unproven without a version
- GIVEN `backlog/u.md` with `kind: bug` and `unproven: [a/x]`
- WHEN the model loads it
- THEN loading fails naming the file and `unproven`

#### Serves
- capability/keep-a-backlog
