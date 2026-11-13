---
component: model
---

## MODIFIED Requirements

### Requirement: parse-changes <!-- base: e3b5847d6cfa5084 -->
The model SHALL load every `changes/<id>/` and `changes/archive/<id>/` directory's `change.md`, `deltas/*.md` and `assets/<node-id>/**`. It SHALL reject a `change.md` whose frontmatter lacks a valid `status` (`proposed|approved|applied|implementing|done|abandoned`), or has keys other than that plus `human_burdens` (a list of short lines, default empty: what the change spends of the human's money or time, or can't undo, `gate/needs-review`), `author`, `from` (the backlog item it was promoted from), `resolves` (a list of backlog slugs the change resolves), `priority` (`P1` to `P5`, default `P5`, `priority-caps`) `frontmatter_keys` (a list of component frontmatter keys the change adds, default empty, `declared-frontmatter-keys`), `renames` (the renames the change records, default empty, `rename-records`) and `removes_capabilities` (a list of capability slugs, default empty: the entries of `CAPABILITIES.md` that the change's branch removes by hand, which the graph SHALL expose on the change, so the markers that cite them read as obsolete while the change is outside the archive, `check.coverage/removed-capabilities`). It SHALL reject a `removes_capabilities` entry that isn't a slug an entry could have (lowercase ASCII letters, digits and `-`, and not empty, `human-layers`), naming the file and the entry, and SHALL accept one whose capability `CAPABILITIES.md` still holds or never held, since the key only declares and whether the capability is gone is read from that file. It SHALL reject any other `priority` value, and a `frontmatter_keys` entry that isn't a lowercase key name (a lowercase letter, then lowercase letters, digits and `_`). It SHALL read the key that is the word `plain-terms-renames` replaced with `human_burdens` as `human_burdens`, on a live change and an archived one, since an archived change keeps the keys it was written with (`archive-immutable`, `renamed-words-still-load`), and SHALL reject a `change.md` that sets both, naming both keys. It SHALL also accept and ignore `after` and `prefer_after`, which changes recorded while changes were ordered against each other; nothing reads them any more (`node-reconciliation`), and archived changes keep theirs (`archive-immutable`). It SHALL reject `gate` on a live change, naming the key, and accept and ignore it on an archived one, which recorded the change's review tier before review became yes or no (`yes-no-review`). Files in a change's directory other than `change.md`, `deltas/*.md` and `assets/<node-id>/**`, a `plan.md` among them, are not parsed. A `resolves` entry MAY name a backlog item that no longer exists, since the build that fixes an item deletes it before the change is archived (`task-deletes-fixed-items`); `gate/resolves-named-items` catches an entry that never named one. It SHALL reject a delta whose frontmatter names no component, by `component` or by the old key (`component-keys`), that sets both `create: true` and `remove: true`, that sets `remove: true` and has a section other than `REMOVED Requirements` or `REMOVED Assets` or whose change holds files under `assets/<node-id>/` for its component, or that has a section other than `Frontmatter`, `Contract`, `Design`, `MODIFIED Frontmatter`, `MODIFIED Contract`, `MODIFIED Design`, `ADDED`/`MODIFIED`/`REMOVED Requirements` or `REMOVED Assets`. Text in a frontmatter section outside its ```` ```yaml ```` fence is rationale and is ignored.

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

#### Scenario: removed capabilities
- GIVEN `changes/x/change.md` with `removes_capabilities: [watch, old-runner]`, `changes/y/change.md` with no `removes_capabilities`, and `changes/z/change.md` with `removes_capabilities: [capability/watch]`
- WHEN the model loads `changes/`
- THEN `x` is loaded removing the capabilities `watch` and `old-runner`, `y` removing none, and loading reports `z`'s `change.md` and `capability/watch`

#### Scenario: renames
- GIVEN `changes/x/change.md` with `renames` holding one `term: sprocket` entry with `to: cogs`, and `changes/y/change.md` with no `renames`
- WHEN the model loads `changes/`
- THEN `x` is loaded with that one rename and `y` with none
