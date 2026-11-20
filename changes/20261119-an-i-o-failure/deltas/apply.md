---
component: apply
base: { contract: "1141b9acdf28fe85", design: "2470d6f123bac0c7" }
---

## MODIFIED Contract
- `validate(graph, change)` returns every reason the change can't be applied to the spec graph, as violations: a status `clm apply` refuses (`change-status`), each delta and asset that can't be applied, and every rule across components that the graph the change leaves would break, so a change that applies leaves a graph that loads.
- `resulting(graph, change)` returns the spec graph `validate` checks the change against, the graph the change would leave, or none when a delta of the change can't be applied, for `check`'s rules that judge a pending change's text (`resulting-graph-call`).
- `stale_bases(component, delta)` returns each Contract, Design or requirement `delta` MODIFIES or REMOVES whose recorded base `validate` judges stale against `component` (`stale-base`), with that base, for `sync.regenerate`'s stale-delta report and merges and for `rename`'s rebase of other changes' deltas (`stale-elements`).
- `hashes(component)` returns a component's `(element, hash)` pairs in `clm hashes`' order: its frontmatter, Contract and Design, then its requirements, then its assets (`hash-command`). The entries of a human layer, which `clm hashes architecture` and `clm hashes capability` print, are the command's and not this call's.
- `apply(root, id)` is `clm apply` on a checkout: it returns the files it wrote and removed, each a `FileOp`, or the violations; when the change or the graph is at fault it has written nothing, and when a write or removal fails it has restored every file it had written or removed (`atomic`, `failed-write`). `apply_into(root, id, journal)` is the same apply performing its files through the caller's `Journal` and restoring none on a failure, so a caller that writes files of its own around the apply, as `rename` does, undoes the apply's with its own (`journal`). `reapply(graph, change)` returns the `specs/` files an already applied change writes and removes, writing none itself (`reapply`). A `FileOp` gives its path from the repository root with `path()`, and `perform(root)` writes or removes that file under `root`.
- A `Journal` on a root (`Journal::new(root)`) performs `FileOp`s under it, remembering what each file held before the first one that touched it; `undo()` puts every file it touched back as it was, and returns each file it couldn't put back with why (`journal`).
- `archive(root, id)` is `clm archive` on a checkout: it returns the directory the change moved to, or the violations.
- `clm apply <change-id>` applies a change's deltas to `specs/` and copies or removes its assets, deletes the `question` items its `resolves` names (`answered-questions`) and the item its `from` names (`promoted-item`), all-or-nothing, and marks the change `applied`. It prints `applied <id>` and a line per file it wrote or removed, and exits 0 (`change-status`). It exits 1, printing each violation on standard output and writing nothing, on any violation of the change, or of the loaded graph that the change doesn't fix (`loaded-graph`); and 1, printing the failure and leaving every file as it was, when a write or removal fails (`failed-write`).
- `clm archive <change-id>` moves a `done` or `abandoned` change's directory to `changes/archive/<id>/` and prints where it went; it exits 1, moving nothing, for any other change (`archive-command`).
- `clm hashes <component>` prints one `<element> <hash>` line per element of the component: `frontmatter`, `contract`, `design`, then `requirement:<slug>` for each requirement, then `asset:<path>` for each asset. `clm hashes architecture` and `clm hashes capability` print one line per entry of that human layer, in file order, `requirement:<slug> <hash>` or `capability:<slug> <hash>`, the word its heading uses (`model/human-layers`). When the repository has the capability layer's component (`model/reserved-ids`), `clm hashes capability` prints that component's element lines first, then the entries' lines: the component holds no requirement, so no line name is printed twice. For an ID that names neither, it exits 2 with a line on standard error.

`clm-apply` offers each of `clm apply`, `clm archive` and `clm hashes` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
Apply computes each new SPEC.md text in memory, starting from the current text. Sections and requirement blocks are replaced by their heading spans (outside code fences). A removed component's `SPEC.md` is removed after its assets, so a write that fails part way never leaves assets without a component.

**Every reason, per delta.** A delta whose `## MODIFIED Frontmatter` is at fault still has its Contract, Design, requirements and assets judged: the frontmatter's violations are added to the rest, not a reason to stop, and each line at fault is its own violation (`atomic`, `frontmatter`). So one `clm apply` names everything a session must fix in the delta, never one fault per run.

**Undoing a failed write** (`failed-write`, `journal`). The files a valid change writes are many, and the filesystem offers no transaction across them, so `apply` keeps a `Journal`: before the first `FileOp` on a path it reads what the path holds, its bytes or nothing, and once an op fails it puts each touched path back in the reverse order: bytes are written back, creating the directories a removal dropped, and a file that wasn't there is removed, with each directory below `specs/`, `changes/` or `backlog/` its removal empties, as `FileOp::perform` drops one. A path whose read before the op fails stops the apply there, before that op, as a failed write. Putting back can itself fail: each file left changed is then named, so the session knows exactly what to restore by hand (`git restore` on a checkout), never a silent half-applied tree. `rename` wraps the apply in its own writes, so it passes its own journal through `apply_into` and undoes everything at once; the journal is the one record of what a run changed, not a second copy in each caller (`reuse-first`).

One function gives the backlog items `clm apply` deletes with a change (`answered-questions`, `promoted-item`). The apply and `resulting` both take them from it (`resulting-graph-call`), so the graph a change is checked against can't hold an item its apply deletes, and the apply builds the graph it checks through `resulting`.

`validate` and `stale_bases` judge a recorded Contract, Design or requirement base through one function, so the two can't come to differ (`stale-elements`).

**Archiving beside an entry.** `archive` asks whether `changes/<id>/` holds a change on disk, not only through the loaded graph: a change both live and in the archive under one ID loads once, as the archived one, and the graph alone would call the live directory missing (`archive-command`).

## MODIFIED Requirements

### Requirement: atomic <!-- base: 4121392e71e2cfb5 -->
`clm apply` SHALL validate every delta of the change before writing anything. If any violation exists, of the change or of the loaded graph as `loaded-graph` counts them, it SHALL print all of them on standard output, one line each as `model/violation-line` gives it, exit 1 and leave every file unchanged, the backlog items `answered-questions` and `promoted-item` would delete among them. A delta's violations SHALL all be printed together: one whose `## MODIFIED Frontmatter` is at fault (`frontmatter`) SHALL still have its Contract, Design, requirements and assets judged, and their violations printed with the frontmatter's.

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

#### Scenario: bad frontmatter and a missing section
- GIVEN component `a` whose `SPEC.md` has no `## Design` section, and a delta for `a` with the correct bases whose `## MODIFIED Frontmatter` holds the lines `  - model` and `: x`, and that has a `## MODIFIED Design`
- WHEN `clm apply <change>` runs
- THEN it exits 1, printing one `apply/frontmatter` line holding `  - model`, one holding `: x`, and one `apply/sections` line naming `a` and `## Design`

### Requirement: frontmatter <!-- base: d56a9ac34c08f64a -->
A MODIFIED Frontmatter SHALL replace the line of each listed top-level key, append keys not present, and leave every other line (including comments) unchanged. It SHALL be invalid, under `apply/frontmatter`, if a listed key's current value spans more than one line, which it does when the line after the key's starts with a space, a tab or `-`, or if a line of the delta's block that is neither blank nor a `#` comment isn't a top-level `key: value` on one line: one with no `:`, one with nothing before its `:`, or one that starts with a space, a tab or `-`. Each such key and each such line SHALL be reported as a violation of its own.

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

### Requirement: archive-command <!-- base: b3d8482abfba9863 -->
`clm archive <change-id>` SHALL move `changes/<id>/` to `changes/archive/<id>/` when the change's status is `done` or `abandoned`, print `archived <id> -> changes/archive/<id>` and exit 0. It SHALL refuse, under `apply/archive-command`, exiting 1 with nothing moved, a change of any other status, an archive entry that already exists, and an ID that names no change outside the archive, as ``no unarchived change `<id>` ``. When `changes/<id>/` holds a change and `changes/archive/<id>/` exists, whatever that entry holds, a change of the same ID among it, the refusal SHALL name `changes/archive/<id>` as already existing, never as ``no unarchived change `<id>` ``.

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

#### Scenario: the archive holds the same change
- GIVEN `changes/x/` with `status: done`, and `changes/archive/x/change.md` with `status: done`
- WHEN `clm archive x` runs
- THEN it exits 1 printing a line that holds `apply/archive-command` and `changes/archive/x`, and none holding `no unarchived change`, and `changes/x/change.md` is still there

#### Scenario: no such change
- GIVEN no `changes/nope/`
- WHEN `clm archive nope` runs
- THEN it exits 1 printing a line that holds `apply/archive-command` and ``no unarchived change `nope` ``

## ADDED Requirements

### Requirement: failed-write
When a change is valid and a write or removal of one of its files fails, `clm apply` SHALL put every file it had written or removed back as it was before the run, its bytes, or its absence for a file it created, print on standard output an `apply/atomic` line naming the file that failed and holding `write failed`, and exit 1, so the run leaves every file unchanged as a refused change does (`atomic`). A file it can't put back SHALL be named on a line of its own under `apply/atomic`, holding `not restored`, so nothing is left changed unnamed. `apply(root, id)` SHALL return those lines as its violations.

#### Scenario: a later write fails
- GIVEN a proposed change `c` with valid deltas for `a` and `b`, `a`'s `SPEC.md` written before `b`'s, and `b`'s `SPEC.md` that can't be written
- WHEN `clm apply c` runs
- THEN it exits 1, prints an `apply/atomic` line naming `b`'s `SPEC.md` and holding `write failed`, and `a`'s `SPEC.md`, `b`'s `SPEC.md` and `changes/c/change.md` hold what they held before

#### Scenario: a removed file comes back
- GIVEN a proposed change `c` that removes component `a.b` and asset `old.png` of `a`, and a file after them that can't be written
- WHEN `clm apply c` runs
- THEN it exits 1, and `specs/a/b/SPEC.md` and `specs/a/assets/old.png` exist with their bytes before the run

### Requirement: journal
A `Journal` on a root SHALL, before the first `FileOp` it performs on a path, remember that path's bytes, or that it held nothing, and SHALL perform no op on a path whose bytes it can't read, returning that failure. Its `undo()` SHALL put each path it touched back in the reverse order of first touch: writing back the bytes, with the directories they need, or removing a file it created together with each directory it created that the removal leaves empty. It SHALL return each path it couldn't put back with the error, and none when all are back. `apply_into(root, id, journal)` SHALL perform every file `apply` would through `journal`, and leave the undo to its caller.

#### Scenario: undo after writes and a removal
- GIVEN a root holding `x/f` with bytes `1` and `y/g`, and a journal that writes `x/f` as `2`, writes the new file `z/new/h`, and removes `y/g`
- WHEN `undo()` runs
- THEN `x/f` holds `1`, `y/g` exists with its bytes, `z/new/` is gone, and it returns no path

#### Scenario: one remembered state per path
- GIVEN a journal that writes `x/f`, first held `1`, as `2` and then as `3`
- WHEN `undo()` runs
- THEN `x/f` holds `1`
