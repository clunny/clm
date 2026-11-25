---
component: model
---

## MODIFIED Requirements

### Requirement: parse-changes <!-- base: e4c8b4b074a94961 -->
The model SHALL load every `changes/<id>/` and `changes/archive/<id>/` directory's `change.md`, `deltas/*.md` and `assets/<node-id>/**`. It SHALL reject a `change.md` whose frontmatter lacks a valid `status` (`proposed|approved|applied|implementing|done|abandoned`), or has keys other than that plus `human_burdens` (a list of short lines, default empty: what the change spends of the human's money or time, or can't undo, `gate/needs-review`), `author`, `from` (the backlog item it was promoted from), `resolves` (a list of backlog slugs the change resolves), `priority` (`P1` to `P5`, `P1` the highest, default `P5`, `priority-caps`), `frontmatter_keys` (a list of component frontmatter keys the change adds, default empty, `declared-frontmatter-keys`), `renames` (the renames the change records, default empty, `rename-records`), `moves` (the requirements the change removes that it merged into others, default empty, `move-records`) and `removes_capabilities` (a list of capability slugs, default empty: the entries of `CAPABILITIES.md` that the change's branch removes by hand, which the graph SHALL expose on the change's frontmatter as `removes_capabilities`, each slug once, a bare key declaring none as a bare `frontmatter_keys` does, `check.coverage/removed-capabilities`). It SHALL reject a `removes_capabilities` entry that isn't a slug an entry could have (lowercase ASCII letters, digits and `-`, and not empty, `human-layers`), naming the file and the entry, and SHALL accept one whose capability `CAPABILITIES.md` still holds or never held, since the key only declares and whether the capability is gone is read from that file. It SHALL reject any other `priority` value, and a `frontmatter_keys` entry that isn't a lowercase key name (a lowercase letter, then lowercase letters, digits and `_`). It SHALL read the key that is the word `plain-terms-renames` replaced with `human_burdens` as `human_burdens`, on a live change and an archived one, since an archived change keeps the keys it was written with (`archive-immutable`, `renamed-words-still-load`), and SHALL reject a `change.md` that sets both, naming both keys. It SHALL also accept and ignore `after` and `prefer_after`, which changes recorded while changes were ordered against each other. It SHALL reject `gate` on a live change, naming the key, and accept and ignore it on an archived one (`yes-no-review`). Files in a change's directory other than `change.md`, `deltas/*.md` and `assets/<node-id>/**`, a `plan.md` among them, are not parsed. A `resolves` entry MAY name a backlog item that no longer exists (`task-deletes-fixed-items`). It SHALL reject a delta whose frontmatter names no component, by `component` or by the old key (`component-keys`), that sets both `create: true` and `remove: true`, that sets `remove: true` and has a section other than `REMOVED Requirements` or `REMOVED Assets` or whose change holds files under `assets/<node-id>/` for its component, or that has a section other than `Frontmatter`, `Contract`, `Design`, `MODIFIED Frontmatter`, `MODIFIED Contract`, `MODIFIED Design`, `ADDED`/`MODIFIED`/`REMOVED Requirements` or `REMOVED Assets`. Text in a frontmatter section outside its ```` ```yaml ```` fence is rationale and is ignored, and a `## Frontmatter` or `## MODIFIED Frontmatter` section with no such fence SHALL be reported. A change whose `change.md` is missing, which is reported at the change's directory, `changes/<id>` or `changes/archive/<id>`, or has no `---` frontmatter block, or holds frontmatter this requirement rejects, SHALL be reported and not loaded, except that a live change setting `gate` is reported and stays loaded. A file under the change's `assets/` that isn't under `assets/<component-id>/`, or whose component the change has no delta for, SHALL be reported, naming the file or the component. Each line of a `## REMOVED Assets` section that starts `- ` SHALL be `` - `<path>` <!-- base: <hash> --> ``, its path relative to the component's `assets/` with no empty, `.` or `..` segment and no leading `/`: a line whose text before any `<!--` is such a path in backticks and that holds no comment of the form `<!-- base: <hash> -->` SHALL be reported naming the path, and a line whose text before any `<!--` is anything else SHALL be reported quoting the line. The base is what follows `base:` in the comment, trimmed, an empty one included. A line of the section that doesn't start `- ` is ignored. A delta MAY record the hashes it was written against, `base` in its frontmatter (`frontmatter`, `contract`, `design`, and `assets` by path) and a `<!-- base: <hash> -->` comment on a requirement's heading, and the model SHALL expose them on the delta. A change SHALL be *pending* while its status is `proposed` or `approved`. The model SHALL offer the parse of one `change.md`'s text alone, `parse_change_frontmatter`, which gives its frontmatter, or nothing when the text has no `---` block or its frontmatter breaks any rule this requirement states of `change.md`'s keys and their values (`status`, the keys it may set, `priority`, `frontmatter_keys`, `removes_capabilities`, both burdens keys set); it differs from loading in two things, checking nothing of `renames` or `moves`, not even that either is a list (`rename-records`, `move-records`), and accepting `gate` wherever the change lies.

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

#### Scenario: a rejected change
- GIVEN `changes/z/change.md` with `priority: P0`
- WHEN the model loads `changes/`
- THEN loading reports `changes/z/change.md`, and the graph holds no change `z`

#### Scenario: assets with no delta
- GIVEN change `c` with a delta for `a`, a file `changes/c/assets/loose.png` and a file `changes/c/assets/zzz/m.png`
- WHEN the model loads `changes/`
- THEN loading reports `model/parse-changes` twice, once for `changes/c/assets/loose.png` and once naming `zzz`

#### Scenario: removed asset lines
- GIVEN a delta whose `## REMOVED Assets` holds `` - `old.png` `` with no base, and deltas whose line names in turn `../SPEC.md`, `/etc/x`, an empty path, `a/./b` and `a//b`, each with a base
- WHEN the model loads `changes/`
- THEN loading reports `model/parse-changes` for each, the first naming `old.png`

#### Scenario: bases
- GIVEN a delta for `a` with `base: { contract: abc, assets: { m.html: abc } }`, removing requirement `x` under the heading `### Requirement: x <!-- base: def -->`, in a change with `status: proposed`
- WHEN the model loads `changes/`
- THEN the delta's Contract base is `abc`, its base for asset `m.html` is `abc`, the removed requirement's base is `def`, and the change is pending

#### Scenario: one change.md alone
- GIVEN the text of a `change.md` with `status: proposed` and `gate: HARD`, and one with `status: nope`
- WHEN `parse_change_frontmatter` reads each
- THEN the first gives its frontmatter with status `proposed`, and the second gives nothing

#### Scenario: a frontmatter section with no fence
- GIVEN a delta for `b` with `create: true` whose `## Frontmatter` section holds `title: B` and no ```` ```yaml ```` fence
- WHEN the model loads `changes/`
- THEN loading reports `model/parse-changes` for the delta, naming `Frontmatter`


#### Scenario: moves alone
- GIVEN the text of a `change.md` with `status: proposed` and `moves: sprocket`
- WHEN `parse_change_frontmatter` reads it
- THEN it gives its frontmatter with status `proposed`

## ADDED Requirements

### Requirement: move-records
The model SHALL load a change's `moves` as a list of entries in file order, each a mapping with exactly the keys `requirement`, naming a requirement the change removes, and `to`, naming the requirement it merged into, and expose them on the change. On a change outside the archive it SHALL report as `model/move-records`, naming the `change.md`, a `moves` that isn't a list, and, naming the entry's 1-based position too: an entry without `requirement` or `to`, or with any other key; a value that isn't `<component>/<slug>`, a component ID, `/` and a slug (`[a-z0-9]+(-[a-z0-9]+)*`); a `requirement` that no delta of the change for its component lists under `## REMOVED Requirements`; a `to` that no delta of the change for its component adds under `## ADDED Requirements` or modifies under `## MODIFIED Requirements`, on the same component as `requirement` or another; and a `requirement` that an earlier entry names. A change with such a violation SHALL NOT be loaded. An archived change's `moves` SHALL be loaded unchecked, leaving out what isn't a list and each entry that isn't a mapping of exactly `requirement` and `to` to strings (`archive-immutable`). What a recorded move does is `check.coverage/status-report`'s.

#### Scenario: a merge recorded
- GIVEN `changes/x/change.md` with `moves` holding `requirement: a/old` with `to: a/new` and `requirement: a/gone` with `to: b/kept`, and deltas of `x` for `a`, removing `old` and `gone` and adding `new`, and for `b`, modifying `kept`
- WHEN the model loads `changes/`
- THEN `x` holds the two moves in that order, and loading reports nothing

#### Scenario: a move the deltas don't make
- GIVEN `changes/x/change.md` whose delta for `a` removes `old` and `gone` and adds `new`, with `moves` holding in turn `requirement: a/kept` with `to: a/new`, `requirement: a/old` with `to: a/new`, `requirement: a/gone` with `to: a/other`, `requirement: a/old` with `to: a/new` again, and an entry with `requirement: a/gone`, `to: new` and `why: merged`
- WHEN the model loads `changes/`
- THEN loading reports `model/move-records` and `changes/x/change.md` with entries `1`, `3`, `4` and `5`, and the graph holds no change `x`

#### Scenario: moves that aren't a list
- GIVEN `changes/x/change.md` with `moves: sprocket`, and `changes/archive/y/change.md` with `moves: sprocket`
- WHEN the model loads `changes/`
- THEN loading reports `model/move-records` and `changes/x/change.md`, the graph holds no change `x`, and it holds `y` with no moves

#### Serves
- architecture/component-reconciliation
