---
title: Rename
status: active
owners: ["@clunny"]
depends_on: [model, apply, check.coverage, gate.fingerprint, vcs]
paths: ["crates/clm-rename/**"]
serves: [architecture/plain-terms]
---

## Contract
`clm rename <change-id>` carries out the renames a live, `proposed` change records in its `change.md`'s `renames` (`model/rename-records`): it generates the change's deltas, applies them, rewrites what cites the renamed IDs and terms, test markers included, rebases the deltas of the other live changes whose status is `proposed` or `approved` and whose bases the rename moves (`other-deltas`), archives the change as `done` and deletes the items its `resolves` names, so the branch lands with nothing left pending but the requirements whose code still names an ID the rename rewrote in their text (`marker-rewrite`). It writes the working tree and commits nothing. It prints, on standard output, one `renamed <old> -> <new>` line per rename, then one `rebased <element> of <component> in <delta path>` line per element of another change's delta it rebased, then one `still names <old> <file>:<line>` line per mention it leaves for the author to judge, and exits 0. It exits 1 on a refusal, with the cause on standard error and nothing written, and 2 with git's message on standard error when git fails. `clm rename --help` prints the usage and exits 0.

The crate `clm-rename` exposes `rename(root, change_id)`, the command on a directory: it returns the lines the command prints, or a `Failure` (`vcs/failure`, which the crate offers again as `Failure`), `Refused` for a refusal and `Git` when git fails. `root` need not be a git checkout: where it holds no `.git`, every file under it outside directories named `.git` and `target` counts as tracked (`finish`), so a caller runs it on a tree it extracted. `replays(repo, base, head)` returns whether `head`'s tree is exactly what `rename` produces from `base`'s with the change `head` adds (`replay`), or a `Failure` when git fails. `gate` calls `replays` to classify a branch and to list a branch's files outside the spec paths (`gate/classify`, `gate/files-outside-spec-paths`), and `sync` calls both to regenerate a rename branch instead of rebasing it (`sync/regenerate-renames`, `regenerate-dont-merge`).

`clm-rename` offers `clm rename` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. Its arguments are `RenameArgs`, which `crates/clm`'s parser reads. `crates/clm` routes the command to it and adds nothing to what it writes. The call and `RenameArgs` are left out of a build for `wasm32-unknown-unknown`.

## Design
**Why one record** (`rename-by-replay`). A rename is mechanical, but its edits reach code paths (test markers) and the human layers, which a spec branch can't edit without being `mixed` or needing review. The pairs in `change.md` are the whole intent: everything else on the branch follows from them and from the base, so a program can reproduce it, and the gate trusts the branch exactly when it reproduces. `rename` writes ordinary deltas, which `clm apply` and the removal and move machinery (`node-removal`, `progress-follows-moves`) read as they read any.

**Reading the record.** `rename` reads the change from the graph `clm_model::load` loads: its status, its `renames` in order and its `resolves` are the loaded `Change`'s (`model/rename-records`), never a second parse of `change.md`. It writes `change.md` only by its `status:` line, so the change applies and is archived with its `renames` as written.

**Generation.** From those renames `rename` builds one map from each old ID to its new one: a component rename maps the component and each descendant (`a` → `b` gives `a.c` → `b.c`) and each of their requirements (`a/x` → `b/x`); a requirement rename maps the one ID. Terms are a list of `(old, new)` pairs, applied in order after the ID map. Each component's new text is computed in memory (frontmatter, Contract, Design, requirements); the deltas are the difference from the graph's text, written to `changes/<id>/deltas/` once what was there is cleared, each MODIFIED element based on `clm-apply`'s `hashes`. A renamed component gets a `remove: true` delta and a `create: true` delta with its rewritten text; its assets are copied to `changes/<id>/assets/<new-id>/` and listed under the old component's `REMOVED Assets`. The steps run in this order: the deltas and assets are written, then the human layers and the backlog items' references, so the graph `clm_apply::apply` checks holds the new names; `clm_apply::apply` writes `specs/` and marks the change `applied`; the markers are rewritten (their IDs, then their versions once the leftovers are found: **Markers**); the other changes' deltas are rebased (**Other changes' deltas**); the flags' versions are re-recorded (`flag-rerecord`); the `status:` line is set to `done` and the change archived with `clm_apply::archive`; and the `resolves` items are deleted.

**What a reference is.** Structured references are whole values: `depends_on` entries, `serves` entries and `#### Serves`, `#### Needs` and `#### Enforced by` lines, and backlog `components`, `blocks` and `requirement` values. In spec text (requirements with their scenarios, Contracts, Designs and the human-layer entries), a reference is a code span whose whole text is an old requirement or entry ID, one that holds a `/`, or a code span whose whole text is an old slug when no other requirement, entry or decision (`docs/decisions/<slug>.md`) has that slug, the bare slug CLM's prose cites a rule by. A component's ID, in prose or in a code span (the word `apply`), is a word, not a reference: renaming it is a `term` pair. The lines of fenced code hold no reference: a fence shows an example, whose IDs are the example's own.

**Terms.** An occurrence of a term counts when neither neighbour continues an ID or a path, as `term-swap` gives the characters. So `sprocket-scoped-context`, `crates/sprocket/` and `req:sprocket/x` are untouched, and `the sprocket.` is swapped. Headings (every line starting with `#`) and the lines of `#### Needs`, `#### Serves` and `#### Enforced by` blocks are never swapped: a heading holds a requirement's slug or an entry's, which only a `requirement` pair renames, and the ID lines are references. A term in fenced code is swapped as anywhere else, since a word is the same word in an example.

**Markers.** The files are those `check.coverage`'s `scan` reads (`check.coverage/markers-from-files`), and the markers its `marker_spans` finds in each, with their byte ranges (`check.coverage/marker-spans`), so a marker `rename` rewrites is exactly one the coverage counts, never one found by a second reading of the pattern. Each is replaced in place by its byte range.

**Which markers stay behind** (`marker-rewrite`, `rename-leaves-disagreeing-proof-behind`). Re-citing a reworded requirement at its new version claims its tests prove the new text. When `references` rewrote an old ID or slug in the requirement's text and the code still names that old one, the claim is false: the code still says what the old text said, so the marker keeps its version and the component's build brings the code to the new name. The leftovers (**Leftovers**) are found after the markers name their new IDs and before their versions are chosen, so a marker `rename` rewrites is never a leftover, and the versions change no line the search reads. A term's leftover never holds a marker back: a word in code may be a sense of the word the swap didn't mean (`component-words`), which the author judges from the `still names` lines, while an old ID names one rule.

**Other changes' deltas** (`other-deltas`, `rename-rebases-other-deltas`). A term swap or a reference rewrite changes the text, and so the hash, of elements another live change's delta may MODIFY or REMOVE; left alone, that delta is stale (`apply/stale-base`) and `clm check` fails the rename branch over a change it doesn't own. The branch can't rebase that delta by hand and still replay (`replay`), and `sync.regenerate`'s `other-delta-merge` would commit a merge the replay doesn't reproduce, so `rename` rebases it as part of its output. After `clm_apply::apply`, for each such change, in ID order, and each of its deltas for a component the graph holds both before and after the rename, it takes the elements `clm_apply::stale_bases` names against the component after the rename and not against it before, and a frontmatter base, which `stale_bases` leaves out, that equals the component's `frontmatter` hash before the rename and not after: those are exactly the ones the rename moved. A base is only ever replaced, never added, since each such element records one. An element the delta MODIFIES has its text in the delta rewritten by the edits `rename` makes to spec text (a Contract, Design or requirement block as `references` and `term-swap` edit it; a MODIFIED Frontmatter by the value edit `references` makes to frontmatter), and the new base is written where `sync.regenerate`'s merge writes one (`sync.regenerate/stale-delta-merge`: the delta frontmatter's base key, or a requirement heading's `<!-- base: -->` comment), located with `clm_model::outline`'s spans. No three-way merge is needed: the edits are a function of the text, so applying them to the delta's text keeps the delta's own edits and adds the rename's, with no conflict to report and no `git` process to run, so the wasm build is unaffected. The replay reproduces these edits, since `rename` makes them from the base, as it reproduces the flags' versions (`flag-rerecord`).

**Leftovers.** After writing, `rename` searches the tracked text files outside `specs/`, `changes/`, `ARCHITECTURE.md` and `CAPABILITIES.md`, and outside `docs/decisions/`, which record history, for each old ID as a whole token, each old term as `Terms` counts it and, for each requirement or entry rename, its old slug as a code span's whole text (`` `slim-nodes` ``, the form `CLAUDE.md` cites an entry by) when `references` would rewrite that span in spec text. A component rename keeps each requirement's slug, so it adds no slug. These are skills, guides, `CLAUDE.md` and code, whose edits are code changes or the author's prose.

**Replay** (`replay`). `replays` reads both revisions through `clm-git`: the change `head` adds (live or new archived, from `clm-fingerprint`'s `added_changes` over the two `git ls-tree -r --full-tree` listings, the shape `gate` and `clm land` use), its `change.md` at `head`, and `base`'s tree, extracted whole with `extract_tree` into a temporary directory (`vcs/extract-as-checkout`). It places the `change.md` at `changes/<id>/change.md` with its `status:` line set to `proposed`, runs `rename` there, and compares what is left with `head`'s listing, kind by kind: each regular file by path, mode and blob ID (`git hash-object --no-filters` of each file the rename left, and its owner's executable bit for the mode); each symbolic link by path and the blob ID of the path it names, read from the directory without following it, since a rename removes a link with a component's directory and writes an asset it read through a link as a regular file, so what `head` must hold is what the rename left, not what `base` held; and each submodule against `base`'s listing, since the extracted tree holds a submodule only as an empty directory and the rename writes none. The executable bit is read only where the platform has one: the crate also builds for `wasm32-unknown-unknown`, where nothing calls `replays`. A new file under `backlog/` is left out because a session files defect reports on any branch; only a regular file is one. It reads `renames` from the blob's frontmatter, since no graph is loaded before the tree is extracted.

**Wasm.** `clm-gate` is in the `web` Worker's wasm build, so `clm-rename` and what it pulls in build for `wasm32-unknown-unknown`, as `clm-gate`'s other dependencies do; the Worker never calls it. The checks' wasm build of `clm-web` shows it.

## Requirements

### Requirement: refusals
`clm rename <change-id>` SHALL exit 1, writing nothing and naming the cause on standard error, when the change isn't live with status `proposed`; when it records no `renames`; when loading reports a violation against the change's `change.md`, such as a `model/rename-records` entry, naming the model's rule and message; when a renamed component or requirement doesn't exist, or its new ID already does; when it renames both a component and a requirement of that component or of one of its descendants; when another live change whose status is `proposed` or `approved` has a delta for a renamed component or one of its descendants, or one that modifies or removes a renamed requirement; when a term occurs nowhere in spec text as `term-swap` counts it; and when the change's `deltas/` holds a directory, naming it.

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
`clm rename` SHALL rewrite every reference to a renamed ID: whole values in `depends_on`, `serves`, `#### Serves`, `#### Needs` and `#### Enforced by` and in backlog items' `components`, `blocks` and `requirement`; and, in spec text, each code span whose whole text is an old requirement or entry ID, or an old slug that no other requirement, entry or decision has. A component's ID in a code span is a word and stays, and no reference is rewritten on a line of fenced code, a line after one that starts, after any indentation, with ```` ``` ```` or `~~~` and before the next line that does. Beyond these references, the terms `term-swap` swaps, the markers `marker-rewrite` rewrites, the versions `flag-rerecord` re-records and the other changes' delta elements `other-deltas` rebases, it SHALL rewrite nothing else, and it SHALL edit no file under `changes/archive/` or `docs/decisions/`.

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

### Requirement: term-swap
For a `term` rename `clm rename` SHALL replace, case-sensitively, with the new text, each occurrence of the term in spec text (each component's requirements with their scenarios, Contract and Design, and the entries of `ARCHITECTURE.md` and `CAPABILITIES.md`) whose neighbours don't continue an ID or a path: the character before it is absent, or is neither a letter, a digit nor one of `-`, `_`, `/`, `.` and `:`; and the character after it is absent, or is `.` or `:` followed by whitespace or the line's end, or is neither a letter, a digit nor one of `-`, `_`, `/`, `.` and `:`. It SHALL leave alone every heading, a line that starts with `#`, and the lines of `#### Needs`, `#### Serves` and `#### Enforced by` blocks, SHALL swap a term on a line of fenced code as on any other line, and SHALL give each component whose text changes a delta modifying each changed element.

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

### Requirement: marker-rewrite
`clm rename` SHALL rewrite each marker, in every file the coverage reads markers from (`check.coverage/markers-from-files`), the markers outside a test included (`check.coverage/test-spans`), as `check.coverage/marker-spans` finds them, that cites a renamed requirement or one whose text the change rewords: a marker citing the requirement's current version SHALL cite the new ID at the requirement's version after the change, and any other marker SHALL cite the new ID with its version unchanged, so a requirement current before the rename is current after it, and one behind stays behind; except that a marker citing the current version of a requirement whose text `references` rewrote SHALL keep that version, under the new ID, when a mention `finish` reports of an old ID or slug rewritten in that text lies in a file the requirement's component owns or a file holding one of its markers, so that requirement is `behind` after the rename. A mention of a term never holds a marker back. Nothing else in the file SHALL change.

#### Scenario: current and behind markers
- GIVEN requirement `a/x` at version `11111111`, a test citing `a/x@11111111` and another citing `a/x@00000000`, so `clm pending a` lists `a/x` as `behind`, and a change renaming `a/x` to `a/y`, whose version becomes `22222222`
- WHEN `clm rename` runs
- THEN the tests cite `a/y@22222222` and `a/y@00000000`, and `clm pending a` lists `a/y` as `behind`, as it listed `a/x` before

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

#### Scenario: a marker outside a test
- GIVEN `src/lib.rs` holding a marker citing `a/x` in a function with no test attribute, and a rename of `a/x` to `a/y`
- WHEN `clm rename` runs
- THEN the marker cites `a/y`

#### Needs
- check.coverage/marker-spans
- check.coverage/test-spans

### Requirement: finish
`clm rename` SHALL, after generating the deltas, which replace every file directly in the change's `deltas/`, apply the change, set its status to `done` by its `status:` line alone, archive it, delete each `backlog/<slug>.md` its `resolves` names, and print one `renamed <old> -> <new>` line per rename in record order. It SHALL then print one `still names <old> <file>:<line>` line for each mention of an old ID or term it left, and of the old slug of a renamed requirement or entry written as a code span's whole text when no other requirement, entry or decision has that slug, in the tracked text files outside `specs/`, `changes/`, `ARCHITECTURE.md`, `CAPABILITIES.md` and `docs/decisions/`, sorted by file and line, and exit 0. Where the root holds no `.git`, every file under it outside directories named `.git` and `target` SHALL count as tracked. It SHALL commit nothing. When git fails it SHALL exit 2 with git's message on standard error.

#### Scenario: done and archived
- GIVEN change `r`, `proposed`, renaming `a/x` to `a/y` with `resolves: [q]`, `backlog/q.md`, and `plugin/skills/s/SKILL.md` naming `a/x`
- WHEN `clm rename r` runs
- THEN `changes/archive/r/change.md` has `status: done` and differs from the input only in that line, `changes/r/` and `backlog/q.md` are gone, it prints `renamed a/x -> a/y` and `still names a/x plugin/skills/s/SKILL.md:<line>`, and it exits 0

#### Scenario: a bare slug left
- GIVEN `ARCHITECTURE.md` with entry `sprocket-scoped-context`, no other requirement, entry or decision with that slug, `plugin/skills/s/SKILL.md` reading `` (`sprocket-scoped-context`) `` on one line and `the sprocket-scoped-context rule` on another, and change `r`, `proposed`, with `renames` holding `requirement: architecture/sprocket-scoped-context` and `to: cogs-scoped-context`
- WHEN `clm rename r` runs
- THEN it prints `still names sprocket-scoped-context plugin/skills/s/SKILL.md:<line>` for the first line and nothing for the second, and exits 0

#### Scenario: the archived record loads
- GIVEN component `a` with requirement `x`, and change `r`, `proposed`, with `renames` holding `requirement: a/x` and `to: y`
- WHEN `clm rename r` runs
- THEN the tree loads with no violation, none under `model/parse-changes` for `changes/archive/r/change.md`, and that file still holds `renames`

#### Scenario: deltas already there
- GIVEN component `a`, change `r`, `proposed`, with `renames` holding `component: a` and `to: b`, and a file `changes/r/deltas/old.md` that is no delta
- WHEN `clm rename r` runs
- THEN `changes/archive/r/deltas/` holds `b.md` and no `old.md`

#### Scenario: nothing committed
- GIVEN a git checkout at commit `c` holding change `r`, `proposed`, renaming `a/x` to `a/y`
- WHEN `clm rename r` runs
- THEN `HEAD` is still `c`, and `git status` shows the rename's files as changes not committed

#### Scenario: a tree that is no checkout
- GIVEN a directory with no `.git` holding change `r`, `proposed`, renaming `a/x` to `a/y`, and `notes.txt` naming `a/x`
- WHEN `clm rename r` runs
- THEN it prints `still names a/x notes.txt:<line>` and exits 0

### Requirement: replay
`replays(repo, base, head)` SHALL be true exactly when `head` adds one change, live or new archived, whose `change.md` at `head` sets `renames`, and `head`'s tracked tree is exactly the tree `clm rename` leaves when run on `base`'s tree with that `change.md` placed live with status `proposed`, compared entry by entry:
- the regular files of `head`'s tree, leaving out each one under `backlog/` at a path where `base`'s tree holds no entry, and the regular files the rename leaves are the same paths, each with the same mode, `100755` in `head`'s tree for a file the rename leaves executable by its owner and `100644` for any other, and the same git blob ID, the ID of a file the rename leaves being that of its bytes as they stand, with no filter applied;
- the symbolic links of `head`'s tree and the symbolic links the rename leaves, none of them followed, are the same paths, each naming the same path byte for byte;
- the submodules of `head`'s tree are the submodules of `base`'s, at the same paths with the same commit IDs, since `clm rename` neither writes nor removes one.

It SHALL be false when `clm rename` refuses or fails there, SHALL read both revisions without the working tree, SHALL be false without extracting a tree when `head` adds no such change or more than one change, and SHALL return an error when git fails to list or read a revision.

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

#### Scenario: a symbolic link added
- GIVEN that branch with one more commit adding a symbolic link `l` to `README.md`
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is false

#### Scenario: a mode changed
- GIVEN that branch with one more commit that makes `crates/x/src/lib.rs` executable and changes nothing else
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is false

#### Scenario: a base that holds a symbolic link
- GIVEN a `base` that holds a symbolic link `l` to `README.md`, and a branch cut from it on which `clm rename r` ran and its result was committed
- WHEN `replays(repo, base, HEAD)` runs
- THEN it is true; and with one more commit that points `l` at `ARCHITECTURE.md`, it is false

#### Scenario: a rename that refuses on the base
- GIVEN a `head` that adds one change whose `change.md` sets `renames` holding `requirement: a/none` with `to: y`, where `base` holds no requirement `a/none`
- WHEN `replays(repo, base, head)` runs
- THEN it is false, not an error

#### Serves
- architecture/humans-review-intent
- capability/land-safely

### Requirement: flag-rerecord
`clm rename` SHALL rewrite the `requirement` value of each backlog item that records one (a `workaround` flag's `<component>/<slug>@<version>`, `workaround-flags`) and names a renamed requirement or one whose text the change rewords, as `marker-rewrite` rewrites a marker: a version that was the requirement's current version SHALL become its version after the change, under the new ID, and any other version SHALL stay as it is under the new ID, so a flag that was current before the rename is current after it and `clm check` raises no `model/parse-backlog` violation for it. Nothing else in the item SHALL change.

#### Scenario: a flagged requirement reworded
- GIVEN requirement `a/x` at version `11111111` reading `the sprocket`, `backlog/f.md` with `kind: workaround` and `requirement: a/x@11111111`, and change `r`, `proposed`, with `renames` holding `term: sprocket` and `to: cogs`
- WHEN `clm rename r` runs
- THEN `backlog/f.md`'s `requirement` is `a/x` at its new version, the rest of the file is unchanged, and loading reports no `model/parse-backlog` violation for `backlog/f.md`

#### Scenario: a flagged requirement renamed
- GIVEN requirement `a/x` at version `11111111`, `backlog/f.md` with `kind: workaround` and `requirement: a/x@11111111`, and change `r`, `proposed`, with `renames` holding `requirement: a/x` and `to: y`
- WHEN `clm rename r` runs
- THEN `backlog/f.md`'s `requirement` is `a/y` at its version after the change, and loading reports no `model/parse-backlog` violation for `backlog/f.md`

#### Serves
- architecture/plain-terms

### Requirement: component-key
Each delta `clm rename` generates SHALL name its component with the frontmatter key `component`, never `node` (`model/component-keys`).

#### Scenario: generated deltas
- GIVEN a change whose `renames` holds `component: a.b` with `to: a.c`, on a graph with component `a.b`
- WHEN `clm rename` runs on it
- THEN every delta it writes sets `component:` and none sets `node:`

#### Needs
- model/component-keys

#### Serves
- architecture/plain-terms

### Requirement: other-deltas
`clm rename` SHALL, for each live change other than its own whose status is `proposed` or `approved`, rebase each element (frontmatter, Contract, Design or requirement) that one of the change's deltas MODIFIES or REMOVES and whose recorded base `apply/stale-base` accepts against the graph before the rename and refuses against the graph after it: an element the delta MODIFIES SHALL have its text in the delta rewritten by the edits `references` and `term-swap` make to spec text, and each such element SHALL record the element's hash after the rename as its base, so `apply/stale-base` reports nothing for an element it rebased. It SHALL change nothing else in the other change's files, and SHALL print one `rebased <element> of <component> in <delta path>` line per element it rebased, `<element>` named as `clm hashes` names it and `<delta path>` relative to the root, in order of change ID, then of component ID, then as `clm hashes` orders a component's elements, after the `renamed` lines and before the `still names` lines.

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

### Requirement: commands-in-crate
`clm-rename` SHALL offer `clm rename` as a call of its own that takes the command's arguments as the command line gives them, the repository root the command line resolves, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2, and for `--help` it SHALL write the usage to the output stream and return 0.

#### Scenario: a command called in a test
- GIVEN a repository holding a `proposed` change `r` that renames `a/x` to `a/y`, and `notes.txt` naming `a/x`
- WHEN `clm-rename`'s call runs with the arguments of `clm rename r`, that root and two buffers
- THEN the output buffer holds `renamed a/x -> a/y` and `still names a/x notes.txt:1`, one to a line, the error buffer is empty, and the call returns 0, the status `clm rename` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is passed an option `clm rename` doesn't have
- THEN it returns 2 with the usage error in the error buffer and nothing in the output buffer, and the caller's process goes on

#### Scenario: help
- GIVEN the same call
- WHEN it is passed `--help`
- THEN it returns 0 with the usage, which holds `Usage: clm rename`, in the output buffer and nothing in the error buffer

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: undo-on-failure
When a step of `clm rename` fails after it has begun writing and before the change is archived (a write or a removal, the apply, a refusal or git), it SHALL put every file it wrote or removed back as it was before the run, those its apply wrote and removed among them, and exit as that failure exits (`refusals`, `finish`), so the tree is as the run found it. It SHALL perform its own files and the apply's through one `apply/journal`, the apply's through `apply_into`, and undo that journal once. A file it can't put back SHALL be named on standard error on a line of its own holding `not restored`.

#### Needs
- apply/journal

#### Scenario: a write after the apply fails
- GIVEN change `r`, `proposed`, renaming `a/x` to `a/y`, a test file citing `a/x` that can't be written, and `backlog/q.md` that `r`'s `resolves` names
- WHEN `clm rename r` runs
- THEN it exits 1, `specs/a/SPEC.md` and `changes/r/change.md` hold what they held before, `changes/r/deltas/` holds what it held before, and `backlog/q.md` still exists
