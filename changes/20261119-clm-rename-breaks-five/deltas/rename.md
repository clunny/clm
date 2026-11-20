---
component: rename
base: { design: "7f5ae6b82a35166d" }
---

## MODIFIED Design
**Why one record** (`rename-by-replay`). A rename is mechanical, but its edits reach code paths (test markers) and the human layers, which a spec branch can't edit without being `mixed` or needing review. The pairs in `change.md` are the whole intent: everything else on the branch follows from them and from the base, so a program can reproduce it, and the gate trusts the branch exactly when it reproduces. `rename` writes ordinary deltas, which `clm apply` and the removal and move machinery (`node-removal`, `progress-follows-moves`) read as they read any.

**Reading the record.** `rename` reads the change from the graph `clm_model::load` loads: its status, its `renames` in order and its `resolves` are the loaded `Change`'s (`model/rename-records`), never a second parse of `change.md`. It writes `change.md` only by its `status:` line, so the change applies and is archived with its `renames` as written.

**Generation.** From those renames `rename` builds one map from each old ID to its new one: a component rename maps the component and each descendant (`a` → `b` gives `a.c` → `b.c`) and each of their requirements (`a/x` → `b/x`); a requirement rename maps the one ID. Terms are a list of `(old, new)` pairs, applied in order after the ID map. Each component's new text is computed in memory (frontmatter, Contract, Design, requirements); the deltas are the difference from the graph's text, written to `changes/<id>/deltas/` once what was there is cleared, each MODIFIED element based on `clm-apply`'s `hashes`. A renamed component gets a `remove: true` delta and a `create: true` delta with its rewritten text; its assets are copied to `changes/<id>/assets/<new-id>/` and listed under the old component's `REMOVED Assets`. The steps run in this order: the deltas and assets are written, then the human layers and the backlog items' references, so the graph `clm_apply::apply` checks holds the new names; `clm_apply::apply` writes `specs/` and marks the change `applied`; the markers are rewritten (their IDs, then their versions once the leftovers are found: **Markers**); the other changes' deltas are rebased (**Other changes' deltas**); the flags' versions are re-recorded (`flag-rerecord`); the `status:` line is set to `done` and the change archived with `clm_apply::archive`; and the `resolves` items are deleted.

**What a reference is.** Structured references are whole values: `depends_on` entries, `serves` entries and `#### Serves`, `#### Needs` and `#### Enforced by` lines, and backlog `components`, `blocks` and `requirement` values. A list's items are its values whether it is written in flow style (`[a, b]`) or, in a backlog item, which `rename` writes itself, in block style (a `- a` line per item under the key), and a value is the same value quoted or bare. A component's frontmatter changes only through a delta, which can't modify a value on more than one line (`apply/frontmatter`), so a block-style list there that holds an old ID is a refusal (`refusals`), never a half-rewritten list. No other key holds a reference: `successor` holds a change ID, which a token equal to an old component ID doesn't make one. In spec text (requirements with their scenarios, Contracts, Designs and the human-layer entries), a reference is a code span whose whole text is an old requirement or entry ID, one that holds a `/`, or a code span whose whole text is an old slug when no other requirement, entry or decision (`docs/decisions/<slug>.md`) has that slug, the bare slug CLM's prose cites a rule by. A component's ID, in prose or in a code span (the word `apply`), is a word, not a reference: renaming it is a `term` pair. The lines of fenced code hold no reference: a fence shows an example, whose IDs are the example's own.

**Terms.** An occurrence of a term counts when neither neighbour continues an ID or a path, as `term-swap` gives the characters. So `sprocket-scoped-context`, `crates/sprocket/` and `req:sprocket/x` are untouched, and `the sprocket.` is swapped. Headings (every line starting with `#`) and the lines of `#### Needs`, `#### Serves` and `#### Enforced by` blocks are never swapped: a heading holds a requirement's slug or an entry's, which only a `requirement` pair renames, and the ID lines are references. A term in fenced code is swapped as anywhere else, since a word is the same word in an example. A line of fenced code is never a heading, even one starting with `#` (a shell comment, or a heading in an example), so it is swapped too, and it starts and ends no block: the ID lines of a block are the lines after its heading outside fenced code, up to the next heading outside it, and a fence opened in the block holds no ID line.

**Markers.** The files are those `check.coverage`'s `scan` reads (`check.coverage/markers-from-files`), and the markers its `marker_spans` finds in each, with their byte ranges (`check.coverage/marker-spans`), so a marker `rename` rewrites is exactly one the coverage counts, never one found by a second reading of the pattern. Each is replaced in place by its byte range. That holds under `changes/archive/` and `docs/decisions/` too, which `references` otherwise leaves as history (`rename-by-replay`): a marker there that kept an old ID would name no requirement, and `check/dangling-markers` would fail the rename branch over a file the author never touched. The marker is the only text `rename` edits there.

**Which markers stay behind** (`marker-rewrite`, `rename-leaves-disagreeing-proof-behind`). Re-citing a reworded requirement at its new version claims its tests prove the new text. When `references` rewrote an old ID or slug in the requirement's text and the code still names that old one, the claim is false: the code still says what the old text said, so the marker keeps its version and the component's build brings the code to the new name. The leftovers (**Leftovers**) are found after the markers name their new IDs and before their versions are chosen, so a marker `rename` rewrites is never a leftover, and the versions change no line the search reads. A term's leftover never holds a marker back: a word in code may be a sense of the word the swap didn't mean (`component-words`), which the author judges from the `still names` lines, while an old ID names one rule.

**Other changes' deltas** (`other-deltas`, `rename-rebases-other-deltas`). A term swap or a reference rewrite changes the text, and so the hash, of elements another live change's delta may MODIFY or REMOVE; left alone, that delta is stale (`apply/stale-base`) and `clm check` fails the rename branch over a change it doesn't own. The branch can't rebase that delta by hand and still replay (`replay`), and `sync.regenerate`'s `other-delta-merge` would commit a merge the replay doesn't reproduce, so `rename` rebases it as part of its output. After `clm_apply::apply`, for each such change, in ID order, and each of its deltas for a component the graph holds both before and after the rename (a component only the rename creates, a renamed component's new ID, is left out, since nothing before the rename accepted a base of it), it takes the elements `clm_apply::stale_bases` names against the component after the rename and not against it before, and a frontmatter base, which `stale_bases` leaves out, that equals the component's `frontmatter` hash before the rename and not after: those are exactly the ones the rename moved. A base is only ever replaced, never added, since each such element records one. An element the delta MODIFIES has its text in the delta rewritten by the edits `rename` makes to spec text (a Contract, Design or requirement block as `references` and `term-swap` edit it; a MODIFIED Frontmatter by the value edit `references` makes to frontmatter), and the new base is written where `sync.regenerate`'s merge writes one (`sync.regenerate/stale-delta-merge`: the delta frontmatter's base key, or a requirement heading's `<!-- base: -->` comment), located with `clm_model::outline`'s spans. No three-way merge is needed: the edits are a function of the text, so applying them to the delta's text keeps the delta's own edits and adds the rename's, with no conflict to report and no `git` process to run, so the wasm build is unaffected. The replay reproduces these edits, since `rename` makes them from the base, as it reproduces the flags' versions (`flag-rerecord`).

**Leftovers.** After writing, `rename` searches the tracked text files outside `specs/`, `changes/`, `ARCHITECTURE.md` and `CAPABILITIES.md`, and outside `docs/decisions/`, which record history, for each old ID as a whole token, each old term as `Terms` counts it and, for each requirement or entry rename, its old slug as a code span's whole text (`` `slim-nodes` ``, the form `CLAUDE.md` cites an entry by) when `references` would rewrite that span in spec text. A component rename keeps each requirement's slug, so it adds no slug. These are skills, guides, `CLAUDE.md` and code, whose edits are code changes or the author's prose.

**Replay** (`replay`). `replays` reads both revisions through `clm-git`: the change `head` adds (live or new archived, from `clm-fingerprint`'s `added_changes` over the two `git ls-tree -r --full-tree` listings, the shape `gate` and `clm land` use), its `change.md` at `head`, and `base`'s tree, extracted whole with `extract_tree` into a temporary directory (`vcs/extract-as-checkout`). It places the `change.md` at `changes/<id>/change.md` with its `status:` line set to `proposed`, runs `rename` there, and compares what is left with `head`'s listing, kind by kind: each regular file by path, mode and blob ID (`git hash-object --no-filters` of each file the rename left, and its owner's executable bit for the mode); each symbolic link by path and the blob ID of the path it names, read from the directory without following it, since a rename removes a link with a component's directory and writes an asset it read through a link as a regular file, so what `head` must hold is what the rename left, not what `base` held; and each submodule against `base`'s listing, since the extracted tree holds a submodule only as an empty directory and the rename writes none. The executable bit is read only where the platform has one: the crate also builds for `wasm32-unknown-unknown`, where nothing calls `replays`. A new file under `backlog/` is left out because a session files defect reports on any branch; only a regular file is one. It reads `renames` from the blob's frontmatter, since no graph is loaded before the tree is extracted.

**Wasm.** `clm-gate` is in the `web` Worker's wasm build, so `clm-rename` and what it pulls in build for `wasm32-unknown-unknown`, as `clm-gate`'s other dependencies do; the Worker never calls it. The checks' wasm build of `clm-web` shows it.

## MODIFIED Requirements

### Requirement: refusals <!-- base: ba98c4c4e93b5b8f -->
`clm rename <change-id>` SHALL exit 1, writing nothing and naming the cause on standard error, when the change isn't live with status `proposed`; when it records no `renames`; when loading reports a violation against the change's `change.md`, such as a `model/rename-records` entry, naming the model's rule and message; when a renamed component or requirement doesn't exist, or its new ID already does; when it renames both a component and a requirement of that component or of one of its descendants; when another live change whose status is `proposed` or `approved` has a delta for a renamed component or one of its descendants, or one that modifies or removes a renamed requirement; when a term occurs nowhere in spec text as `term-swap` counts it; when a component's frontmatter holds a `depends_on` or `serves` value that spans more than one line and names a renamed ID, naming the component and the key, since its delta can't modify that value (`apply/frontmatter`); and when the change's `deltas/` holds a directory, naming it.

#### Scenario: target exists
- GIVEN components `a` and `b`, and change `x`, `proposed`, with `renames` holding `component: a` and `to: b`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `b`, and the tree is unchanged

#### Scenario: another change targets it
- GIVEN requirement `a/old`, change `y`, `proposed`, whose delta for `a` modifies `old`, and change `x`, `proposed`, with `renames` holding `requirement: a/old` and `to: new`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `y` and `a/old`, and the tree is unchanged

#### Scenario: another change already applied
- GIVEN requirement `a/old`, change `y`, `applied`, whose delta for `a` removes `old`, and change `x`, `proposed`, with `renames` holding `requirement: a/old` and `to: new`
- WHEN `clm rename x` runs
- THEN it carries the rename out

#### Scenario: term absent
- GIVEN change `x`, `proposed`, with `renames` holding `term: grommet stub` and `to: spare part`, and no spec text holding `grommet stub`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `grommet stub`, and the tree is unchanged

#### Scenario: a move the model rejects
- GIVEN components `a` and `b`, and change `x`, `proposed`, with `renames` holding `component: a` and `to: b.a`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `model/rename-records` and entry `1`, and the tree is unchanged

#### Scenario: a component and its own rule
- GIVEN component `a` with requirement `x`, and change `r`, `proposed`, with `renames` holding `component: a` with `to: b` and `requirement: a/x` with `to: y`
- WHEN `clm rename r` runs
- THEN it refuses naming `a/x` and saying to rename them in separate changes, `specs/a/SPEC.md` is still there and `specs/b/` isn't

#### Scenario: a block-style list in a component
- GIVEN components `a` and `c`, component `b` whose frontmatter holds `depends_on:` followed by the lines `  - a` and `  - c`, and change `r`, `proposed`, with `renames` holding `component: a` and `to: d`
- WHEN `clm rename r` runs
- THEN it exits 1 naming `b` and `depends_on`, and the tree is unchanged

#### Needs
- model/rename-records

### Requirement: references <!-- base: ac4e2d433c00c7f1 -->
`clm rename` SHALL rewrite every reference to a renamed ID: whole values in `depends_on`, `serves`, `#### Serves`, `#### Needs` and `#### Enforced by` and in backlog items' `components`, `blocks` and `requirement`, each item of a list a value whether the list is written in flow style (`[a, b]`) or, in a backlog item, in block style (a `- a` line per item), and a quoted value as a bare one, keeping its quotes; and, in spec text, each code span whose whole text is an old requirement or entry ID, or an old slug that no other requirement, entry or decision has. A component's ID in a code span is a word and stays, and no reference is rewritten on a line of fenced code, a line after one that starts, after any indentation, with ```` ``` ```` or `~~~` and before the next line that does. Beyond these references, the terms `term-swap` swaps, the markers `marker-rewrite` rewrites, the versions `flag-rerecord` re-records and the other changes' delta elements `other-deltas` rebases, it SHALL rewrite nothing else, the value of any other frontmatter key, such as `successor`, included, and it SHALL edit no file under `changes/archive/` or `docs/decisions/` but to rewrite a marker there as `marker-rewrite` does, which `check/dangling-markers` would otherwise report as naming no requirement.

#### Scenario: references across the repository
- GIVEN requirement `a/x` renamed to `a/y`, component `b` whose requirement `z` holds `#### Needs` `- a/x` and the text ``(`a/x`, `x`)``, no other requirement or decision with slug `x`, `ARCHITECTURE.md` naming `a/x` under `#### Enforced by`, and `backlog/q.md` with `blocks: [a/x]`
- WHEN `clm rename` runs
- THEN `b/z`'s `#### Needs` names `a/y` and its text reads ``(`a/y`, `y`)``, `ARCHITECTURE.md` names `a/y`, and `backlog/q.md` has `blocks: [a/y]`

#### Scenario: ambiguous slug
- GIVEN requirement `a/x` renamed to `a/y`, and component `b` with its own requirement `x` and a Design citing `` `x` ``
- WHEN `clm rename` runs
- THEN `b`'s Design still cites `` `x` ``

#### Scenario: a component's bare name
- GIVEN component `apply` renamed to `fold`, and a Design reading `` `apply` writes `specs/` ``
- WHEN `clm rename` runs
- THEN that text is unchanged

#### Scenario: an item's components
- GIVEN component `a.b`, a change whose `renames` holds `component: a.b` with `to: a.c`, and `backlog/x.md` with `components: [a.b]`
- WHEN `clm rename` runs on it
- THEN `backlog/x.md` holds `components: [a.c]`

#### Scenario: a reference in fenced code
- GIVEN requirement `a/x` renamed to `a/y`, and component `b` whose Design holds a fenced code block with the line `` see `a/x` `` and, outside it, the line `` see `a/x` ``
- WHEN `clm rename` runs
- THEN the line in the block is unchanged, and the line outside it reads `` see `a/y` ``

#### Scenario: a list in block style
- GIVEN components `a` and `c`, `backlog/x.md` whose frontmatter holds `components:` followed by the lines `  - a` and `  - c`, and change `r`, `proposed`, with `renames` holding `component: a` and `to: d`
- WHEN `clm rename r` runs
- THEN `backlog/x.md`'s `components` lines read `  - d` and `  - c`, and the rest of the file is unchanged

#### Scenario: a key that holds no reference
- GIVEN component `a`, component `b` whose frontmatter holds `successor: "a"` and `depends_on: [a]`, and change `r`, `proposed`, with `renames` holding `component: a` and `to: d`
- WHEN `clm rename r` runs
- THEN `b`'s frontmatter holds `successor: "a"` and `depends_on: [d]`

#### Scenario: a marker in the archive
- GIVEN requirement `a/x`, an archived change `old` whose `changes/archive/old/assets/a/t.rs` holds a test citing `a/x` with no version, `docs/decisions/d.txt` reading `` `a/x` ``, and change `r`, `proposed`, with `renames` holding `requirement: a/x` and `to: y`
- WHEN `clm rename r` runs
- THEN `changes/archive/old/assets/a/t.rs` differs from before only in that its marker cites `a/y`, `docs/decisions/d.txt` is unchanged, and every marker the coverage reads in the tree names a requirement the spec holds

### Requirement: term-swap <!-- base: d6b8103731b41e61 -->
For a `term` rename `clm rename` SHALL replace, case-sensitively, with the new text, each occurrence of the term in spec text (each component's requirements with their scenarios, Contract and Design, and the entries of `ARCHITECTURE.md` and `CAPABILITIES.md`) whose neighbours don't continue an ID or a path: the character before it is absent, or is neither a letter, a digit nor one of `-`, `_`, `/`, `.` and `:`; and the character after it is absent, or is `.` or `:` followed by whitespace or the line's end, or is neither a letter, a digit nor one of `-`, `_`, `/`, `.` and `:`. It SHALL leave alone every heading, a line outside fenced code (as `references` gives it) that starts with `#`, and the lines of `#### Needs`, `#### Serves` and `#### Enforced by` blocks, a block's lines being those outside fenced code after its heading and before the next heading; SHALL swap a term on a line of fenced code as on any other line, one starting with `#` included, since a line of fenced code is no heading and starts or ends no block; and SHALL give each component whose text changes a delta modifying each changed element.

#### Scenario: a word in prose
- GIVEN component `a` whose Design reads `the sprocket of a component` and whose requirement `x` reads `print the sprocket.`, and change `r` with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN the Design reads `the cogs of a component` and `a/x` reads `print the cogs.`

#### Scenario: part of an ID or a path
- GIVEN spec text holding `the sprocket`, `sprocket-scoped-context`, `crates/sprocket/` and `Sprocket`, and change `r` with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `the sprocket` reads `the cogs`, and `sprocket-scoped-context`, `crates/sprocket/` and `Sprocket` are unchanged

#### Scenario: a heading
- GIVEN component `a` with requirement `sprocket`, whose heading is `### Requirement: sprocket`, and change `r` with `renames` holding `term: sprocket` and `to: cogs`, and `the sprocket` in `a`'s Design
- WHEN `clm rename r` runs
- THEN `a` still holds requirement `sprocket`

#### Scenario: a term before punctuation
- GIVEN a Design reading `sprocket: yes. Also sprocket.x and sprocket:x`, and change `r` with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN the Design reads `cogs: yes. Also sprocket.x and sprocket:x`

#### Scenario: a term in fenced code
- GIVEN a Design holding a fenced code block with the line `print the sprocket`, and change `r` with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN the line reads `print the cogs`

#### Scenario: a heading in fenced code
- GIVEN requirement `a/x` whose text holds a fenced code block with the lines `# the sprocket` and `#### Needs`, followed, after the block, by the line `print the sprocket`, and change `r` with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN the line in the block reads `# the cogs`, and the line after the block reads `print the cogs`

### Requirement: flag-rerecord <!-- base: 6809c25b50559290 -->
`clm rename` SHALL rewrite the `requirement` value of each backlog item that records one (a `workaround` flag's `<component>/<slug>@<version>`, `workaround-flags`) and names a renamed requirement or one whose text the change rewords, as `marker-rewrite` rewrites a marker: a version that was the requirement's current version SHALL become its version after the change, under the new ID, and any other version SHALL stay as it is under the new ID, so a flag that was current before the rename is current after it and `clm check` raises no `model/parse-backlog` violation for it. It SHALL find the item's `requirement:` line in its frontmatter however the value is spaced after the key, and rewrite the ID and version inside the value's quotes, single or double, or the bare value, never only a value whose line matches `requirement: <id>@<version>` exactly. Nothing else in the item SHALL change, the value's quotes included.

#### Scenario: a flagged requirement reworded
- GIVEN requirement `a/x` at version `11111111` reading `the sprocket`, `backlog/f.md` with `kind: workaround` and `requirement: a/x@11111111`, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `backlog/f.md`'s `requirement` is `a/x` at its new version, the rest of the file is unchanged, and loading reports no `model/parse-backlog` violation for `backlog/f.md`

#### Scenario: a flagged requirement renamed
- GIVEN requirement `a/x` at version `11111111`, `backlog/f.md` with `kind: workaround` and `requirement: a/x@11111111`, and change `r`, `proposed`, with `renames` holding `requirement: a/x` and `to: y`
- WHEN `clm rename r` runs
- THEN `backlog/f.md`'s `requirement` is `a/y` at its version after the change, and loading reports no `model/parse-backlog` violation for `backlog/f.md`

#### Scenario: a quoted flag
- GIVEN requirement `a/x` at version `11111111` reading `the sprocket`, `backlog/f.md` with `kind: workaround` and the line `requirement:  "a/x@11111111"`, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN that line reads `requirement:  "a/x@<new>"`, `<new>` the version of `a/x` after the change, and the rest of the file is unchanged

#### Serves
- architecture/plain-terms

### Requirement: other-deltas <!-- base: 361ed0c7813401cf -->
`clm rename` SHALL, for each live change other than its own whose status is `proposed` or `approved`, rebase each element (frontmatter, Contract, Design or requirement) that one of the change's deltas MODIFIES or REMOVES and whose recorded base `apply/stale-base` accepts against the graph before the rename and refuses against the graph after it, so never an element of a component the graph before the rename doesn't hold, such as the new ID of a component the rename renames: an element the delta MODIFIES SHALL have its text in the delta rewritten by the edits `references` and `term-swap` make to spec text, and each such element SHALL record the element's hash after the rename as its base, so `apply/stale-base` reports nothing for an element it rebased. It SHALL change nothing else in the other change's files, and SHALL print one `rebased <element> of <component> in <delta path>` line per element it rebased, `<element>` named as `clm hashes` names it and `<delta path>` relative to the root, in order of change ID, then of component ID, then as `clm hashes` orders a component's elements, after the `renamed` lines and before the `still names` lines.

#### Scenario: a reworded Contract another change modifies
- GIVEN component `a` whose Contract reads `Print the sprocket.` and whose Design reads `Read the file.`, change `y`, `proposed`, whose delta for `a` modifies the Contract to `Print the sprocket.` and `Exit 0.` and the Design to `Read the file twice.`, both on their current bases, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `y`'s delta modifies the Contract to `Print the cogs.` and `Exit 0.` on the base `clm hashes a` gives for the Contract, its Design text and base are unchanged, it prints `rebased contract of a in changes/y/deltas/a.md`, it exits 0, and loading reports no violation for `y` and `apply/stale-base` reports nothing for it

#### Scenario: a removed requirement
- GIVEN requirement `a/x` reading `the sprocket`, change `y`, `proposed`, whose delta for `a` removes `x` on its current base, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `y`'s delta still removes `x`, on the base `clm hashes a` gives for `x`, and nothing else in it changed

#### Scenario: a rename branch that rebased another delta
- GIVEN a branch cut from `base`, which holds change `y` as in the first scenario, on which `clm rename r` ran and its result was committed
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is true

#### Scenario: an element stale before the rename
- GIVEN change `y`, `proposed`, whose delta for `a` modifies the Design on a base that is already stale, and change `r`, `proposed`, whose term swap rewords `a`'s Design
- WHEN `clm rename r` runs
- THEN `y`'s delta is unchanged and no `rebased` line names it

#### Scenario: a component only the rename creates
- GIVEN component `a` with requirement `x` and a Design reading `` See `a/x`. ``, which the rename rewrites, no component `b`, change `y`, `proposed`, whose delta for `b`, with no `create: true`, modifies the Design on the base `clm hashes a` gives for `a`'s Design, and change `r`, `proposed`, with `renames` holding `component: a` and `to: b`
- WHEN `clm rename r` runs
- THEN `y`'s delta is unchanged and no `rebased` line names it, since no base of it was accepted against the graph before the rename

