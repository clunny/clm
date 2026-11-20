---
title: Apply
status: active
owners: ["@clunny"]
depends_on: [model]
paths: ["crates/clm-apply/**"]
serves: [capability/propose-intent, architecture/hash-based-staleness]
---

## Contract
- `validate(graph, change)` returns every reason the change can't be applied to the spec graph, as violations: a status `clm apply` refuses (`change-status`), each delta and asset that can't be applied, and every rule across components that the graph the change leaves would break, so a change that applies leaves a graph that loads.
- `resulting(graph, change)` returns the spec graph `validate` checks the change against, the graph the change would leave, or none when a delta of the change can't be applied, for `check`'s rules that judge a pending change's text (`resulting-graph-call`).
- `stale_bases(component, delta)` returns each Contract, Design or requirement `delta` MODIFIES or REMOVES whose recorded base `validate` judges stale against `component` (`stale-base`), with that base, for `sync.regenerate`'s stale-delta report and merges and for `rename`'s rebase of other changes' deltas (`stale-elements`).
- `hashes(component)` returns a component's `(element, hash)` pairs in `clm hashes`' order: its frontmatter, Contract and Design, then its requirements, then its assets (`hash-command`). The entries of a human layer, which `clm hashes architecture` and `clm hashes capability` print, are the command's and not this call's.
- `apply(root, id)` is `clm apply` on a checkout: it returns the files it wrote and removed, each a `FileOp`, or the violations; when the change or the graph is at fault it has written nothing (`atomic`). `reapply(graph, change)` returns the `specs/` files an already applied change writes and removes, writing none itself (`reapply`). A `FileOp` gives its path from the repository root with `path()`, and `perform(root)` writes or removes that file under `root`.
- `archive(root, id)` is `clm archive` on a checkout: it returns the directory the change moved to, or the violations.
- `clm apply <change-id>` applies a change's deltas to `specs/` and copies or removes its assets, deletes the `question` items its `resolves` names (`answered-questions`) and the item its `from` names (`promoted-item`), all-or-nothing, and marks the change `applied`. It prints `applied <id>` and a line per file it wrote or removed, and exits 0 (`change-status`). It exits 1, printing each violation on standard output and writing nothing, on any violation of the change, or of the loaded graph that the change doesn't fix (`loaded-graph`).
- `clm archive <change-id>` moves a `done` or `abandoned` change's directory to `changes/archive/<id>/` and prints where it went; it exits 1, moving nothing, for any other change (`archive-command`).
- `clm hashes <component>` prints one `<element> <hash>` line per element of the component: `frontmatter`, `contract`, `design`, then `requirement:<slug>` for each requirement, then `asset:<path>` for each asset. `clm hashes architecture` and `clm hashes capability` print one line per entry of that human layer, in file order, `requirement:<slug> <hash>` or `capability:<slug> <hash>`, the word its heading uses (`model/human-layers`). When the repository has the capability layer's component (`model/reserved-ids`), `clm hashes capability` prints that component's element lines first, then the entries' lines: the component holds no requirement, so no line name is printed twice. For an ID that names neither, it exits 2 with a line on standard error.

`clm-apply` offers each of `clm apply`, `clm archive` and `clm hashes` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## Design
Apply computes each new SPEC.md text in memory, starting from the current text. Sections and requirement blocks are replaced by their heading spans (outside code fences). A removed component's `SPEC.md` is removed after its assets, so a write that fails part way never leaves assets without a component.

One function gives the backlog items `clm apply` deletes with a change (`answered-questions`, `promoted-item`). The apply and `resulting` both take them from it (`resulting-graph-call`), so the graph a change is checked against can't hold an item its apply deletes, and the apply builds the graph it checks through `resulting`.

`validate` and `stale_bases` judge a recorded Contract, Design or requirement base through one function, so the two can't come to differ (`stale-elements`).

## Requirements

### Requirement: atomic
`clm apply` SHALL validate every delta of the change before writing anything. If any violation exists, of the change or of the loaded graph as `loaded-graph` counts them, it SHALL print all of them on standard output, one line each as `model/violation-line` gives it, exit 1 and leave every file unchanged, the backlog items `answered-questions` and `promoted-item` would delete among them.

#### Scenario: one bad delta
- GIVEN a change with a valid delta for `a` and a stale delta for `b`
- WHEN `clm apply <change>` runs
- THEN it exits 1, names `b`'s stale element, and neither `a` nor `b` is modified

#### Scenario: refused change keeps its question
- GIVEN `backlog/q.md` with `kind: question`, and change `c` whose `resolves` is `[q]` with a stale delta
- WHEN `clm apply c` runs
- THEN it exits 1 and `backlog/q.md` still exists

#### Scenario: refused change keeps its item
- GIVEN `backlog/i.md` with `kind: idea`, and change `c` with `from: backlog/i` and a stale delta
- WHEN `clm apply c` runs
- THEN it exits 1 and `backlog/i.md` still exists

### Requirement: stale-base
A delta SHALL be invalid when a base hash it records (frontmatter, contract, design, or a requirement's `<!-- base: -->`) differs from the current element's hash, the two compared as whole strings, and when it MODIFIES or REMOVES an element without recording that element's base. A MODIFIED requirement whose block has no `#### Serves` block keeps the rule's own (`requirements`), so only its version can go stale: its base SHALL be stale only when its length is any other than the rule's hash's, shorter or longer, or its first 8 hex digits differ from the rule's current version (`model/content-hash`), and an edit to only the rule's `#### Serves` block leaves it valid. Each stale or missing base, and a delta that modifies or removes a component the graph doesn't hold, SHALL be reported under `apply/stale-base`, naming the component and, for a base, the element: a stale base with the hash recorded and the current one, a missing base with `clm hashes <component>`, the command that prints it, and a component that is gone as ``component `<id>` no longer exists``.

#### Scenario: requirement changed since the delta was written
- GIVEN a delta modifying `a/r` with base `h1`
- AND `a/r` now hashes to `h2`
- WHEN the change is validated
- THEN it reports `a/r` as stale, with both hashes

#### Scenario: bare version
- GIVEN a delta modifying `a/r` whose block has no `#### Serves` block, with only the first 8 hex digits of `a/r`'s hash as its base, or those digits followed by `zz`
- WHEN the change is validated
- THEN it reports `a/r` under `apply/stale-base`

#### Scenario: no recorded base
- GIVEN a delta for `a` with a `## MODIFIED Design` and a `REMOVED Requirements` entry for `x`, recording no base for either
- WHEN the change is validated
- THEN it reports both under `apply/stale-base`, each message holding `clm hashes a`

#### Scenario: serves edited under a delta that carries a block
- GIVEN a delta modifying `a/r` whose block has a `#### Serves` block, with `a/r`'s hash as its base
- AND `a/r`'s `#### Serves` block is then edited and nothing else
- WHEN the change is validated
- THEN it reports `a/r` as stale

#### Scenario: serves edited under a delta without a block
- GIVEN a delta modifying `a/r` whose block has no `#### Serves` block, with `a/r`'s hash as its base
- AND `a/r`'s `#### Serves` block is then edited and nothing else
- WHEN the change is applied
- THEN it applies, and `a/r` has the delta's text and the edited `#### Serves` block

### Requirement: requirements
ADDED requirements SHALL be appended to the component's Requirements section and SHALL be invalid if the slug exists. MODIFIED requirements SHALL replace the existing block in place, except that a MODIFIED requirement without a `#### Serves` block SHALL keep the existing block's; and REMOVED requirements SHALL delete it. Both SHALL be invalid if the slug does not exist. Each is reported under `apply/requirements`, naming the requirement.

#### Scenario: modify in place
- GIVEN component `a` with requirements `x`, `y`, `z`
- WHEN a delta modifies `y`
- THEN `a`'s requirements are `x`, new `y`, `z` in that order

#### Scenario: serves kept
- GIVEN component `a` whose requirement `y` has a `#### Serves` block naming `capability/c`, and a delta modifying `y` with new text and no `#### Serves`
- WHEN the change is applied
- THEN `y` has the new text and its `#### Serves` block still names `capability/c`; a delta whose `y` has its own `#### Serves` replaces the block

### Requirement: sections
A MODIFIED Contract or MODIFIED Design SHALL replace that section's body. A delta that modifies the Contract or the Design of a component whose `SPEC.md` has no `## Contract` or `## Design` section SHALL be invalid under `apply/sections`, naming the component and the section.

#### Scenario: modify contract
- GIVEN a delta with `## MODIFIED Contract` and the correct base
- WHEN applied
- THEN the component's Contract is the delta's text and its Design is byte-for-byte unchanged

#### Scenario: no such section
- GIVEN component `a` whose `SPEC.md` has no `## Design` section, and a delta with `## MODIFIED Design` and the correct base
- WHEN the change is validated
- THEN it reports the delta under `apply/sections`, naming `a` and `## Design`

### Requirement: frontmatter
A MODIFIED Frontmatter SHALL replace the line of each listed top-level key, append keys not present, and leave every other line (including comments) unchanged. It SHALL be invalid, under `apply/frontmatter`, if a listed key's current value spans more than one line, which it does when the line after the key's starts with a space, a tab or `-`, or if a line of the delta's block that is neither blank nor a `#` comment isn't a top-level `key: value` on one line: one with no `:`, one with nothing before its `:`, or one that starts with a space, a tab or `-`.

#### Scenario: comment preserved
- GIVEN frontmatter `status: unbuilt  # frontier` and `depends_on: [model]`
- WHEN a delta modifies `depends_on: [model, apply]`
- THEN the `status` line, including its comment, is unchanged

#### Scenario: current value over several lines
- GIVEN frontmatter whose `depends_on:` is followed by the line `  - clm`, and a delta that modifies `depends_on: []` with the correct base
- WHEN the change is validated
- THEN it reports the delta under `apply/frontmatter`, its message holding `spans more than one line`

#### Scenario: a list item in the delta
- GIVEN a delta whose `## MODIFIED Frontmatter` holds `depends_on:` and then the line `  - model`, with the correct base
- WHEN the change is validated
- THEN it reports the delta under `apply/frontmatter`, its message holding that line

### Requirement: create-node
A delta with `create: true` SHALL write a new SPEC.md from its Frontmatter, Contract, Design and ADDED Requirements. It SHALL be invalid, under `apply/create-node`, if the component exists, if its parent neither exists nor is created by the same change, or if it has no `## Frontmatter` section.

#### Scenario: new child node
- GIVEN a change creating `a` and `a.b`
- WHEN applied
- THEN `specs/a/SPEC.md` and `specs/a/b/SPEC.md` both exist and `clm check` is clean

#### Scenario: exists, or no parent
- GIVEN component `a`, no component `q`, and a change creating `a` and `q.r`
- WHEN the change is validated
- THEN it reports both deltas under `apply/create-node`

#### Scenario: no frontmatter section
- GIVEN a delta for `b` with `create: true`, a `## Contract` and a `## Design` and no `## Frontmatter`
- WHEN the change is validated
- THEN it reports the delta under `apply/create-node`, its message naming `## Frontmatter`

### Requirement: change-status
`clm apply` SHALL refuse, under `apply/change-status`, a change whose status is not `proposed` or `approved`, and an ID that names no change, as ``no change `<id>` ``, and SHALL set the status line to `applied` on success. On success it SHALL print on standard output `applied <id>` and then one line for each file it wrote or removed, two spaces and `wrote <path>` or `removed <path>`, the path from the repository root, with the change's `change.md` last, and exit 0.

#### Scenario: already applied
- GIVEN a change with `status: applied`
- WHEN `clm apply <change>` runs
- THEN it exits 1 and nothing changes

#### Scenario: applied output
- GIVEN a proposed change `c` whose one delta is for the root component
- WHEN `clm apply c` runs
- THEN it exits 0, standard output is the lines `applied c`, `  wrote specs/SPEC.md` and `  wrote changes/c/change.md`, and standard error is empty

#### Scenario: no such change
- GIVEN no change `nope`
- WHEN `clm apply nope` runs
- THEN it exits 1 printing a line that holds `apply/change-status` and ``no change `nope` ``, and nothing changes

### Requirement: hash-command
`clm hashes <component>` SHALL print one `<element> <hash>` line per element of the component: `frontmatter`, `contract`, `design`, then `requirement:<slug>` for each requirement in file order, then `asset:<path>` for each asset in byte order of the paths. `clm hashes architecture` and `clm hashes capability` SHALL print one line per entry of that layer, in file order, `requirement:<slug> <hash>` for an architecture entry and `capability:<slug> <hash>` for a capability, the word its heading uses (`model/human-layers`). When component `capability` loads, `clm hashes capability` SHALL print its element hashes before the entries', so a delta for that component can record its bases. A component's hashes SHALL equal the hashes `validate` compares bases against; an entry's is the hash whose first 8 hex digits are the version tests cite, since no delta edits a human layer. For an ID that names neither a layer nor a component the graph holds, it SHALL print nothing on standard output and ``clm hashes: no valid component `<id>` `` on standard error, and exit 2. Every message `clm apply` prints that points to the command, such as `apply/stale-base`'s for a missing base, SHALL name `clm hashes`.

#### Scenario: hash then modify
- GIVEN the hash `clm hashes a` prints for `requirement:r`
- WHEN a delta records it as `r`'s base and modifies `r`
- THEN validation reports no staleness

#### Scenario: hashes of a layer
- GIVEN a `CAPABILITIES.md` with entries `a` and `b`, and an `ARCHITECTURE.md` with entry `x`
- WHEN `clm hashes capability` and `clm hashes architecture` run
- THEN the first prints `capability:a <hash>` and `capability:b <hash>`, and the second `requirement:x <hash>`, each the entry's hash

#### Scenario: hashes of the capability node
- GIVEN a `CAPABILITIES.md` with entry `a`, and a component `capability` with frontmatter, a Contract and a Design
- WHEN `clm hashes capability` runs
- THEN it prints `frontmatter`, `contract` and `design` lines, then `capability:a <hash>`
- AND a delta for component `capability` that records the printed `design` hash as its base and modifies its Design reports no staleness

#### Scenario: assets after requirements
- GIVEN component `a` with requirements and the assets `z.txt` and `m.html`
- WHEN `clm hashes a` runs
- THEN its last two lines are `asset:m.html <hash>` and `asset:z.txt <hash>`, each the asset's hash (`model/node-assets`)

#### Scenario: unknown component
- GIVEN no component `nope`
- WHEN `clm hashes nope` runs
- THEN it exits 2, standard output is empty and standard error is ``clm hashes: no valid component `nope` ``

#### Serves
- capability/prove-by-tests
- architecture/hash-based-staleness

### Requirement: archive-command
`clm archive <change-id>` SHALL move `changes/<id>/` to `changes/archive/<id>/` when the change's status is `done` or `abandoned`, print `archived <id> -> changes/archive/<id>` and exit 0. It SHALL refuse, under `apply/archive-command`, exiting 1 with nothing moved, a change of any other status, an archive entry that already exists, and an ID that names no change outside the archive, as ``no unarchived change `<id>` ``.

#### Scenario: archive a done change
- GIVEN `changes/x/` with `status: done`
- WHEN `clm archive x` runs
- THEN `changes/archive/x/` holds the same files and `changes/x/` is gone

#### Scenario: refuse an applied change
- GIVEN `changes/x/` with `status: applied`
- WHEN `clm archive x` runs
- THEN it exits 1 and nothing moves

#### Scenario: archive entry exists
- GIVEN `changes/x/` with `status: abandoned`, and a directory `changes/archive/x/`
- WHEN `clm archive x` runs
- THEN it exits 1 naming `apply/archive-command`, and `changes/x/change.md` is still there

#### Scenario: no such change
- GIVEN no `changes/nope/`
- WHEN `clm archive nope` runs
- THEN it exits 1 printing a line that holds `apply/archive-command` and ``no unarchived change `nope` ``

#### Serves
- capability/wrap-up-session

### Requirement: apply-assets
For each file `changes/<id>/assets/<node-id>/<path>`, apply SHALL write it to that component's `assets/<path>`. It SHALL treat it as an addition when the component has no such asset, and otherwise require the delta's `base.assets` to record the asset's current hash. For each entry `` `<path>` <!-- base: <hash> --> `` under the delta's `## REMOVED Assets`, it SHALL delete the asset, requiring the recorded hash to match. A missing or mismatched base, and a recorded base for an asset the component no longer has, is a stale delta, reported under `apply/stale-base` naming the component and the asset, and nothing is written; a `## REMOVED Assets` entry with no base never reaches this rule, since its delta doesn't load (`model/parse-changes`). Deleting an asset SHALL also remove each directory under `specs/` that the deletion leaves empty. The root component has no assets: a change whose delta for `clm` comes with a file under `assets/clm/`, or lists a removed asset, SHALL be invalid under `apply/apply-assets`. An asset the change both writes and removes, or removes twice, SHALL be invalid under `apply/apply-assets`.

#### Scenario: modified asset
- GIVEN component `a` with asset `m.html`, and a change with `assets/a/m.html` whose delta's `base.assets` records `m.html`'s current hash
- WHEN the change is applied
- THEN `specs/a/assets/m.html` has the change's content

#### Scenario: stale asset
- GIVEN the same change after `specs/a/assets/m.html` was edited on `main`
- WHEN the change is applied
- THEN apply exits 1 naming `a` and `m.html`, and writes nothing

#### Scenario: removed asset
- GIVEN component `a` with asset `old.png`, and a delta for `a` listing `old.png` under `## REMOVED Assets` with its current hash
- WHEN the change is applied
- THEN `specs/a/assets/old.png` no longer exists

#### Scenario: emptied directory
- GIVEN component `a` with assets `m.html` and `img/old.png`, and a delta for `a` listing `img/old.png` under `## REMOVED Assets` with its current hash
- WHEN the change is applied
- THEN `specs/a/assets/img/` no longer exists and `specs/a/assets/m.html` is unchanged

#### Scenario: root asset
- GIVEN a change with a delta for `clm` and a file `assets/clm/x.png`
- WHEN the change is applied
- THEN it is refused under `apply/apply-assets`, and no `specs/assets/` exists

#### Scenario: written and removed
- GIVEN component `a` with asset `m.html`, and a change that both holds `assets/a/m.html` and lists `m.html` under `## REMOVED Assets`, or lists it there twice, each with its current hash
- WHEN the change is applied
- THEN it is refused under `apply/apply-assets`, and `specs/a/assets/m.html` is unchanged

### Requirement: remove-node
A delta with `remove: true` SHALL delete the component's `SPEC.md`, its assets, and its directory once the directory holds nothing else (`node-removal`). It SHALL be invalid if the component doesn't exist, as `stale-base` reports it; if its `base` lacks the component's `frontmatter`, `contract` or `design` hash, or any of them is stale; if any of the component's requirements isn't listed under its `REMOVED Requirements`, or any of its assets under its `REMOVED Assets`, each with its base as `stale-base` requires; if the component is the root `clm`; or if the component has a child component the same change doesn't also remove, or that it creates. A file under the component's directory that is neither its `SPEC.md` nor an asset stays, and so does the directory. What still names the removed component or one of its requirements, such as another component's `depends_on` or `#### Needs`, or a backlog item's `components` or `blocks`, makes the change invalid by `resulting-graph`, unless the same change's deltas, or a backlog edit made before applying, drop it. A requirement or an asset left out, the root, and a child left behind or created are each reported under `apply/remove-node`, naming the component and what is left.

#### Scenario: node removed
- GIVEN component `a` with requirement `x` and asset `m.html`, and a change whose delta for `a` has `remove: true`, the bases of `a`'s frontmatter, contract and design, `x` under `REMOVED Requirements` and `m.html` under `REMOVED Assets`
- WHEN `clm apply <change>` runs
- THEN `specs/a/` no longer exists and the change is `applied`

#### Scenario: requirement left out
- GIVEN component `a` with requirements `x` and `y`, and a delta for `a` with `remove: true` listing only `x` under `REMOVED Requirements`
- WHEN `clm apply <change>` runs
- THEN it exits 1 naming `a/y`, and `specs/a/SPEC.md` is unchanged

#### Scenario: child left behind
- GIVEN components `a` and `a.b`, and a change whose only delta removes `a`
- WHEN `clm apply <change>` runs
- THEN it exits 1 naming `a.b`, and nothing is written

#### Scenario: root
- GIVEN a delta for `clm` with `remove: true`
- WHEN `clm apply <change>` runs
- THEN it exits 1 naming `clm`, and nothing is written

#### Scenario: removed and recreated as a parent
- GIVEN component `a`, and a change with a valid removal delta for `a` and a delta creating `a.c`
- WHEN `clm apply <change>` runs
- THEN it exits 1 naming `a` and `a.c`, and nothing is written

#### Scenario: parent and child removed
- GIVEN components `a` and `a.b`, and a change with a valid removal delta for each
- WHEN `clm apply <change>` runs
- THEN `specs/a/` no longer exists

#### Scenario: dependant edited in the same change
- GIVEN component `a`, component `b` with `depends_on: [a]`, and a change with a valid removal delta for `a` and a delta for `b` whose `## MODIFIED Frontmatter` sets `depends_on: []`, with the correct base
- WHEN `clm apply <change>` runs
- THEN it exits 0, `specs/a/` no longer exists and `b`'s `depends_on` is `[]`

#### Serves
- capability/propose-intent
- architecture/spec-is-current

### Requirement: declared-frontmatter-keys
When validation checks the frontmatter a delta produces, from a `## MODIFIED Frontmatter` or a `create: true` delta's `## Frontmatter`, it SHALL judge it as loading judges the frontmatter of a loaded component with the delta's component ID: by `model/valid-frontmatter` with the keys that live changes declare in `frontmatter_keys`, the change being applied among them, since it is live (`declared-frontmatter-keys`), and by `model/optional-nodes`, which refuses `optional: true` on the root `clm`. A key the model doesn't know and no live change declares, or any other failure of that check, SHALL be a violation under `apply/frontmatter` for a `## MODIFIED Frontmatter` and `apply/create-node` for a created component, with a message naming the component and the key; nothing SHALL be written.

#### Scenario: key the same change declares
- GIVEN change `x` with `frontmatter_keys: [colour]` and a delta for `a` whose `## MODIFIED Frontmatter` sets `colour: blue`, with the correct base
- WHEN `clm apply x` runs
- THEN it exits 0, `a`'s frontmatter has `colour: blue`, and `clm check` reports nothing

#### Scenario: key another live change declares
- GIVEN change `y`, `applied`, with `frontmatter_keys: [colour]`, and change `x`, declaring nothing, whose delta creates component `b` with `colour: red` in its `## Frontmatter`
- WHEN `clm apply x` runs
- THEN it exits 0 and `specs/b/SPEC.md` has `colour: red`

#### Scenario: undeclared key
- GIVEN change `x` declaring nothing, and a delta for `a` whose `## MODIFIED Frontmatter` sets `colour: blue`
- WHEN `clm apply x` runs
- THEN it exits 1 naming `a` and `colour`, and nothing is written

#### Scenario: optional root
- GIVEN change `x` with a delta for `clm` whose `## MODIFIED Frontmatter` sets `optional: true`, with the correct base
- WHEN `clm apply x` runs
- THEN it exits 1 naming `clm` and `optional`, and nothing is written

#### Needs
- model/optional-nodes

#### Serves
- capability/propose-intent
- architecture/checks-relax-before-use

### Requirement: resulting-graph
Once every delta of the change applies, whatever the change's status, validation SHALL check the spec graph the change would leave, the one `resulting` returns for it (`resulting-graph-call`): each component the change writes parsed from its new `SPEC.md` text as loading parses it (`model/graph-check`), with the keys live changes declare (`declared-frontmatter-keys`), at the path it is written to and with no assets, which no rule across components reads; each component it removes gone; each backlog item `clm apply` deletes with the change gone; every other component and backlog item, and the changes and decisions, as they are. It SHALL run the model's graph check on the current graph, with every item, and on that graph with the backlog's `workaround` items left out (the graph `resulting` returns keeps them), and the component parser on the current text of each existing component the change writes. A flag's `requirement` holds the version a build built (`model/parse-backlog`), which only the current graph gives: a flag on a requirement the change rewrites or removes stays valid while the change is pending (`workaround-flags`). Each violation of the resulting graph, from the parser or the graph check, that the current graph doesn't have (the same path, rule and message) SHALL be a violation under `apply/resulting-graph`, with the model's path and the message ``change `<id>` would leave: <model rule>: <model message>``; nothing SHALL be written. Violations the current graph already has are not reported again, so `clm check`, which validates every `proposed` or `approved` change against a graph that may have violations of its own, reports each once. A violation whose message the change alters, such as a dependency cycle it lengthens, counts as new.

#### Scenario: required node depends on a node made optional
- GIVEN component `a`, component `b` with `depends_on: [a]`, and change `x` whose delta for `a` sets `optional: true` in its `## MODIFIED Frontmatter`, with the correct base
- WHEN `clm apply x` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/optional-nodes`, `b` and `a`, and nothing is written

#### Scenario: dependant left behind
- GIVEN component `a`, component `b` with `depends_on: [a]`, and change `x` whose only delta is a valid removal of `a`
- WHEN `clm apply x` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/dependency-dag`, `b` and `a`, and `specs/a/SPEC.md` still exists

#### Scenario: backlog blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, a backlog item whose `blocks` names `a/y`, and change `c` whose delta for `a` removes `y` with its base
- WHEN `clm apply c` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/parse-backlog` and `a/y`, and nothing is written

#### Scenario: answered question blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, `backlog/q.md` with `kind: question` and `blocks: [a/y]`, and a proposed change `c` whose `resolves` is `[q]` and whose delta for `a` removes `y` with its base
- WHEN `clm check` runs, then `clm apply c`
- THEN `clm check` reports nothing under `apply/resulting-graph`, and `clm apply c` exits 0, removes `a/y` and deletes `backlog/q.md`

#### Scenario: resolved bug blocks a removed requirement
- GIVEN component `a` with requirements `x` and `y`, `backlog/b.md` with `kind: bug` and `blocks: [a/y]`, and change `c` whose `resolves` is `[b]` and whose delta for `a` removes `y` with its base
- WHEN `clm apply c` runs
- THEN it exits 1 naming `apply/resulting-graph`, `model/parse-backlog` and `a/y`, and `backlog/b.md` still exists

#### Scenario: answered question already blocks no requirement
- GIVEN component `a` with requirement `x`, `backlog/q.md` with `kind: question` and `blocks: [a/gone]`, naming no requirement, and change `c` whose `resolves` is `[q]` with a valid delta that modifies `x`
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/q.md` no longer exists, and the graph then loads with no violation

#### Scenario: violation already present
- GIVEN component `b` with `depends_on: [gone]`, naming no component, and a proposed change `x` whose valid delta modifies `a`'s Design
- WHEN `clm check` runs
- THEN it reports `b`'s `depends_on` once, under `model/dependency-dag`, and nothing under `apply/resulting-graph`

#### Scenario: workaround flag on a rewritten requirement
- GIVEN component `a` whose requirement `x` is at version `3fa2c9d1`, `backlog/workaround-a-x-1.md` with `kind: workaround` and `requirement: a/x@3fa2c9d1`, and a proposed change `c` whose delta for `a` modifies `x`'s text with its base
- WHEN `clm check` runs, then `clm apply c`, then `clm check` again
- THEN the first `clm check` reports nothing, `clm apply c` exits 0 and writes `x`'s new text, and the second `clm check` reports `backlog/workaround-a-x-1.md` under `model/parse-backlog` with `x`'s new version

#### Needs
- model/graph-check

#### Serves
- capability/catch-drift
- architecture/deterministic-enforcement

### Requirement: loaded-graph
A violation of the graph `clm apply` loads SHALL block the change unless the change fixes it: a violation the model's graph check (`model/graph-check`), or its component parser on the current text of a component the change writes, reports before the change, and neither reports on the graph the change would leave (`resulting-graph`), doesn't block it. Violations are compared by path, rule and message, counted, so a change that removes one of two copies fixes one. Every other violation of the loaded graph blocks it and SHALL be printed as the model reports it, under the model's rule: one the change leaves as it is, and one that neither of those reports, such as a change, backlog item, decision or human-layer file that doesn't parse, or a component whose `SPEC.md` doesn't parse, even when the change writes it. When a delta is itself invalid, or the change can't be applied for its status or doesn't exist, `clm apply` SHALL print those violations and every violation of the loaded graph, since what the change would fix can't be judged. The change's own violations come first. So a change can repair what the graph it starts from breaks, such as a human-layer entry the branch cuts while its deltas repoint the rules that served it, and a change that applies still leaves a graph that loads.

#### Scenario: violation the change fixes
- GIVEN component `b` with `depends_on: [gone]`, naming no component, and change `c` whose delta for `b` sets `depends_on: []` with the correct base
- WHEN `clm apply c` runs
- THEN it exits 0, and the graph then loads with no violation

#### Scenario: entry cut with the rules that served it
- GIVEN rule `a/y` whose `#### Serves` names `capability/gone`, a `CAPABILITIES.md` that no longer holds `gone`, and change `c` whose delta modifies `a/y` to serve `capability/kept`, an entry it holds
- WHEN `clm apply c` runs
- THEN it exits 0, and `a/y` serves `capability/kept`

#### Scenario: violation the change leaves
- GIVEN component `b` with `depends_on: [gone]`, and change `c` whose valid delta modifies only component `a`'s frontmatter
- WHEN `clm apply c` runs
- THEN it exits 1 printing `model/dependency-dag` for `specs/b/SPEC.md`, and nothing is written

#### Scenario: a file the model can't parse
- GIVEN another change whose `change.md` has no valid `status`, and change `c` with a valid delta
- WHEN `clm apply c` runs
- THEN it exits 1 printing the other change's violation, and nothing is written

#### Scenario: invalid delta and a broken graph
- GIVEN component `b` with `depends_on: [gone]`, and change `c` whose delta for `b` sets `depends_on: []` with a stale base
- WHEN `clm apply c` runs
- THEN it exits 1 printing both `apply/stale-base` and `model/dependency-dag`, and nothing is written

#### Serves
- capability/propose-intent
- architecture/deterministic-enforcement

#### Needs
- model/graph-check

### Requirement: resulting-graph-call
`resulting(graph, change)` SHALL return the spec graph `resulting-graph` checks for the change: each component the change writes parsed from its new `SPEC.md` text at the path it is written to, each component it removes gone, each backlog item `clm apply` deletes with the change (`answered-questions`, `promoted-item`) gone, and every other component and backlog item, and the changes, decisions and human layers, as in `graph`; or none when a delta of the change can't be applied, whatever the change's status. A written component whose new text the parser yields no component for is left out of the graph, and `resulting-graph` reports the parse. It SHALL write nothing.

#### Scenario: graph a removal leaves
- GIVEN component `a` with requirements `x` and `y`, and change `c` whose delta for `a` removes `y` with its base
- WHEN `resulting(graph, c)` is called
- THEN it returns a graph whose component `a` holds `a/x` and not `a/y`, with the source `clm apply c` would write to `specs/a/SPEC.md`, and that file is unchanged

#### Scenario: delta that can't apply
- GIVEN change `c` whose delta for `a` holds a stale base
- WHEN `resulting(graph, c)` is called
- THEN it returns none

#### Scenario: items apply deletes
- GIVEN `backlog/q.md` with `kind: question`, `backlog/b.md` with `kind: bug`, `backlog/h.md` with `kind: human-task`, `backlog/i.md` with `kind: idea`, and change `c` with `resolves: [q, b, h]`, `from: backlog/i` and a valid delta
- WHEN `resulting(graph, c)` is called
- THEN it returns a graph whose backlog holds `b` and `h` and neither `q` nor `i`, and all four files still exist

#### Scenario: promoted human task stays
- GIVEN `backlog/h.md` with `kind: human-task`, and change `c` with `from: backlog/h` and a valid delta
- WHEN `resulting(graph, c)` is called
- THEN it returns a graph whose backlog holds `h`

#### Serves
- capability/catch-drift

### Requirement: stale-elements
`clm-apply` SHALL offer `stale_bases(component, delta)`, `component` being the delta's component in the tree it is applied to or none when that tree lacks it. It SHALL return each element `delta` MODIFIES or REMOVES with a recorded base (its Contract and Design when it modifies them or removes the component, and each requirement it modifies or removes) whose base `stale-base` judges stale against `component`, as pairs of the element, named as `clm hashes` names it, and that base: those `component` holds in `clm hashes` order, then those it lacks (a requirement it doesn't have, or every element when `component` is none) in the delta's order: Contract, Design, the MODIFIED requirements, then the REMOVED ones. It SHALL leave out an element without a recorded base, a frontmatter base and an asset base. A REMOVED requirement, and a MODIFIED one whose block carries its own `#### Serves` block, are judged by the whole hash. `validate` SHALL judge each recorded Contract, Design and requirement base it compares with an element `component` holds as `stale_bases` judges it, so the base of an element the delta modifies or removes is stale for `validate` exactly when it is stale for `stale_bases`; `validate` also compares a base the delta records for a Contract or Design it leaves alone, which `stale_bases` leaves out, and reports a missing base and a component that no longer exists (`stale-base`) and a requirement that doesn't exist (`requirements`).

#### Scenario: serves edited under a delta without a block
- GIVEN component `a` with requirements `p` and `q`, and a delta modifying `p`, its block without a `#### Serves` block, and `q`, each with its hash as its base
- AND `p`'s `#### Serves` block and a scenario line of `q` are then edited
- WHEN `stale_bases(a, delta)` runs
- THEN it returns `("requirement:q", <q's base>)` alone, and `validate` reports `apply/stale-base` for `a/q` and not for `a/p`

#### Scenario: node gone
- GIVEN a delta modifying `a`'s Design with base `h1`, and a tree without component `a`
- WHEN `stale_bases(None, delta)` runs
- THEN it returns `("design", "h1")`

#### Serves
- architecture/hash-based-staleness
- architecture/reuse-first

### Requirement: answered-questions
`clm apply` SHALL delete `backlog/<slug>.md` for each `<slug>` the change's `resolves` names whose item has `kind: question`, among the files it removes, so that the spec it applies answers the question and the requirements the item `blocks` are released when the spec branch lands (`requirement-blocks`), not when the change is archived. It SHALL delete no item of another kind, and a `resolves` entry naming no item in `backlog/` SHALL delete nothing and be no violation. Each deleted item SHALL be among the files it prints as removed (`change-status`), as `removed backlog/<slug>.md`.

#### Scenario: answered question
- GIVEN `backlog/q.md` with `kind: question` and `blocks: [a/x]`, and change `c` whose `resolves` is `[q]` with a valid delta for `a`
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/q.md` no longer exists, and it prints `removed backlog/q.md`

#### Scenario: other kinds stay
- GIVEN `backlog/b.md` with `kind: bug` and `backlog/h.md` with `kind: human-task`, and change `c` whose `resolves` is `[b, h]` with a valid delta
- WHEN `clm apply c` runs
- THEN both items still exist

#### Scenario: already deleted
- GIVEN change `c` whose `resolves` is `[q]`, with no `backlog/q.md`, and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0 and reports nothing about `q`

#### Serves
- capability/keep-a-backlog

### Requirement: says-component
The messages of `clm apply` SHALL say component and components, never the old words, as `component-words` defines them: among them ``component `<id>` already exists``, ``component `<id>` no longer exists``, ``the root component `<id>` can't be removed`` and `the root component has no assets`. The rules a violation names (`apply/create-node`, `apply/remove-node`) are requirement IDs and keep their spelling.

#### Scenario: creating a component that exists
- GIVEN component `a`, and a change whose delta for `a` has `create: true`
- WHEN `clm apply <change>` runs
- THEN it exits 1 with a message holding ``component `a` already exists``, and nothing is written

#### Serves
- architecture/plain-terms

### Requirement: promoted-item
`clm apply` SHALL delete `backlog/<slug>.md` when the change's `from` is `backlog/<slug>` and that file exists, whatever the item's `kind` except `human-task`, among the files it removes, since `from` names the item the change was promoted from and the change now carries its work, so the item leaves the backlog when the spec branch lands and no later session takes it for open work. A `human-task` item SHALL stay: it is a step only the human takes outside the spec, and its deletion releases the requirements it `blocks` (`requirement-blocks`), so it stays until the human deletes it, as `answered-questions` keeps one. A `from` naming no item in `backlog/` SHALL delete nothing and be no violation, so a branch that deleted the item by hand applies as before; a `from` that isn't `backlog/<slug>` with a bare slug SHALL delete nothing either, since `check/backlog-entry-shape` reports it. The deleted item SHALL be among the files it prints as removed (`change-status`), as `removed backlog/<slug>.md`, once even when `resolves` names the same item.

#### Scenario: promoted idea
- GIVEN `backlog/i.md` with `kind: idea`, and change `c` with `from: backlog/i` and a valid delta for `a`
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/i.md` no longer exists, and it prints `removed backlog/i.md`

#### Scenario: any kind
- GIVEN `backlog/b.md` with `kind: bug`, and change `c` with `from: backlog/b` and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0 and `backlog/b.md` no longer exists

#### Scenario: human task stays
- GIVEN `backlog/h.md` with `kind: human-task` and `blocks: [a/x]`, and change `c` with `from: backlog/h` and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0 and `backlog/h.md` still exists

#### Scenario: already deleted
- GIVEN change `c` with `from: backlog/i`, no `backlog/i.md`, and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0 and reports nothing about `i`

#### Scenario: named in from and resolves
- GIVEN `backlog/q.md` with `kind: question`, and change `c` with `from: backlog/q`, `resolves: [q]` and a valid delta
- WHEN `clm apply c` runs
- THEN it exits 0, `backlog/q.md` no longer exists, and `removed backlog/q.md` is printed once

#### Serves
- capability/keep-a-backlog

### Requirement: commands-in-crate
`clm-apply` SHALL offer each of `clm apply`, `clm archive` and `clm hashes` as a call of its own (the Contract's) that takes the command's arguments as the command line gives them, the repository root its caller found, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. It SHALL print a violation as the line `model/violation-line` gives for it.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN the call is given the arguments of `clm apply` and two buffers
- THEN the buffers hold what `clm apply` prints to standard output and to standard error there, and the call returns the status `clm apply` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is given an option `clm apply` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the caller's process goes on

#### Needs
- model/violation-line

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: reapply
`reapply(graph, change)` SHALL return the files under `specs/` that the change's deltas write and remove on `graph`, the component files and assets `clm apply` would write and remove for them, or every violation of a delta that can't be applied. It SHALL NOT judge the change's status, run `resulting-graph` or `loaded-graph`, delete a backlog item (`answered-questions`, `promoted-item`) or write the change's `change.md`, and it SHALL write no file itself: `clm rebase` rebuilds `specs/` with it on a newer `main`, for a change the branch's own `clm apply` already applied (`sync/reapply`), and the load after the rebuild reports what the rebuilt graph breaks.

#### Scenario: an applied change that leaves a dependant
- GIVEN component `a`, component `b` with `depends_on: [a]`, and change `c`, `applied`, whose only delta is a valid removal of `a`
- WHEN `validate` and `reapply` run on it
- THEN `validate` reports `apply/change-status` and `apply/resulting-graph`, and `reapply` returns files and no violation

#### Scenario: an answered question is left
- GIVEN `backlog/q.md` with `kind: question`, and change `c`, `applied`, with `resolves: [q]` and a valid delta for `a`
- WHEN `reapply` runs on it
- THEN the files it returns hold `specs/a/SPEC.md` and neither `backlog/q.md` nor `changes/c/change.md`, and no file has changed

#### Scenario: a stale delta
- GIVEN change `c`, `applied`, whose delta for `a` holds a stale base
- WHEN `reapply` runs on it
- THEN it returns the `apply/stale-base` violation and no file
