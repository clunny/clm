---
title: Rename
status: active
owners: ["@clunny"]
depends_on: [model, apply, check.coverage, gate.fingerprint, vcs]
paths: ["crates/clm-rename/**"]
serves: [architecture/plain-terms]
---

## Contract
`clm rename <change-id>` carries out the renames a live, `proposed` change records in its `change.md`'s `renames` (`model/rename-records`): it generates the change's deltas, applies them, rewrites what cites the renamed IDs and terms, test markers included, rebases the other live changes' deltas whose bases the rename moves (`other-deltas`), archives the change as `done` and deletes the items its `resolves` names, so the branch lands with nothing left pending but the requirements whose code still names an ID the rename rewrote in their text (`marker-rewrite`). It writes the working tree and commits nothing. It prints one `renamed <old> -> <new>` line per rename, then one `rebased <element> of <component> in <delta path>` line per element of another change's delta it rebased, then one `still names <old> <file>:<line>` line per mention it leaves for the author to judge, and exits 0; it exits 1 on a refusal, writing nothing.

The crate `clm-rename` exposes `rename(root, change_id)`, the command on a checkout, and `replays(repo, base, head)`, whether `head`'s tree is exactly what `rename` produces from `base`'s with the change `head` adds (`replay`). `gate` calls `replays` to classify a branch, and `sync` calls both to regenerate a rename branch instead of rebasing it (`regenerate-dont-merge`).

## Design
**Why one record** (`rename-by-replay`). A rename is mechanical, but its edits reach code paths (test markers) and the human layers, which a spec branch can't edit without being `mixed` or needing review. The pairs in `change.md` are the whole intent: everything else on the branch follows from them and from the base, so a program can reproduce it, and the gate trusts the branch exactly when it reproduces. `clm apply` and the delta format are unchanged: `rename` writes ordinary deltas, and the removal and move machinery (`node-removal`, `progress-follows-moves`) reads them as it reads any.

**Reading the record.** `rename` reads the change from the graph `clm_model::load` loads: its status, its `renames` in order and its `resolves` are the loaded `Change`'s (`model/rename-records`), never a second parse of `change.md`, and a violation the load reports against the change's `change.md` refuses the rename with the model's message (`refusals`). It writes `change.md` only by its `status:` line, so the change loads, applies and is archived with its `renames` as written, and the archived copy passes `model/parse-changes` like any other.

**Generation.** From those renames `rename` builds one map from each old ID to its new one: a component rename maps the component and each descendant (`a` → `b` gives `a.c` → `b.c`) and each of their requirements (`a/x` → `b/x`); a requirement rename maps the one ID. Terms are a list of `(old, new)` pairs, applied in order after the ID map. Each component's new text is computed in memory (frontmatter, Contract, Design, requirements); the deltas are the difference from the graph's text, written to `changes/<id>/deltas/`, replacing any there, each MODIFIED element based on `clm-apply`'s `hashes`. A renamed component gets a `remove: true` delta and a `create: true` delta with its rewritten text; its assets are copied to `changes/<id>/assets/<new-id>/` and listed under the old component's `REMOVED Assets`. Then `clm_apply::apply` writes `specs/` and marks the change `applied`, the human layers are written, the markers rewritten (their IDs, then their versions once the leftovers are found: **Markers**), the other live changes' deltas rebased (**Other changes' deltas**), the `status:` line set to `done` and the change archived with `clm_apply::archive`, and the `resolves` items deleted, in that order.

**What a reference is.** Structured references are whole values: `depends_on` entries, `serves` entries and `#### Serves`, `#### Needs` and `#### Enforced by` lines, and backlog `components`, `blocks` and `requirement` values. In spec text (requirements with their scenarios, Contracts, Designs and the human-layer entries), a reference is a code span whose whole text is an old ID, or a code span whose whole text is an old slug when no other requirement, entry or decision (`docs/decisions/<slug>.md`) has that slug, the bare slug CLM's prose cites a rule by. A component's bare ID in prose (the word `apply`) is a word, not a reference: renaming it is a `term` pair.

**Terms.** An occurrence of a term counts when neither neighbour continues an ID or a path: the character before it is absent or is neither alphanumeric nor one of `-`, `_`, `/`, `.`, `:`, and the character after it is absent, or is neither alphanumeric nor one of `-`, `_`, `/`, or is `.` or `:` followed by whitespace or the line's end. So `sprocket-scoped-context`, `crates/sprocket/` and `req:sprocket/x` are untouched, and `the sprocket.` is swapped. Headings (every line starting with `#`) and the ID lines of `#### Needs`, `#### Serves` and `#### Enforced by` blocks are never swapped: a heading holds a requirement's slug or an entry's, which only a `requirement` pair renames, and the ID lines are references.

**Markers.** The files are those `check.coverage`'s `scan` reads (`markers-from-files`), and the markers its `marker_spans` finds in each, with their byte ranges (`check.coverage/marker-spans`), so a marker `rename` rewrites is exactly one the coverage counts, never one found by a second reading of the pattern. Each is replaced in place by its byte range; nothing else in a code file changes.

**Which markers stay behind** (`marker-rewrite`, `rename-leaves-disagreeing-proof-behind`). Re-citing a reworded requirement at its new version claims its tests prove the new text. When `references` rewrote an old ID or slug in the requirement's text and the code still names that old one, the claim is false: the code still says what the old text said. So a marker of such a requirement, when it cites the requirement's current version, keeps that version (under the new ID, when the requirement is renamed too) if a leftover (**Leftovers**) names one of the old IDs or slugs `references` rewrote in that requirement's text and lies in a file the requirement's component owns (`check.coverage`'s `owner`) or a file holding one of the requirement's markers. The requirement is then `behind` and its component's build brings the code to the new name. The leftovers are found after the markers name their new IDs and before their versions are chosen, so a marker `rename` rewrites is never a leftover, and the versions change no line the search reads. A term's leftover never holds a marker back: a word in code may be a sense of the word the swap didn't mean (`component-words`), which the author judges from the `still names` lines, while an old ID names one rule.

**Other changes' deltas** (`other-deltas`, `rename-rebases-other-deltas`). A term swap or a reference rewrite changes the text, and so the hash, of elements another live change's delta may MODIFY or REMOVE; left alone, that delta is stale (`apply/stale-base`) and `clm check` fails the rename branch over a change it doesn't own. The branch can't rebase that delta by hand and still replay (`replay`), and `sync.regenerate`'s `other-delta-merge` would commit a merge the replay doesn't reproduce, so `rename` rebases it as part of its output. After `clm_apply::apply`, for each live change other than its own whose status is `proposed` or `approved`, in ID order, and each of its deltas for a component the graph holds both before and after the rename, it takes the elements `clm_apply::stale_bases` names against the component after the rename and not against it before, and a frontmatter base, which `stale_bases` leaves out, that equals the component's `frontmatter` hash before the rename and not after: those are exactly the ones the rename moved. A base is only ever replaced, never added, since each such element records one. An element the delta MODIFIES has its text in the delta rewritten by the edits `rename` makes to spec text (a Contract, Design or requirement block as `references` and `term-swap` edit it, headings and ID lines left as they leave them; a MODIFIED Frontmatter by the value edit `references` makes to frontmatter), and every such element, modified or removed (any element of a `remove: true` delta included), records the element's hash after the rename as its base, written where `sync.regenerate`'s merge writes one (`stale-delta-merge`: the delta frontmatter's base key, or a requirement heading's `<!-- base: -->` comment), located with `clm_model::outline`'s spans. No three-way merge is needed: the edits are a function of the text, so applying them to the delta's text keeps the delta's own edits and adds the rename's, with no conflict to report and no `git` process to run, so the wasm build is unaffected. An element whose base was already stale, an ADDED requirement, a `create: true` delta and the rest of the other change's files are left as they are; `refusals` keeps a renamed component or requirement out of other deltas, so no rebased element is one the rename removes. It prints one `rebased <element> of <component> in <delta path>` line per element, `<element>` named as `clm hashes` names it and `<delta path>` relative to the root, in change, component and `clm hashes` order. The replay reproduces these edits, since `rename` makes them from the base.

**Leftovers.** After writing, `rename` searches the tracked text files outside `specs/`, `changes/`, `ARCHITECTURE.md` and `CAPABILITIES.md`, and outside `docs/decisions/`, which record history, for each old ID as a whole token, each old term as `Terms` counts it and, for each requirement or entry rename, its old slug as a code span's whole text (`` `slim-nodes` ``, the form `CLAUDE.md` cites an entry by) when `references` would rewrite that span in spec text, that is when no other requirement, entry or decision has the slug, and prints each hit. A component rename keeps each requirement's slug, so it adds no slug. These are skills, guides, `CLAUDE.md` and code, whose edits are code changes or the author's prose; the report never fails the command.

**Replay** (`replay`). `replays` reads both revisions through `clm-git`: the change `head` adds (live or new archived, from `clm-fingerprint`'s `added_changes` over the two `git ls-tree -r` listings, the shape `gate` and `clm land` use), its `change.md` at `head`, and `base`'s tracked files, extracted with `extract_paths` into a temporary directory (`extract-as-checkout`). It places the `change.md` at `changes/<id>/change.md` with its `status:` line set to `proposed`, runs `rename` there, and compares the result with `head`'s tree by path and blob hash (`git hash-object` against `git ls-tree -r`), leaving out files under `backlog/` that `head` adds and `base` lacks, since a session files defect reports on any branch. It runs only when `head` adds exactly one change and that change's `change.md` at `head` sets `renames`, which it reads from the blob's frontmatter, since no graph is loaded before the tree is extracted; otherwise the answer is no without extracting anything.

**Wasm.** `clm-gate` is in the `web` Worker's wasm build, so `clm-rename` and what it pulls in SHALL build for `wasm32-unknown-unknown`, as `clm-gate`'s other dependencies do; the Worker never calls it. The checks' wasm build of `clm-web` proves it.

**Tests.** A test that runs the `clm` binary sits in the root's `crates/clm/tests/rename.rs`, as `gate`'s do (`binary-tests-apart`); the rest are the library's, under `crates/clm-rename/tests/`. They judge a tree after a rename by every violation `clm_model::load` reports, the archived change's included, with none filtered out.

## Requirements

### Requirement: refusals
`clm rename <change-id>` SHALL exit 1, writing nothing and naming the cause, when the change isn't live with status `proposed`; when it records no `renames`; when loading the graph reports a violation against the change's `change.md`, such as a `model/rename-records` entry, naming the model's message; when a renamed component or requirement doesn't exist, or its new ID already does; when a live change other than this one has a delta for a renamed component, or one that modifies or removes a renamed requirement; and when a term occurs nowhere in spec text as `term-swap` counts it. It SHALL take the change's status, `renames` and `resolves` from the change `clm_model::load` loads, and SHALL write the change's `change.md` only by its `status:` line.

#### Scenario: target exists
- GIVEN components `a` and `b`, and change `x`, `proposed`, with `renames` holding `component: a` and `to: b`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `b`, and the tree is unchanged

#### Scenario: another change targets it
- GIVEN requirement `a/old`, change `y`, `proposed`, whose delta for `a` modifies `old`, and change `x`, `proposed`, with `renames` holding `requirement: a/old` and `to: new`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `y` and `a/old`, and the tree is unchanged

#### Scenario: term absent
- GIVEN change `x`, `proposed`, with `renames` holding `term: grommet stub` and `to: spare part`, and no spec text holding `grommet stub`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `grommet stub`, and the tree is unchanged

#### Scenario: a move the model rejects
- GIVEN components `a` and `b`, and change `x`, `proposed`, with `renames` holding `component: a` and `to: b.a`
- WHEN `clm rename x` runs
- THEN it exits 1 naming `model/rename-records` and entry `1`, and the tree is unchanged

#### Scenario: the archived record loads
- GIVEN component `a` with requirement `x`, and change `r`, `proposed`, with `renames` holding `requirement: a/x` and `to: y`
- WHEN `clm rename r` runs
- THEN `clm_model::load` reports no violation of the tree, `changes/archive/r/change.md` among it, and that file still holds `renames`

#### Needs
- model/rename-records

### Requirement: component-rename
For a `component` rename `clm rename` SHALL give the change a `remove: true` delta for the component and each descendant and a `create: true` delta for each under its new ID, holding the same frontmatter, Contract, Design, requirements and assets with references rewritten (`references`), so that once applied `specs/` holds the component at its new path with every requirement's text unchanged but for those references.

#### Scenario: a component and its child
- GIVEN component `a` with requirement `x`, its child `a.c` with requirement `y`, and change `r`, `proposed`, with `renames` holding `component: a` and `to: b`
- WHEN `clm rename r` runs
- THEN `specs/b/SPEC.md` holds `b/x`, `specs/b/c/SPEC.md` holds `b.c/y`, `specs/a/` is gone, and `changes/archive/r/deltas/` holds `remove: true` deltas for `a` and `a.c` and `create: true` deltas for `b` and `b.c`

### Requirement: requirement-rename
For a `requirement` rename `clm rename` SHALL give the requirement's component a delta removing the old requirement and adding it under the new slug with its body unchanged but for references, or, for an `architecture/` or `capability/` entry, rewrite the entry's heading in `ARCHITECTURE.md` or `CAPABILITIES.md`, keeping the heading the model gives for that file's entries (`model/human-layers`).

#### Scenario: a node rule
- GIVEN component `a` with requirement `stale-base`, and change `r`, `proposed`, with `renames` holding `requirement: a/stale-base` and `to: base-moved`
- WHEN `clm rename r` runs
- THEN `specs/a/SPEC.md` holds `### Requirement: base-moved` with `stale-base`'s body and no `stale-base`

#### Scenario: an architecture entry
- GIVEN `ARCHITECTURE.md` with entry `sprocket-scoped-context` under `## Constraints`, component `a` with `serves: [architecture/sprocket-scoped-context]`, and change `r`, `proposed`, with `renames` holding `requirement: architecture/sprocket-scoped-context` and `to: cogs-scoped-context`
- WHEN `clm rename r` runs
- THEN `ARCHITECTURE.md` holds `### Constraint: cogs-scoped-context`, and `a`'s `serves` is `[architecture/cogs-scoped-context]`

#### Needs
- model/human-layers

### Requirement: references
`clm rename` SHALL rewrite every reference to a renamed ID: whole values in `depends_on`, `serves`, `#### Serves`, `#### Needs` and `#### Enforced by` and in backlog items' `components`, `blocks` and `requirement`; and, in spec text, each code span whose whole text is an old ID, or an old slug that no other requirement, entry or decision has. Outside the other changes' delta elements that `other-deltas` rebases, it SHALL rewrite nothing else, and it SHALL edit no file under `changes/archive/` or `docs/decisions/`.

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

### Requirement: term-swap
For a `term` rename `clm rename` SHALL replace, case-sensitively, each occurrence of the term in spec text (each component's requirements with their scenarios, Contract and Design, and the entries of `ARCHITECTURE.md` and `CAPABILITIES.md`) whose neighbours don't continue an ID or a path, as the Design's **Terms** defines them, with the new text, leaving headings and the ID lines of `#### Needs`, `#### Serves` and `#### Enforced by` blocks alone, and give each component whose text changes a delta modifying each changed element.

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

### Requirement: marker-rewrite
`clm rename` SHALL rewrite each marker, in the files and as `check.coverage/markers-from-files` reads them, that cites a renamed requirement or one whose text the change rewords: a marker citing the requirement's current version SHALL cite the new ID at the requirement's version after the change, and any other marker SHALL cite the new ID with its version unchanged, so a requirement current before the rename is current after it, and one behind stays behind; except that a marker citing the current version of a requirement whose text `references` rewrote SHALL keep that version, under the new ID, when a mention `finish` reports of an old ID or slug rewritten in that text lies in a file the requirement's component owns or a file holding one of its markers, so that requirement is `behind` after the rename. A mention of a term never holds a marker back. Nothing else in the file SHALL change.

#### Scenario: current and behind markers
- GIVEN requirement `a/x` at version `11111111`, a test citing `a/x@11111111` and another citing `a/x@00000000`, and a change renaming `a/x` to `a/y`, whose version becomes `22222222`
- WHEN `clm rename` runs
- THEN the tests cite `a/y@22222222` and `a/y@00000000`, and `clm pending a` lists nothing for `a/y`, which the first test keeps current

#### Scenario: reworded requirement
- GIVEN requirement `a/x` at version `11111111` reading `the sprocket`, a test citing it at `11111111`, and a change swapping `sprocket` for `cogs`
- WHEN `clm rename` runs
- THEN the test cites `a/x` at its new version, and `clm pending a` lists nothing for `a/x`

#### Scenario: code still names a rewritten entry
- GIVEN `ARCHITECTURE.md` with entry `sprocket-scoped-context`, no other requirement, entry or decision with that slug, component `a` with `paths: ["plugin/**"]` whose requirement `x` at version `11111111` reads ``(`sprocket-scoped-context`)``, a test citing `a/x@11111111`, `plugin/skills/s/SKILL.md` reading `` `sprocket-scoped-context` ``, and change `r`, `proposed`, with `renames` holding `requirement: architecture/sprocket-scoped-context` and `to: cogs-scoped-context`
- WHEN `clm rename r` runs
- THEN `a/x` reads ``(`cogs-scoped-context`)``, the test still cites `a/x@11111111`, `clm pending a` lists `a/x` as `behind`, and it prints `still names sprocket-scoped-context plugin/skills/s/SKILL.md:<line>`

#### Scenario: another component's code names it
- GIVEN the same, but `plugin/skills/s/SKILL.md` owned by component `b`, and no file holding a marker of `a/x` naming `sprocket-scoped-context`
- WHEN `clm rename r` runs
- THEN the test cites `a/x` at its new version, and `clm pending a` lists nothing for `a/x`

#### Scenario: a term left in the code
- GIVEN requirement `a/x` of component `a` with `paths: ["plugin/**"]`, at version `11111111` reading `the sprocket`, a test citing it at `11111111`, `plugin/skills/s/SKILL.md` reading `the sprocket`, and a change swapping `sprocket` for `cogs`
- WHEN `clm rename` runs
- THEN the test cites `a/x` at its new version, and it prints `still names sprocket plugin/skills/s/SKILL.md:<line>`

#### Needs
- check.coverage/marker-spans

### Requirement: finish
`clm rename` SHALL, after generating the deltas, apply the change, set its status to `done` by its `status:` line alone, archive it, delete each `backlog/<slug>.md` its `resolves` names, and print one `renamed <old> -> <new>` line per rename in record order. It SHALL then print one `still names <old> <file>:<line>` line for each mention of an old ID or term it left, and of the old slug of a renamed requirement or entry written as a code span's whole text when no other requirement, entry or decision has that slug, in the tracked text files outside `specs/`, `changes/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `docs/decisions/`, sorted by file and line, and exit 0.

#### Scenario: done and archived
- GIVEN change `r`, `proposed`, renaming `a/x` to `a/y` with `resolves: [q]`, `backlog/q.md`, and `plugin/skills/s/SKILL.md` naming `a/x`
- WHEN `clm rename r` runs
- THEN `changes/archive/r/change.md` has `status: done` and differs from the input only in that line, `changes/r/` and `backlog/q.md` are gone, it prints `renamed a/x -> a/y` and `still names a/x plugin/skills/s/SKILL.md:<line>`, and it exits 0

#### Scenario: a bare slug left
- GIVEN `ARCHITECTURE.md` with entry `sprocket-scoped-context`, no other requirement, entry or decision with that slug, `plugin/skills/s/SKILL.md` reading `` (`sprocket-scoped-context`) `` on one line and `the sprocket-scoped-context rule` on another, and change `r`, `proposed`, with `renames` holding `requirement: architecture/sprocket-scoped-context` and `to: cogs-scoped-context`
- WHEN `clm rename r` runs
- THEN it prints `still names sprocket-scoped-context plugin/skills/s/SKILL.md:<line>` for the first line and nothing for the second, and exits 0

### Requirement: replay
`replays(repo, base, head)` SHALL be true exactly when `head` adds one change, live or new archived, whose `change.md` at `head` sets `renames`, and `head`'s tracked tree equals, by path and content, the tree `clm rename` produces from `base`'s tracked files with that `change.md` placed live with status `proposed`, leaving out files under `backlog/` that `head` adds and `base` lacks. It SHALL read both revisions without the working tree, and be false without extracting a tree when `head` adds no such change or more than one change.

#### Scenario: an untouched rename branch
- GIVEN a branch cut from `base` on which `clm rename r` ran and its result was committed
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is true

#### Scenario: an extra edit
- GIVEN that branch with one more commit editing a line of `crates/x/src/lib.rs`
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is false

#### Scenario: a defect report filed
- GIVEN that branch with one more commit adding `backlog/d.md`
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is true

#### Serves
- architecture/humans-review-intent
- capability/land-safely

### Requirement: flag-rerecord
`clm rename` SHALL rewrite the `requirement` value of each backlog item that records one (a `workaround` flag's `<component>/<slug>@<version>`, `workaround-flags`) and names a renamed requirement or one whose text the change rewords, as `marker-rewrite` rewrites a marker: a version that was the requirement's current version SHALL become its version after the change, under the new ID, and any other version SHALL stay as it is under the new ID, so a flag that was current before the rename is current after it and `clm check` raises no `model/parse-backlog` violation for it. Nothing else in the item SHALL change. The replay (`replay`) reproduces the rewrite, since `rename` makes it from the base.

#### Scenario: a flagged requirement reworded
- GIVEN requirement `a/x` at version `11111111` reading `the sprocket`, `backlog/f.md` with `kind: workaround` and `requirement: a/x@11111111`, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `backlog/f.md`'s `requirement` is `a/x` at its new version, the rest of the file is unchanged, and `clm_model::load` reports no violation for `backlog/f.md`

#### Scenario: a flagged requirement renamed
- GIVEN requirement `a/x` at version `11111111`, `backlog/f.md` with `kind: workaround` and `requirement: a/x@11111111`, and change `r`, `proposed`, with `renames` holding `requirement: a/x` and `to: y`
- WHEN `clm rename r` runs
- THEN `backlog/f.md`'s `requirement` is `a/y` at its version after the change, and `clm_model::load` reports no violation for `backlog/f.md`

#### Serves
- architecture/plain-terms

### Requirement: component-key
Each delta `clm rename` generates SHALL name its component with the frontmatter key `component`, never `node` (`model/component-keys`), and `clm rename` SHALL rewrite a backlog item's `components` values wherever `references` rewrites an item's components.

#### Scenario: generated deltas
- GIVEN a change whose `renames` holds `component: a.b` with `to: a.c`, on a graph with node `a.b`
- WHEN `clm rename` runs on it
- THEN every delta it writes sets `component:` and none sets `node:`

#### Scenario: an item's components
- GIVEN the same change and `backlog/x.md` with `components: [a.b]`
- WHEN `clm rename` runs on it
- THEN `backlog/x.md` holds `components: [a.c]`

#### Needs
- model/component-keys

#### Serves
- architecture/plain-terms

### Requirement: other-deltas
`clm rename` SHALL, for each live change other than its own whose status is `proposed` or `approved`, rebase each element (frontmatter, Contract, Design or requirement) that one of the change's deltas MODIFIES or REMOVES and whose recorded base `apply/stale-base` accepts against the graph before the rename and refuses against the graph after it: an element the delta MODIFIES SHALL have its text in the delta rewritten by the edits `references` and `term-swap` make to spec text, and each such element SHALL record the element's hash after the rename as its base, so `apply/stale-base` reports nothing for that change. It SHALL change nothing else in the other change's files, and SHALL print one `rebased <element> of <component> in <delta path>` line per element it rebased, after the `renamed` lines and before the `still names` lines.

#### Scenario: a reworded Contract another change modifies
- GIVEN component `a` whose Contract reads `Print the sprocket.` and whose Design reads `Read the file.`, change `y`, `proposed`, whose delta for `a` modifies the Contract to `Print the sprocket.` and `Exit 0.` and the Design to `Read the file twice.`, both on their current bases, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `y`'s delta modifies the Contract to `Print the cogs.` and `Exit 0.` on the base `clm hashes a` gives for the Contract, its Design text and base are unchanged, it prints `rebased contract of a in changes/y/deltas/a.md`, it exits 0, and `clm_model::load` and `clm_apply::validate` report no violation for `y`

#### Scenario: a removed requirement
- GIVEN requirement `a/x` reading `the sprocket`, change `y`, `proposed`, whose delta for `a` removes `x` on its current base, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `y`'s delta still removes `x`, on the base `clm hashes a` gives for `x`, and nothing else in it changed

#### Scenario: a rename branch that rebased another delta
- GIVEN a branch cut from `base`, which holds change `y` as in the first scenario, on which `clm rename r` ran and its result was committed
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is true
