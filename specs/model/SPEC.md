---
title: Model
status: active
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-model/**"]
successor: "20261021-walking-skeleton"
serves: [capability/propose-intent, capability/catch-drift, architecture/deterministic-enforcement]
---

## Contract
Parses `specs/` and `changes/` (see docs/spec-format.md) into a validated spec graph of components, requirements, assets, ownership tree and dependency DAG, with a content hash per element. A component's assets are the files under `specs/<component path>/assets/` (`node-assets`). A requirement exposes the requirements of other components it needs built first, from its `#### Needs` block (`requirement-needs`). The graph exposes the component frontmatter keys that live changes declare in `frontmatter_keys`, and the check a component's frontmatter must pass given them, so `apply` validates a delta's frontmatter by the same rule (`declared-frontmatter-keys`). The model exposes its component parser and its check of the rules across components, both of which loading runs, so `apply` checks the graph a change would leave by the same rules (`graph-check`). The graph exposes the root's `outside` globs, the code no component owns yet, absent when the root doesn't set the key; `check`'s path ownership reads them, and runs only when the key is set (`incremental-adoption`). The graph also holds the two human layers, the entries of `ARCHITECTURE.md` and `CAPABILITIES.md` (`human-layers`), as requirements with IDs `architecture/<slug>` and `capability/<slug>`, which `Graph::requirement` finds and `Graph::requirements` lists after every component's, and the heading each layer's file writes its entries under, which `rename` rewrites; `capability` and `architecture` are reserved component IDs (`reserved-ids`). A component's `serves`, and a requirement's `#### Serves` block, name the entries a rule serves; `Graph::effective_serves` gives a rule's (`serves`). The model loads from a directory on disk or from an in-memory set of files, with the same result, and builds for `wasm32-unknown-unknown` so the `web` Worker can use it.

## Design
TBD.

## Requirements

### Requirement: parse-node-specs
The model SHALL load every `specs/**/SPEC.md` that isn't inside an `assets/` directory as a component whose ID is its directory path under `specs/`, dot-separated, with `specs/SPEC.md` as root `clm`.

#### Scenario: nested node
- GIVEN `specs/check/drift/SPEC.md`
- WHEN the model loads `specs/`
- THEN a component `check.drift` exists with parent `check`

#### Scenario: spec inside assets
- GIVEN `specs/a/SPEC.md` and `specs/a/assets/SPEC.md`
- WHEN the model loads `specs/`
- THEN component `a` exists, no component `a.assets` exists, and `SPEC.md` is one of `a`'s assets

### Requirement: valid-frontmatter
The model SHALL reject a component whose frontmatter is missing `title`, `status` or `owners`, has an unknown key, or has a `status` outside `unbuilt|active|deprecated`. A `status` that is the word `plain-terms-renames` replaced with `unbuilt` SHALL load as `unbuilt`, so a tree from before that rename still loads, as the gate loads a merge base's (`renamed-words-still-load`). A key is unknown when the model doesn't know it and no live change declares it in `frontmatter_keys` (`parse-changes`, `declared-frontmatter-keys`). A live change is one outside `changes/archive/` whose status isn't `abandoned`. A declaration names a top-level key only. A change whose `change.md` doesn't parse declares nothing. A key the model doesn't know but a live change declares SHALL be accepted on any component with any value, which the parsed frontmatter leaves out; the component's frontmatter hash still covers its line. A key the model knows SHALL be validated as before, whether or not a change declares it. The model SHALL accept and ignore `significant`, whatever its value, a key components set while review had tiers (`yes-no-review`), and `capabilities`, whatever its value, a key nothing read and that `serves` replaces (`human-layers`), so a tree from before still loads, as the gate loads a merge base's. `serves` is a key the model knows (`serves`).

#### Scenario: legacy significant
- GIVEN a SPEC.md whose frontmatter has `significant: true`
- WHEN the model loads `specs/`
- THEN loading reports nothing

#### Scenario: legacy capabilities
- GIVEN a SPEC.md whose frontmatter has `capabilities: [x]`
- WHEN the model loads `specs/`
- THEN loading reports nothing

#### Scenario: the old status name
- GIVEN a SPEC.md whose `status` is the word `plain-terms-renames` replaced with `unbuilt`
- WHEN the model loads `specs/`
- THEN loading reports nothing, and the component's status is `unbuilt`

#### Scenario: bad status
- GIVEN a SPEC.md with `status: done`
- WHEN the model loads it
- THEN loading fails naming the file and the field

#### Scenario: unknown key
- GIVEN a SPEC.md whose frontmatter has `colour: blue`, and no live change declaring `colour`
- WHEN the model loads `specs/`
- THEN loading fails naming the file and `colour`

#### Scenario: key a live change declares
- GIVEN a SPEC.md whose frontmatter has `colour: blue`, and `changes/x/change.md` with `status: applied` and `frontmatter_keys: [colour]`
- WHEN the model loads `specs/` and `changes/`
- THEN loading reports nothing, the component's parsed frontmatter is as it would be without the `colour` line, and its frontmatter hash differs from that of the same file without the line

#### Scenario: declaration no longer live
- GIVEN the same SPEC.md, and the change declaring `colour` archived as `changes/archive/x/`, or outside the archive with `status: abandoned`
- WHEN the model loads `specs/` and `changes/`
- THEN loading fails naming the file and `colour`

#### Scenario: declared key the model knows
- GIVEN a SPEC.md with `optional: maybe`, and a live change with `frontmatter_keys: [optional]`
- WHEN the model loads `specs/` and `changes/`
- THEN loading fails naming the file and `optional`, as it does with no declaration

### Requirement: dependency-dag
The model SHALL reject `depends_on` entries naming nonexistent components, and any dependency cycle.

#### Scenario: cycle
- GIVEN `a` depends on `b` and `b` depends on `a`
- WHEN the model loads `specs/`
- THEN loading fails and names the cycle

### Requirement: unique-requirement-ids
The model SHALL reject duplicate requirement slugs within a component, and requirements with no scenario.

#### Scenario: duplicate slug
- GIVEN a component with two `### Requirement: foo` blocks
- WHEN the model loads it
- THEN loading fails naming `<component>/foo`

### Requirement: content-hash
The model SHALL give each requirement, contract, design and frontmatter a hash that does not change when only trailing whitespace or line endings change, and always changes when any other text changes, a requirement's `#### Serves` block included. A requirement's *version* SHALL be the first 8 hex digits of its hash, as `clm hashes <component>` prints it, which is what a test marker cites (`versioned-markers`), and SHALL leave out its `#### Serves` block (`serves`), since what a rule serves isn't what the code must do, and `serves` names entries without versions. So a requirement's hash SHALL be the first 8 hex digits of the content hash of its block less its `#### Serves` block (the lines from that block's heading to its end), followed by the last 8 hex digits of the content hash of its whole block (`serves-in-the-base`): a requirement with no `#### Serves` block hashes as its block's content hash, and an edit to only the block moves the hash, which deltas record as their bases and `clm rebase` compares across trees, but not the version.

#### Scenario: whitespace-insensitive
- GIVEN a requirement and a copy differing only in CRLF vs LF
- WHEN both are hashed
- THEN the hashes are equal

#### Scenario: version
- GIVEN a requirement whose hash `clm hashes` prints as `0123456789abcdef`
- WHEN the model gives its version
- THEN it is `01234567`

#### Scenario: serves block
- GIVEN a requirement and a copy differing only by a `#### Serves` block
- WHEN both are hashed
- THEN their versions are equal and their hashes differ, and the one without the block hashes as its block's content hash

#### Serves
- capability/catch-drift
- architecture/hash-based-staleness

### Requirement: parse-changes
The model SHALL load every `changes/<id>/` and `changes/archive/<id>/` directory's `change.md`, `deltas/*.md` and `assets/<node-id>/**`. It SHALL reject a `change.md` whose frontmatter lacks a valid `status` (`proposed|approved|applied|implementing|done|abandoned`), or has keys other than that plus `human_burdens` (a list of short lines, default empty: what the change spends of the human's money or time, or can't undo, `gate/needs-review`), `author`, `from` (the backlog item it was promoted from), `resolves` (a list of backlog slugs the change resolves), `priority` (`P1` to `P5`, default `P5`, `priority-caps`) `frontmatter_keys` (a list of component frontmatter keys the change adds, default empty, `declared-frontmatter-keys`) and `renames` (the renames the change records, default empty, `rename-records`). It SHALL reject any other `priority` value, and a `frontmatter_keys` entry that isn't a lowercase key name (a lowercase letter, then lowercase letters, digits and `_`). It SHALL read the key that is the word `plain-terms-renames` replaced with `human_burdens` as `human_burdens`, on a live change and an archived one, since an archived change keeps the keys it was written with (`archive-immutable`, `renamed-words-still-load`), and SHALL reject a `change.md` that sets both, naming both keys. It SHALL also accept and ignore `after` and `prefer_after`, which changes recorded while changes were ordered against each other; nothing reads them any more (`node-reconciliation`), and archived changes keep theirs (`archive-immutable`). It SHALL reject `gate` on a live change, naming the key, and accept and ignore it on an archived one, which recorded the change's review tier before review became yes or no (`yes-no-review`). Files in a change's directory other than `change.md`, `deltas/*.md` and `assets/<node-id>/**`, a `plan.md` among them, are not parsed. A `resolves` entry MAY name a backlog item that no longer exists, since the build that fixes an item deletes it before the change is archived (`task-deletes-fixed-items`); `gate/resolves-named-items` catches an entry that never named one. It SHALL reject a delta whose frontmatter names no component, by `component` or by the old key (`component-keys`), that sets both `create: true` and `remove: true`, that sets `remove: true` and has a section other than `REMOVED Requirements` or `REMOVED Assets` or whose change holds files under `assets/<node-id>/` for its component, or that has a section other than `Frontmatter`, `Contract`, `Design`, `MODIFIED Frontmatter`, `MODIFIED Contract`, `MODIFIED Design`, `ADDED`/`MODIFIED`/`REMOVED Requirements` or `REMOVED Assets`. Text in a frontmatter section outside its ```` ```yaml ```` fence is rationale and is ignored.

#### Scenario: unknown delta section
- GIVEN a delta with a `## RENAMED Requirements` section
- WHEN the model loads `changes/`
- THEN loading reports the file and the section name

#### Scenario: create and remove
- GIVEN a delta for `a` with `create: true` and `remove: true`
- WHEN the model loads `changes/`
- THEN loading reports the file, `create` and `remove`

#### Scenario: removal with a modified section
- GIVEN a delta for `a` with `remove: true` and a `## MODIFIED Design` section
- WHEN the model loads `changes/`
- THEN loading reports the file and the section name

#### Scenario: archived change
- GIVEN `changes/archive/x/change.md` with `status: done`
- WHEN the model loads `changes/`
- THEN change `x` is loaded and marked as archived

#### Scenario: burdens
- GIVEN `changes/x/change.md` with `human_burdens: [a paid model run on every land]` and `changes/y/change.md` with no `human_burdens`
- WHEN the model loads `changes/`
- THEN `x`'s burdens are that one line and `y`'s are empty

#### Scenario: the old burdens key
- GIVEN `changes/archive/w/change.md` whose key the word `plain-terms-renames` replaced with `human_burdens` holds `[a paid model run]`, and `changes/x/change.md` setting both that key and `human_burdens`
- WHEN the model loads `changes/`
- THEN `w`'s burdens are that one line, and loading reports `changes/x/change.md` naming both keys

#### Scenario: gate on a live change
- GIVEN `changes/x/change.md` and `changes/archive/w/change.md`, each with `gate: HARD`
- WHEN the model loads `changes/`
- THEN loading reports `changes/x/change.md` naming `gate`, and `w` is loaded

#### Scenario: resolves a deleted item
- GIVEN `changes/x/change.md`, outside the archive, with `resolves: [gone]` and no `backlog/gone.md`
- WHEN the model loads `changes/`
- THEN change `x` is loaded with `resolves` `[gone]`, and loading reports nothing

#### Scenario: order keys ignored
- GIVEN `changes/x/change.md` with `prefer_after: [y]` and `changes/archive/w/change.md` with `after: [nope]`, where no change `y` or `nope` exists
- WHEN the model loads `changes/`
- THEN both changes are loaded and loading reports nothing

#### Scenario: change assets
- GIVEN `changes/x/assets/a/m.html` and a delta for `a` with `## REMOVED Assets` listing `old.png`
- WHEN the model loads `changes/`
- THEN change `x` holds `m.html` as a new or modified asset of `a`, and `old.png` as a removed one

#### Scenario: change priority
- GIVEN `changes/x/change.md` with `priority: P2`, `changes/y/change.md` with no `priority`, and `changes/z/change.md` with `priority: P0`
- WHEN the model loads `changes/`
- THEN `x` is loaded with priority `P2`, `y` with priority `P5`, and loading reports `z`'s `change.md` and `P0`

#### Scenario: declared frontmatter keys
- GIVEN `changes/x/change.md` with `frontmatter_keys: [colour, size_hint]`, `changes/y/change.md` with no `frontmatter_keys`, and `changes/z/change.md` with `frontmatter_keys: [Colour]`
- WHEN the model loads `changes/`
- THEN `x` is loaded declaring `colour` and `size_hint`, `y` declaring none, and loading reports `z`'s `change.md` and `Colour`

#### Scenario: renames
- GIVEN `changes/x/change.md` with `renames` holding one `term: sprocket` entry with `to: cogs`, and `changes/y/change.md` with no `renames`
- WHEN the model loads `changes/`
- THEN `x` is loaded with that one rename and `y` with none

### Requirement: parse-backlog
The model SHALL load every `backlog/<slug>.md` as a backlog item with frontmatter `title` (required), `kind` (`idea|deferred|question|bug|human-task|workaround`, required), `components` (component IDs, default empty, which an item may still write under the old key, `component-keys`), `origin` (a change ID, optional), `priority` (`P1` to `P5`, default `P5`, `priority-caps`) `blocks` (requirement IDs `<component>/<slug>`, or human-layer entry IDs `architecture/<slug>` and `capability/<slug>`, default empty: the requirements that can't be built until the item is resolved, `requirement-blocks`) and `requirement` (`<component>/<slug>@<version>`, required on a `workaround` item and rejected on any other kind: the requirement a build found hard to build, at the version it built, `workaround-flags`), `issues` (a list of positive whole numbers, default empty: the host's issues the item was taken in from, `issues-enter-through-the-backlog`) and `alerts` (a list of positive whole numbers, default empty: the host's dependency alerts the item was taken in from, `dependency-alerts-through-intake`) and `verify_failures` (a list of commit IDs, each 40 lower-case hex digits, default empty: the red `verified` results on `main` the item was taken in from, `verify-after-land`), rejecting unknown keys, any other `priority` value, an `issues` or `alerts` entry that isn't a positive whole number and a `verify_failures` entry that isn't 40 lower-case hex digits. A `blocks` entry, or a workaround item's `requirement`, MAY name a human-layer entry that exists (`human-layers`), whose version is its block's. It SHALL reject a `workaround` item whose `requirement` names a requirement that doesn't exist, or whose version isn't the first 8 hex digits of that requirement's current content hash, so the spec change that removes or rewrites a requirement deletes its flags or re-records them at the new version. It SHALL reject an item naming an origin change that doesn't exist, a component that neither exists nor is created by a delta with `create: true` of a change outside the archive, or a `blocks` entry naming a requirement that neither exists nor is added, under `## ADDED Requirements`, by a delta for its component of a change outside the archive, so the spec PR that removes a requirement edits the items that block it, and the spec PR that adds a requirement may file the item that blocks it. A change counts whatever its `status`, as for `components`; a delta that failed to load adds nothing. A `kind` that is the word `plain-terms-renames` replaced with `human-task` or `workaround` SHALL load as that kind, so an item from before that rename, in the tree or in a commit whose history `effort` reads, still loads (`renamed-words-still-load`). A `human-task` item is a step only a person can take, such as creating an account or setting a secret. A `workaround` item is a build's flag that a requirement was hard to build; `clm health` counts them (`check.health`).

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

#### Serves
- capability/keep-a-backlog

### Requirement: parse-decisions
The model SHALL load every `docs/decisions/<name>.md` as a decision with frontmatter `title`, `date`, `status` (`accepted|superseded`) and `superseded_by`, rejecting unknown keys. `superseded_by` SHALL be present exactly when `status` is `superseded`, and SHALL name an existing decision.

#### Scenario: dangling supersession
- GIVEN a decision with `status: superseded` and `superseded_by: ghost`
- WHEN the model loads
- THEN loading reports the decision file and `ghost`

### Requirement: node-assets
The model SHALL load every file under `specs/<component path>/assets/` as an asset of that component, keyed by its path relative to `assets/`, with a content hash: `content_hash` of its text when it is UTF-8, otherwise the first 8 bytes of the SHA-256 of its bytes, in hex.

#### Scenario: asset hash
- GIVEN `specs/a/assets/mockups/m.html` with text `<p>x</p>`
- WHEN the model loads `specs/`
- THEN component `a` has asset `mockups/m.html` with hash `content_hash("<p>x</p>")`

### Requirement: load-from-files
The model SHALL read, from a directory, every file under `specs/`, `changes/`, `backlog/` and `docs/decisions/`, and the root files `ARCHITECTURE.md` and `CAPABILITIES.md` when they exist (`human-layers`). It SHALL load the graph from an in-memory map of repo-relative paths to file contents (`load_files`), giving the same graph, assets included, and the same violations as `load` gives for a directory holding those files.

#### Scenario: same as disk
- GIVEN a directory with a valid component with an asset, an invalid component, a change, an `ARCHITECTURE.md` and a `CAPABILITIES.md` with entries, and a map holding the same paths and contents
- WHEN the graph is loaded from each
- THEN the two graphs and the two violation lists are equal

#### Serves
- capability/follow-from-browser
- capability/catch-drift

### Requirement: builds-for-wasm
The `clm-model` crate SHALL build for `wasm32-unknown-unknown`.

#### Scenario: wasm build
- GIVEN the workspace with the `wasm32-unknown-unknown` target installed
- WHEN `cargo build -p clm-model --target wasm32-unknown-unknown` runs
- THEN it succeeds

#### Serves
- capability/follow-from-browser

### Requirement: optional-nodes
A component's frontmatter MAY set `optional` to `true` or `false`, and its absence SHALL mean `false`. A component is *optional* when it or any of its ancestors sets `optional: true`, and *required* otherwise. The model SHALL reject a component whose `optional` is any other value, naming the file and the field, and SHALL reject `optional: true` on the root component. The model SHALL expose one check of a component's frontmatter, which takes the component's ID (`declared-frontmatter-keys`): it runs the rules the frontmatter's text alone decides (`valid-frontmatter`, `size-budget`, the value of `optional` and the shape of `outside`) and then, when they pass, the root rules (`optional: true` on the root, and `outside` on any other component, `outside-globs`), and loading SHALL run it for every component, so `apply` refuses a delta's frontmatter by exactly the rules loading uses. The model SHALL expose no frontmatter check that takes no component ID, so no caller can check a component's frontmatter by fewer rules than loading uses. The root rules SHALL stay violations that keep the component loaded with its parsed frontmatter, as now, and SHALL run only once the frontmatter's other rules pass, so a bad `optional` value or any other frontmatter error is reported alone, as now. It SHALL reject a `depends_on` entry from a required component to an optional one, naming both components, so CLM works without its optional parts (`web-optional`). It SHALL expose whether each component is optional.

#### Scenario: bad value
- GIVEN a SPEC.md whose frontmatter has `optional: maybe`
- WHEN the model loads it
- THEN loading fails naming the file and `optional`

#### Scenario: required depends on optional
- GIVEN component `a` with `optional: true` and component `b`, without the flag, with `depends_on: [a]`
- WHEN the model loads `specs/`
- THEN loading fails naming `b` and `a`

#### Scenario: optional depends on required
- GIVEN component `a` with `optional: true` and `depends_on: [b]`, and component `b` without the flag
- WHEN the model loads `specs/`
- THEN loading succeeds, `a` is optional and `b` is required

#### Scenario: inherited from an ancestor
- GIVEN component `a` with `optional: true`, its child `a.x` without the flag, and component `b` with `depends_on: [a.x]`
- WHEN the model loads `specs/`
- THEN `a.x` is optional, and loading fails naming `b` and `a.x`

#### Scenario: optional root
- GIVEN `specs/SPEC.md` with `optional: true`
- WHEN the model loads `specs/`
- THEN loading fails naming the file and `optional`

#### Scenario: root rule in the exposed check
- GIVEN frontmatter text that is valid apart from `optional: true`
- WHEN the model's frontmatter check runs on it for component `clm`, and again for component `a`
- THEN the first reports one violation under `model/optional-nodes` naming `optional` and still gives the parsed frontmatter, and the second passes

#### Scenario: other rules for any node
- GIVEN frontmatter text that is valid apart from `optional: maybe`, and again apart from an undeclared key `shape: round`
- WHEN the model's frontmatter check runs on each for component `a`
- THEN each fails with one violation naming `specs/a/SPEC.md` and the key

#### Serves
- architecture/web-optional
- architecture/deterministic-enforcement

### Requirement: requirement-needs
A requirement MAY hold one `#### Needs` block among its scenarios: a bullet list of requirement IDs `<component>/<slug>`, the requirements of other components that must be current before it can be built (`requirement-needs`). The model SHALL expose each requirement's needs. The block runs from its heading to the next `###` or `####` heading; blank lines in it are ignored, and headings inside code fences don't count, as for scenarios. The model SHALL report, as violations that keep the component loaded (as `dependency-dag` does): a need naming no existing requirement, so the spec PR that removes a requirement edits what needed it; a need naming a human-layer entry (`human-layers`), which isn't built in order with rules; a need naming a requirement of the same component, since order within a component is the build's; a need from a required component's requirement to an optional component's (`optional-nodes`), so CLM still works without its optional parts; a malformed block (a line that isn't a `- <component>/<slug>` bullet, a second `#### Needs` in one requirement, or one outside any requirement); and a cycle of needs, naming the requirements on it. Needs of different requirements of two components in opposite directions are not a cycle. The block is part of the requirement's text, so adding or removing a need changes its version (`content-hash`).

#### Scenario: need parsed
- GIVEN `specs/a/SPEC.md` whose requirement `x` has a `#### Needs` block with `- b/y`, and `specs/b/SPEC.md` with requirement `y`
- WHEN the model loads `specs/`
- THEN `a/x` needs `b/y`, and loading reports nothing

#### Scenario: dangling need
- GIVEN `a/x` with `#### Needs` naming `b/ghost`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md`, `a/x` and `b/ghost`

#### Scenario: need on a human-layer entry
- GIVEN `a/x` with `#### Needs` naming `architecture/git-is-truth`, an entry of `ARCHITECTURE.md`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md`, `a/x` and `architecture/git-is-truth`

#### Scenario: same node
- GIVEN `a/x` with `#### Needs` naming `a/y`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md` and `a/y`

#### Scenario: cycle
- GIVEN `a/x` needing `b/y` and `b/y` needing `a/x`
- WHEN the model loads `specs/`
- THEN loading reports `a/x` and `b/y`

#### Scenario: opposite directions
- GIVEN `a/x` needing `b/y` and `b/z` needing `a/w`
- WHEN the model loads `specs/`
- THEN both graphs load and loading reports nothing

#### Scenario: required needs optional
- GIVEN component `b` with `optional: true`, component `a` without it, and `a/x` needing `b/y`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md`, `a/x` and `b/y`

#### Scenario: malformed block
- GIVEN `a/x` with a `#### Needs` block whose line is `needs b/y`
- WHEN the model loads `specs/`
- THEN loading reports `specs/a/SPEC.md` and `a/x`

#### Serves
- capability/build-from-spec
- architecture/needs-alone-order

### Requirement: size-budget
A component's frontmatter MAY set `size_budget` to a mapping whose keys `spec_tokens` and `code_tokens` are each optional and each a positive integer: the component's own budget, in place of `config`'s `component_size`, for that count (`node-size-budgets`). The model SHALL reject a component whose `size_budget` isn't such a mapping, has another key, or has a value that isn't a positive integer (zero, negative, a float or a string), with a message naming the file and `size_budget` itself, not only a parser error, and SHALL expose each key on the component, absent when the frontmatter doesn't set it.

#### Scenario: budget set
- GIVEN a SPEC.md whose frontmatter has `size_budget: { spec_tokens: 30000 }`
- WHEN the model loads it
- THEN the component's spec budget is 30000 and its code budget is absent

#### Scenario: no budget
- GIVEN a SPEC.md whose frontmatter has no `size_budget`
- WHEN the model loads it
- THEN both budgets are absent

#### Scenario: bad value
- GIVEN a SPEC.md whose frontmatter has `size_budget: { spec_tokens: lots }`
- WHEN the model loads it
- THEN loading fails naming the file and `size_budget`

#### Scenario: unknown key
- GIVEN a SPEC.md whose frontmatter has `size_budget: { lines: 500 }`
- WHEN the model loads it
- THEN loading fails naming the file and `size_budget`

#### Serves
- architecture/slim-components
- capability/know-health

### Requirement: requirement-text-under-a-heading
The model SHALL reject a delta whose `## ADDED Requirements` or `## MODIFIED Requirements` section holds a non-blank line before the section's first `### Requirement:` heading outside a code fence, or holds non-blank lines and no such heading at all. Every non-blank line counts, fence markers and HTML comments included. Each such section SHALL be one `model/requirement-text-under-a-heading` violation, whose path is the delta's file and whose message names the section and the 1-based line number in the file of the first such line, and the delta SHALL not be loaded. A `## REMOVED Requirements` section MAY hold text before its first heading, which is rationale and is ignored, as text after a removed requirement's heading is.

#### Scenario: modified text with no heading
- GIVEN a delta for `a` whose `## MODIFIED Requirements` section holds a requirement's body and a `#### Scenario:` but no `### Requirement:` heading
- WHEN the model loads `changes/`
- THEN loading reports the delta's file, `MODIFIED Requirements` and the line number of the body's first line

#### Scenario: added text before the first heading
- GIVEN a delta for `a` whose `## ADDED Requirements` section holds one line of text and then `### Requirement: x`
- WHEN the model loads `changes/`
- THEN loading reports the delta's file, `ADDED Requirements` and that line's number

#### Scenario: removal rationale
- GIVEN a delta for `a` whose `## REMOVED Requirements` section holds a line saying where the requirements moved and then `### Requirement: x <!-- base: <hash> -->`
- WHEN the model loads `changes/`
- THEN loading reports nothing for the delta, and it removes `x`

#### Scenario: blank lines
- GIVEN a delta for `a` whose `## MODIFIED Requirements` heading is followed by blank lines and then `### Requirement: x <!-- base: <hash> -->`
- WHEN the model loads `changes/`
- THEN loading reports nothing for the delta

### Requirement: misplaced-delta-text
The model SHALL reject a delta that holds a non-blank line after its frontmatter's closing `---` and before its first section heading (its first `## ` line outside a code fence), or a `### Requirement:` heading, outside a code fence, in a section other than `## ADDED Requirements`, `## MODIFIED Requirements` or `## REMOVED Requirements`. Every non-blank line before the first section counts, fence markers and HTML comments included, and a delta with no section at all and a non-blank line after its frontmatter is rejected the same way. A `### Requirement:` heading counts in every other section, the frontmatter sections' rationale included; one inside a code fence is text and doesn't count. The text before the first section SHALL be one `model/misplaced-delta-text` violation, and each section holding such a heading SHALL be one more, however many it holds; each violation's path is the delta's file and its message names the 1-based line number in the file of the first such line and, for a heading, the section that holds it; and the delta SHALL not be loaded. The rule reports independently of the delta's other rules, so a section that another rule also rejects, such as an unknown section or one a `remove: true` delta may not hold, is reported by both. The rule is checked on every delta of a change outside the archive; a delta under `changes/archive/<id>/` SHALL be loaded as it was when the change landed, its text before the first section ignored as before, since the archive is never edited (`archive-immutable`).

#### Scenario: requirement under a design section
- GIVEN a delta for `a` whose `## MODIFIED Design` section holds a paragraph and then `### Requirement: x` with a `#### Scenario:`
- WHEN the model loads `changes/`
- THEN loading reports `model/misplaced-delta-text`, the delta's file, `MODIFIED Design` and the line number of `### Requirement: x`, and the delta isn't loaded

#### Scenario: requirement under a contract section
- GIVEN a delta for `a` with `create: true` whose `## Contract` section holds `### Requirement: x` and whose `## Design` section holds `### Requirement: y`
- WHEN the model loads `changes/`
- THEN loading reports two `model/misplaced-delta-text` violations, one naming `Contract` and `x`'s line, one naming `Design` and `y`'s line

#### Scenario: two headings in one section
- GIVEN a delta for `a` whose `## MODIFIED Design` section holds `### Requirement: x` and later `### Requirement: y`
- WHEN the model loads `changes/`
- THEN loading reports one `model/misplaced-delta-text` violation, naming `MODIFIED Design` and `x`'s line

#### Scenario: text before the first section
- GIVEN a delta for `a` with one line of text after its frontmatter and then `## ADDED Requirements` holding `### Requirement: x`
- WHEN the model loads `changes/`
- THEN loading reports `model/misplaced-delta-text`, the delta's file and that line's number, and the delta isn't loaded

#### Scenario: heading in a code fence
- GIVEN a delta for `a` whose `## MODIFIED Design` section holds a ```` ```markdown ```` fence containing `### Requirement: x`
- WHEN the model loads `changes/`
- THEN loading reports nothing for the delta, and its design holds the fence

#### Scenario: blank lines before the first section
- GIVEN a delta for `a` with blank lines after its frontmatter and then `## ADDED Requirements` holding `### Requirement: x`
- WHEN the model loads `changes/`
- THEN loading reports nothing for the delta, and it adds `x`

#### Scenario: archived delta with text before its first section
- GIVEN an archived change `changes/archive/x/` whose delta for `a` has a paragraph after its frontmatter and then `## MODIFIED Design`
- WHEN the model loads `changes/`
- THEN loading reports nothing for the delta, and change `x` is loaded and marked as archived

### Requirement: outside-globs
The root component's frontmatter MAY set `outside` to a list of globs, written as a component's `paths` are: the code in the repository that no component owns yet, while it is adopted a part at a time (`incremental-adoption`). Setting the key, even to `[]`, turns on `check`'s path ownership for the repository (`check/unowned-files`). The model SHALL expose the root's `outside` as absent when the key is absent, and otherwise as the list in the order written, empty for `[]`. It SHALL reject, as `model/outside-globs` naming the file and `outside`, an `outside` key on any component but the root (judged by component ID when the graph loads), and an `outside` whose value isn't a list of non-empty strings, a YAML null included. The model's frontmatter check (`optional-nodes`), which loading runs and `apply` runs on a delta, SHALL reject `outside` on any component but `clm`, as loading does, and an `outside` of the wrong shape on any component, the root included.

#### Scenario: globs on the root
- GIVEN `specs/SPEC.md` whose frontmatter has `outside: ["src/web/**", "README.md"]`
- WHEN the model loads `specs/`
- THEN the root's `outside` is `src/web/**` then `README.md`

#### Scenario: empty list
- GIVEN `specs/SPEC.md` whose frontmatter has `outside: []`
- WHEN the model loads `specs/`
- THEN the root's `outside` is present and empty, and loading reports nothing

#### Scenario: no key
- GIVEN `specs/SPEC.md` whose frontmatter has no `outside`
- WHEN the model loads `specs/`
- THEN the root's `outside` is absent and loading reports nothing

#### Scenario: on another node
- GIVEN `specs/a/SPEC.md` whose frontmatter has `outside: ["src/**"]`
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/a/SPEC.md` and `outside`

#### Scenario: bad value
- GIVEN `specs/SPEC.md` whose frontmatter has `outside: "src/**"`, `outside: [""]` or `outside:` with no value
- WHEN the model loads `specs/`
- THEN loading fails naming `specs/SPEC.md` and `outside`

#### Serves
- architecture/incremental-adoption
- capability/adopt-clm

### Requirement: delta-section-fit
The model SHALL reject a delta section that its kind of delta can't apply, rather than load it and let `apply` drop it. A delta with neither `create: true` nor `remove: true` edits an existing component and SHALL hold no `## Frontmatter`, `## Contract` or `## Design` section, which only a `create: true` delta writes. A delta with `create: true` writes a new component whole and SHALL hold no `## MODIFIED Frontmatter`, `## MODIFIED Contract`, `## MODIFIED Design`, `## MODIFIED Requirements`, `## REMOVED Requirements` or `## REMOVED Assets` section, which only edit a component that exists. A `## MODIFIED Contract` or `## MODIFIED Design` section SHALL hold a non-blank line, since it replaces the section's whole body; any line with a non-whitespace character counts, HTML comments and fence markers included. Each such section SHALL be one `model/delta-section-fit` violation, whose path is the delta's file and whose message names the section and the 1-based line number in the file of its heading, and says what is wrong: for a create-only section, that it needs `create: true` and that `## MODIFIED <name>` edits an existing component's; for an edit-only section, that a `create: true` delta writes its component whole; for an empty section, that it is empty. A section that is both edit-only and empty is reported once, as edit-only. A section this rule rejects is judged no further by `parse-changes` (a stray `## Frontmatter` without a ```` ```yaml ```` fence is reported once, by this rule), while `requirement-text-under-a-heading` still judges it, so a `create: true` delta's `## MODIFIED Requirements` holding text before its first heading is reported by both. The delta SHALL not be loaded. A `remove: true` delta's sections stay judged by `parse-changes` alone, so this rule never reports a section that rule rejects. The rule is checked on every delta the model loads, in `changes/<id>/` and `changes/archive/<id>/` alike.

#### Scenario: stray Contract under MODIFIED Contract
- GIVEN a delta for `a`, without `create: true`, holding `## MODIFIED Contract` with no text and then `## Contract` with a paragraph
- WHEN the model loads `changes/`
- THEN loading reports two `model/delta-section-fit` violations, one naming `MODIFIED Contract`, its heading's line and that it is empty, one naming `Contract`, its heading's line, `create: true` and `## MODIFIED Contract`, and the delta isn't loaded

#### Scenario: Design without create
- GIVEN a delta for `a`, without `create: true`, holding `## Design` with a paragraph
- WHEN the model loads `changes/`
- THEN loading reports `model/delta-section-fit`, the delta's file, `Design` and its heading's line, and the delta isn't loaded

#### Scenario: Frontmatter without create
- GIVEN a delta for `a`, without `create: true`, holding `## Frontmatter` with a ```` ```yaml ```` fence
- WHEN the model loads `changes/`
- THEN loading reports `model/delta-section-fit` naming `Frontmatter`, and the delta isn't loaded

#### Scenario: edit section in a create delta
- GIVEN a delta for `b` with `create: true` holding `## Frontmatter`, `## Contract` and `## MODIFIED Requirements` with `### Requirement: x <!-- base: <hash> -->`
- WHEN the model loads `changes/`
- THEN loading reports `model/delta-section-fit` naming `MODIFIED Requirements` and its heading's line, and the delta isn't loaded

#### Scenario: stray Frontmatter without a fence
- GIVEN a delta for `a`, without `create: true`, holding `## Frontmatter` with a line of text and no fence
- WHEN the model loads `changes/`
- THEN loading reports one violation for the section, `model/delta-section-fit` naming `Frontmatter`, and no `model/parse-changes` violation for its missing fence

#### Scenario: edit section with stray text in a create delta
- GIVEN a delta for `b` with `create: true` holding `## Frontmatter` and `## MODIFIED Requirements` with a line of text before `### Requirement: x <!-- base: <hash> -->`
- WHEN the model loads `changes/`
- THEN loading reports `model/delta-section-fit` and `model/requirement-text-under-a-heading`, both naming `MODIFIED Requirements`

#### Scenario: blank MODIFIED Design
- GIVEN a delta for `a` whose `## MODIFIED Design` heading is followed only by blank lines before `## ADDED Requirements`
- WHEN the model loads `changes/`
- THEN loading reports `model/delta-section-fit` naming `MODIFIED Design`, its heading's line and that it is empty, and the delta isn't loaded

#### Scenario: well-formed deltas
- GIVEN a delta for `a` holding `## MODIFIED Contract` with a paragraph, and a delta for `b` with `create: true` holding `## Frontmatter`, `## Contract`, `## Design` and `## ADDED Requirements`
- WHEN the model loads `changes/`
- THEN loading reports nothing for either delta

#### Scenario: removal delta judged once
- GIVEN a delta for `a` with `remove: true` holding a `## Contract` section
- WHEN the model loads `changes/`
- THEN loading reports the `model/parse-changes` violation for `## Contract` and no `model/delta-section-fit` violation

### Requirement: graph-check
The model SHALL expose two functions that loading runs, so a caller holding a graph checks it by exactly the rules loading uses:

- its component parser: given a component's ID, its `SPEC.md` path and text, and the frontmatter keys live changes declare, it returns the component, if one loads, and the violations, as `(Option<clm_model::Node>, Vec<Violation>)`, as loading gives them for that file (`parse-node-specs`, `reserved-ids`, `valid-frontmatter`, `optional-nodes`' and `outside-globs`' root rules, `unique-requirement-ids`, `requirement-needs`' and `serves`' block rules, and the rest of what one file decides);
- its graph check: given a graph, it returns the violations of the rules that join components, and backlog items, to other components: `dependency-dag` (a `depends_on` naming no component, and cycles), `optional-nodes`' rule that a required component doesn't depend on an optional one, `requirement-needs`' rules across requirements (a need naming no requirement, one naming a human-layer entry, one of the same component, one from a required component's requirement to an optional component's, and cycles of needs), `serves`' check that each ID names an entry that exists, and `parse-backlog`'s checks of an item's `origin`, `components` and `blocks` against the graph, exactly as loading runs them today, so a later change to those checks lands in this one function).

Loading SHALL report the same violations as now, from the same rules, for the same files, and keep the same components and items; only where in its code the rules run changes. The check of the root's `SPEC.md` being present, and of a parent component's, stays in loading, since it reads which `SPEC.md` files exist, including those that didn't parse; `apply` checks parents itself (`apply/create-node`, `apply/remove-node`).

#### Scenario: graph check on a loaded graph
- GIVEN component `a` with `optional: true`, component `b` with `depends_on: [a, gone]`, and a backlog item whose `blocks` names `a/none`
- WHEN the model loads `specs/` and `backlog/`, and its graph check runs on the graph that loads
- THEN the graph check reports `b`'s `model/optional-nodes` and `model/dependency-dag` violations and the item's `model/parse-backlog` violation, each as loading reports it

#### Scenario: node parser as loading parses
- GIVEN a `SPEC.md` text for component `a` with two `### Requirement: foo` blocks
- WHEN the component parser runs on it, and the model loads a tree holding it as `specs/a/SPEC.md`
- THEN both report the same `model/unique-requirement-ids` violation naming `a/foo`

#### Scenario: graph check on an edited graph
- GIVEN a loaded graph with component `a` and component `b` with `depends_on: [a]`, which reports nothing
- WHEN `a` is removed from the graph in memory and the graph check runs on it
- THEN it reports `model/dependency-dag` for `b`, naming `a`

### Requirement: human-layers
The model SHALL load the two human layers from the repository root: the entries of `ARCHITECTURE.md`, its architectural constraints, are its `### Constraint: <slug>` blocks inside its `## Constraints` section, and the entries of `CAPABILITIES.md` are its `### Capability: <slug>` blocks inside its `## Capabilities` section. The graph SHALL expose, per layer, the heading its file's entries use: `### Constraint:` and `### Capability:`. Each block runs to the next entry heading or to its section's end, as a component's requirement does, and headings inside code fences don't count; the preamble and any other section are prose the model doesn't read. An entry's ID is `architecture/<slug>` or `capability/<slug>`, its hash is `content_hash` of its block and its version the first 8 hex digits of that, as for a rule (`content-hash`). The graph SHALL expose the entries per layer in file order; `Graph::requirement` SHALL find an entry by its ID, and `Graph::requirements` SHALL list every component's requirements by component ID, then the architecture entries, then the capabilities. A file that is absent has no entries and gives no violation; one that holds no such section has no entries either, and gives a violation only for an entry heading outside it. The model SHALL report as `model/human-layers`, naming the file and the slug: a slug that is empty or holds anything but lowercase ASCII letters, digits and `-`, which no ID could name; a slug used twice in one file; an entry with no `#### Scenario:`; an architecture entry whose text holds neither `SHALL` nor `MUST`; a capability without exactly one `#### Why` block; an entry heading of the file's kind outside its section, so an entry never silently doesn't count; and a `#### Needs` or `#### Serves` block inside an entry. An architecture entry MAY hold one `#### Enforced by` block, a bullet list of `- <component>/<slug>` lines shaped like `#### Needs` (`requirement-needs`), each naming a component's requirement, the rules that enforce the entry; and any entry MAY hold one `#### Untested` block, prose saying what of the entry no program checks and why (`human-layer-proof`). Both blocks are part of the entry's block, so of its hash and version, unlike a rule's `#### Serves`. The graph SHALL expose each entry's enforcing rules, in block order, whether it holds `#### Untested`, and that block's text, trimmed, which the architecture review shows (`effort/architecture-report`). The model SHALL also report as `model/human-layers`, naming the file and the slug: an `#### Enforced by` block in a capability; a second block of either kind in one entry; an empty block; an `#### Enforced by` line that isn't a `- <id>` bullet; and an `#### Enforced by` ID that names a human-layer entry, or names no requirement the graph holds and none that a delta for its component of a change outside the archive adds under `## ADDED Requirements`. A change counts whatever its `status`, and a delta that failed to load adds nothing, as for a backlog item's `blocks` (`parse-backlog`): `ARCHITECTURE.md` is edited directly, holding one set of IDs for the graph before a change and the one after it, so the edit that goes with a change moving or adding a rule names the rule's new ID and passes `clm check` before `clm apply`, while an ID the change removes is reported on the graph it would leave (`apply/resulting-graph`). A file that isn't UTF-8 is reported the same way and has no entries.

#### Scenario: entries load
- GIVEN an `ARCHITECTURE.md` whose `## Constraints` holds `### Constraint: git-is-truth` with a SHALL statement and a scenario, and a `CAPABILITIES.md` whose `## Capabilities` holds `### Capability: approve` with a statement, a `#### Why` and a scenario
- WHEN the model loads
- THEN the graph holds `architecture/git-is-truth` and `capability/approve`, each at the version of its block's content hash, `Graph::requirement` finds both, the architecture layer's heading is `### Constraint:`, and loading reports nothing

#### Scenario: enforcement blocks
- GIVEN an `ARCHITECTURE.md` entry `x` with an `#### Enforced by` block naming `a/y` and `b/z`, rules of components `a` and `b`, and an `#### Untested` paragraph, and a capability `c` with an `#### Untested` paragraph
- WHEN the model loads
- THEN the graph gives `x`'s enforcing rules as `a/y` then `b/z`, says `x` and `c` hold `#### Untested`, gives `x`'s untested text as its paragraph, trimmed, and loading reports nothing

#### Scenario: bad enforcement
- GIVEN an architecture entry whose `#### Enforced by` names `a/gone`, which no component holds, and a capability with an `#### Enforced by` block
- WHEN the model loads
- THEN loading reports `model/human-layers` for each, naming `ARCHITECTURE.md` and the entry's slug with `a/gone`, and `CAPABILITIES.md` and the capability's slug

#### Scenario: enforcement named before apply
- GIVEN an architecture entry whose `#### Enforced by` names `a/new`, which component `a` doesn't hold, and a `proposed` change whose delta for `a` adds `new` under `## ADDED Requirements`
- WHEN the model loads
- THEN loading reports no `model/human-layers` violation, and the entry's enforcing rules are `a/new`

#### Scenario: free-form architecture
- GIVEN an `ARCHITECTURE.md` with a `## Layer 1 invariants` section of bullet items and no `## Constraints` section
- WHEN the model loads
- THEN the graph has no architecture entries and loading reports nothing

#### Scenario: entry outside its section
- GIVEN a `CAPABILITIES.md` with `### Capability: x` under `## Why we exist`
- WHEN the model loads
- THEN loading reports `model/human-layers`, `CAPABILITIES.md` and `x`

#### Scenario: entry heading and no section
- GIVEN a `CAPABILITIES.md` with `### Capability: x` and no `## Capabilities` section
- WHEN the model loads
- THEN the graph has no capability entries, and loading reports `model/human-layers`, `CAPABILITIES.md` and `x`

#### Scenario: capability without why
- GIVEN a `CAPABILITIES.md` whose entry `x` has a scenario and no `#### Why`
- WHEN the model loads
- THEN loading reports `model/human-layers`, `CAPABILITIES.md` and `x`

#### Serves
- architecture/traced-rules
- architecture/humans-review-intent
- capability/review-capability-cost

### Requirement: reserved-ids
The model SHALL reject a top-level component whose ID is `architecture` (`specs/architecture/SPEC.md`), the prefix of the architecture entries' IDs (`human-layers`), as `model/reserved-ids` naming the file, and SHALL NOT load it; its children are then reported as components whose parent has no `SPEC.md` (`parse-node-specs`). A top-level component `capability` (`specs/capability/SPEC.md`) SHALL load as the capability layer's component: the capabilities, the entries of `CAPABILITIES.md`, are placed on it, so they are pending on it and a build of it builds their tests (`human-layer-proof`). The graph SHALL say which component that is, if any; the component's own requirements stay empty and `Graph::requirements` lists the capabilities after the architecture entries as before, so nothing that walks a component's requirements, such as `check/untraced` or the size count, sees them twice. Its `SPEC.md` holds frontmatter, a Contract and a Design, and a `### Requirement:` in it SHALL be reported as `model/reserved-ids` naming the file and the slug, since a capability is written in `CAPABILITIES.md`.

#### Scenario: reserved node
- GIVEN `specs/architecture/SPEC.md`
- WHEN the model loads `specs/`
- THEN loading reports `model/reserved-ids` and `specs/architecture/SPEC.md`, and the graph has no component `architecture`

#### Scenario: capability node
- GIVEN `specs/capability/SPEC.md` with frontmatter, a Contract and a Design, and a `CAPABILITIES.md` with entries `a` and `b`
- WHEN the model loads
- THEN the graph has component `capability`, which it names as the component the capabilities `capability/a` and `capability/b` are placed on, and loading reports nothing

#### Scenario: requirement in the capability node
- GIVEN `specs/capability/SPEC.md` holding `### Requirement: x`
- WHEN the model loads
- THEN loading reports `model/reserved-ids`, `specs/capability/SPEC.md` and `x`

#### Serves
- architecture/traced-rules
- architecture/humans-review-intent

### Requirement: serves
A component's frontmatter MAY hold `serves`, a list of human-layer entry IDs (`human-layers`): the entries its rules serve. A requirement MAY hold one `#### Serves` block among its scenarios, a bullet list of entry IDs shaped like `#### Needs` (`requirement-needs`), naming what that rule serves instead of its component's. A rule's effective serves, which `Graph::effective_serves` gives, is its own block's IDs when it has the block, and its component's `serves` otherwise; it is empty for an entry or an ID the graph doesn't hold. The model SHALL report as `model/serves`, keeping the component loaded: an ID in `serves` or in a block that isn't `architecture/<slug>` or `capability/<slug>`, or that names no entry; a block line that isn't a `- <id>` bullet; a second `#### Serves` block in one requirement; an empty block; and a block outside any requirement. The block is part of the requirement's text and of its hash, but not of its version, so adding, editing or removing it moves the rule's hash, which deltas record as their bases, and leaves its version as it was (`content-hash`); `serves` names entries without versions, so rewording an entry leaves the rules that serve it current.

#### Scenario: block overrides the node
- GIVEN component `a` with `serves: [capability/approve]`, its requirement `x` with a `#### Serves` block naming `architecture/git-is-truth`, and its requirement `y` with none
- WHEN the model loads
- THEN `a/x`'s effective serves is `architecture/git-is-truth` and `a/y`'s is `capability/approve`

#### Scenario: unknown entry
- GIVEN component `a` with `serves: [capability/nope]` and no capability `nope`
- WHEN the model loads
- THEN loading reports `model/serves`, `specs/a/SPEC.md` and `capability/nope`

#### Scenario: not a layer ID
- GIVEN component `a` with `serves: [land]`, or a `#### Serves` line `- land/approve`
- WHEN the model loads
- THEN loading reports `model/serves` and the ID

#### Serves
- architecture/traced-rules
- capability/review-capability-cost

### Requirement: term-lists
`model` SHALL expose, as public constants, every component status (`Status`), change status (`ChangeStatus`) and backlog kind (`BacklogKind`), each with the word CLM writes for it in a file (`unbuilt`, `applied`, `bug`), and never a word it only reads, such as one `plain-terms-renames` replaced (`renamed-words-still-load`), and the list of keys a live change's `change.md` frontmatter may set: those `ChangeFrontmatter` reads, without `after`, `prefer_after` and `gate`, which it accepts only to ignore or reject, and without the key it reads as `human_burdens`. A test SHALL fail when an enum gains a variant, or `ChangeFrontmatter` a field, that its list leaves out, so a new term can't escape the glossary check (`clm/glossary-covers-terms`).

#### Scenario: lists match the types
- GIVEN the model
- WHEN a test reads each list
- THEN every variant of `Status`, `ChangeStatus` and `BacklogKind` appears once with the word CLM writes for it, the list holds no word `plain-terms-renames` replaced, and the key list holds every `ChangeFrontmatter` field but the three ignored or rejected ones

#### Serves
- architecture/plain-terms

### Requirement: rename-records
The model SHALL load a change's `renames` as a list of entries in file order, each a mapping with exactly one of `component`, `requirement` or `term`, naming what is renamed, and `to`, naming its new name, and expose them on the change. On a change outside the archive it SHALL report as `model/rename-records`, naming the `change.md` and the entry's 1-based position: an entry with none or more than one of the three keys, without `to`, or with any other key; a value that isn't a non-empty string or holds a line break; a `component` or its `to` that isn't a component ID, or a `to` that differs from its `component` in anything but the last dot-separated segment, since a rename keeps the component's parent; a `requirement` that isn't `<component>/<slug>`, `architecture/<slug>` or `capability/<slug>`, or whose `to` isn't a slug (`[a-z0-9]+(-[a-z0-9]+)*`); a `to` equal to what it renames; and two entries renaming the same thing, or giving two things the same new name. An archived change's `renames` SHALL be loaded as written, unchecked (`archive-immutable`). Whether a renamed ID exists is `clm rename`'s to judge (`rename/refusals`), since the graph a change is checked against may already hold its result.

#### Scenario: valid entries
- GIVEN `changes/x/change.md` with `renames` holding `component: check.coverage` with `to: check.markers`, `requirement: apply/stale-base` with `to: base-moved`, and `term: sprocket` with `to: cogs`
- WHEN the model loads `changes/`
- THEN `x` holds the three renames in that order, and loading reports nothing

#### Scenario: a move, not a rename
- GIVEN `changes/x/change.md` with `renames` holding `component: check.coverage` with `to: model.coverage`
- WHEN the model loads `changes/`
- THEN loading reports `model/rename-records`, `changes/x/change.md` and entry `1`

#### Scenario: two keys in one entry
- GIVEN `changes/x/change.md` with `renames` holding one entry with `term: sprocket`, `requirement: a/x` and `to: y`
- WHEN the model loads `changes/`
- THEN loading reports `model/rename-records`, `changes/x/change.md` and entry `1`

#### Serves
- architecture/plain-terms

### Requirement: component-keys
A delta SHALL name the component it edits with the frontmatter key `component`, and a backlog item SHALL list the components it concerns with `components`, the names `plain-terms-renames` gives them. The model SHALL reject a delta of a change outside the archive that sets `node`, and a backlog item in `backlog/` that sets `nodes`, naming the file, the old key and the new one. It SHALL still load the `node` key of an archived change's delta as its `component`, since the archive keeps what it was written with (`archive-immutable`). It SHALL still read `nodes` in a backlog item's frontmatter read from git history rather than loaded from `backlog/`, such as `effort`'s history and the gate's merge base, for the same reason. No `clm:remove-by 20261102-rename-node-to-component-contract` marker SHALL be left. The loaded delta and item, and what the graph exposes of them, SHALL be the same whichever key an archived delta or a history read used.

#### Scenario: a delta by its new key
- GIVEN `changes/c/deltas/a.md` with `component: a` and a `## MODIFIED Design` section whose base is node `a`'s Design hash
- WHEN the model loads `changes/`
- THEN change `c` holds a delta for `a`, and loading reports nothing

#### Scenario: the old keys refused
- GIVEN `backlog/x.md` with `nodes: [a]`, and `changes/c/deltas/a.md` with `node: a`
- WHEN the model loads
- THEN loading reports `backlog/x.md` with `nodes` and `components`, and `changes/c/deltas/a.md` with `node` and `component`

#### Scenario: the archive keeps its key
- GIVEN `changes/archive/c/deltas/a.md` with `node: a`
- WHEN the model loads `changes/`
- THEN archived change `c` holds a delta for `a`, and loading reports nothing

#### Scenario: an unknown component
- GIVEN `backlog/x.md` with `components: [ghost]`
- WHEN the model loads
- THEN loading reports `backlog/x.md` and `ghost`

#### Serves
- architecture/plain-terms
- capability/keep-a-backlog

### Requirement: says-component
The model's messages SHALL say component, components and ancestors, never the old words, as `component-words` defines them: among them `the root component has no assets`, ``asset of component `<id>` ``, ``parent component `<id>` has no SPEC.md``, ``the root component can't be `optional` ``, ``depends on unknown component `<id>` ``, ``required component `<a>` depends on optional component `<b>` ``, ``unknown component `<id>` `` for a backlog item, `edits an existing component's <element>`, a delta that `creates its component or removes it`, and the shapes it names, `<component>/<slug>` and `<component>/<slug>@<version>`. The rules a violation names are requirement IDs and keep their spelling, and a message naming an old key to say it is old (`component-keys`) keeps it. A test reads the string literals in `crates/clm-model/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: an unknown component in an item
- GIVEN `backlog/x.md` with `components: [ghost]`
- WHEN the model loads
- THEN the violation's message reads ``unknown component `ghost` ``

#### Scenario: a required component on an optional one
- GIVEN component `a` with `optional: true` and component `b` with `depends_on: [a]`
- WHEN the model loads `specs/`
- THEN the violation's message names ``required component `b` `` and ``optional component `a` ``

#### Serves
- architecture/plain-terms

### Requirement: backlog-components
The graph SHALL expose `Graph::backlog_node_known(id)`, whether a backlog item may name component `id`: true when the component is in the graph, or when a delta with `create: true` of a change outside the archive creates it, and false otherwise. `parse-backlog` SHALL judge an item's `components` by it, so every caller that writes a backlog item, such as `clm defect` (`plugin.hooks`), applies the rule `clm check` applies.

#### Scenario: known nodes
- GIVEN a graph with component `a`, a live change whose delta for `b` has `create: true`, and an archived change whose delta for `c` has `create: true`
- WHEN `backlog_node_known` is asked about `a`, `b`, `c` and `d`
- THEN it answers true for `a` and `b`, and false for `c` and `d`

#### Serves
- capability/keep-a-backlog
- capability/report-defects

### Requirement: capability-tests-component
The model SHALL give the component that builds the capability tests as `Graph::capability_tests_node`: the graph's capability component (`reserved-ids`) when it holds one, else its root, the component `clm pending` places `CAPABILITIES.md`'s entries on. Every caller that asks which component that is asks the model, rather than keeping the rule itself.

#### Scenario: with the capability node
- GIVEN a graph with root `clm` and component `capability`
- WHEN `capability_tests_node` is asked
- THEN it gives `capability`

#### Scenario: without it
- GIVEN a graph with root `clm` and no component `capability`
- WHEN `capability_tests_node` is asked
- THEN it gives `clm`

### Requirement: parse-backlog-item
`clm-model` SHALL offer `parse_backlog_item(slug, text)`, which parses one backlog file's text by `parse-backlog`'s rules for its frontmatter keys and values alone, returning the item or the violations, and checks nothing against a graph: not that its components, origin, `blocks` or `requirement` exist. `clm intake` reads items deleted at older commits with it, whose components may since have gone.

#### Scenario: a deleted item's components are gone
- GIVEN the text of a backlog file with `kind: bug`, `components: [ghost]` and `issues: [67]`
- WHEN `parse_backlog_item("x", text)` runs
- THEN it returns item `x` with issues `[67]` and no violation

#### Scenario: an unknown key
- GIVEN the text of a backlog file with `kind: bug` and `colour: red`
- WHEN `parse_backlog_item("x", text)` runs
- THEN it returns a violation naming `colour`

#### Serves
- capability/contribute-from-outside

### Requirement: carried-constraints
The model SHALL compile in, with `include_str!`, the text of CLM's own `ARCHITECTURE.md` at this repository's root as it is when the crate is built, and expose its *carried entries*: each architecture entry holding a `#### Carries` block, in file order, with its slug, its tier (the block's one line, `- method`, `- replaceable` or `- default`), its statement (the entry's text from after its heading to its first `####` heading, trimmed) and, for a `replaceable` entry, its `#### Guarantee` block's text, trimmed. A repository whose own `ARCHITECTURE.md` holds an entry with a `#### Carries` block is the *source*, CLM's own, and its graph takes the carried entries from its own file; any other repository's graph, including one with no `ARCHITECTURE.md`, takes them from the compiled copy, so it holds the set its `clm` enforces. Outside the source, an entry of the repository's `ARCHITECTURE.md` MAY hold one `#### Replaces` block of `- <slug>` lines, each naming a carried entry it replaces or drops. `clm_model::carried()` SHALL give the compiled copy's carried entries without loading any repository, for a caller that runs outside one. The graph SHALL say whether it is the source, and expose the carried entries and, outside the source, the carried entries in force: those no entry's `#### Replaces` names, in the carried order. `#### Carries`, `#### Guarantee` and `#### Replaces` blocks are part of the entry's block, so of its hash, as `#### Enforced by` is (`human-layers`). The model SHALL report as `model/human-layers`, naming `ARCHITECTURE.md` and the entry's slug: a `#### Carries` block that isn't exactly one line naming a tier; a `replaceable` entry without exactly one `#### Guarantee` block, and a `#### Guarantee` block in any other entry; a second `#### Carries` or `#### Replaces` block in one entry; a `#### Replaces` block in the source, or one that is empty; a `#### Replaces` line that isn't a `- <slug>` bullet, that names no carried entry, or that names a `method` entry, naming that slug; and, outside the source, an entry whose slug is a carried entry's slug. It SHALL report as `model/human-layers`, naming `CAPABILITIES.md` and the slug, a capability holding any of the three blocks. A test of the compiled copy reads this repository's `ARCHITECTURE.md`, so it holds a `clm:reads` marker naming the file (`doc-readers-marked`). Since a project's `#### Replaces` names CLM's slugs, a later `clm` that renames or removes a carried entry fails that project's `clm check` until the project follows, and one that adds an entry with a slug the project uses fails it too: CLM renames a carried entry the way `expand-contract` says, keeping the old slug readable in `#### Replaces` for a release. Any repository whose `ARCHITECTURE.md` holds a `#### Carries` block counts as the source, so a project could take the carried set out of its bundles by adding one; that is an `ARCHITECTURE.md` edit a human reviews, which the `architecture-auditor` reports (`plugin/architecture-auditor`).

#### Scenario: the source
- GIVEN an `ARCHITECTURE.md` with entry `a` holding `#### Carries` `- method`, entry `b` holding `#### Carries` `- replaceable` and a `#### Guarantee` paragraph, and entry `c` with no `#### Carries`
- WHEN the model loads
- THEN the graph is the source, its carried entries are `a` (`method`) then `b` (`replaceable`, with its guarantee text), `c` is not among them, and loading reports nothing

#### Scenario: a project
- GIVEN an `ARCHITECTURE.md` with no `#### Carries` block, whose entry `x` holds `#### Replaces` naming `expand-contract` and `yagni`
- WHEN the model loads
- THEN the graph is not the source, its carried entries are the compiled copy's, holding `proven-by-tests`, `expand-contract` and `yagni`, the entries in force hold `proven-by-tests` and neither `expand-contract` nor `yagni`, and loading reports nothing

#### Scenario: no architecture file
- GIVEN a repository with no `ARCHITECTURE.md`
- WHEN the model loads
- THEN the entries in force are every carried entry of the compiled copy

#### Scenario: bad replaces
- GIVEN a project `ARCHITECTURE.md` whose entry `x` holds `#### Replaces` naming `proven-by-tests` and `no-such-entry`, and whose entry `yagni` is its own
- WHEN the model loads
- THEN loading reports `model/human-layers` for `x` with `proven-by-tests`, for `x` with `no-such-entry`, and for `yagni`

#### Scenario: bad carries
- GIVEN a source `ARCHITECTURE.md` with entry `a` holding `#### Carries` `- optional`, entry `b` holding `#### Carries` `- replaceable` and no `#### Guarantee`, entry `c` holding `#### Carries` `- method` and a `#### Guarantee`, and entry `d` holding `#### Replaces`
- WHEN the model loads
- THEN loading reports `model/human-layers` for each of `a`, `b`, `c` and `d`

#### Serves
- architecture/carried-constraints
