---
title: Regenerate
status: active
owners: ["@clunny"]
depends_on: [model, apply, check, vcs]
paths: ["crates/clm-regenerate/**"]
successor: ""
serves: [capability/work-concurrently, architecture/hash-based-staleness]
---

## Contract
The regenerated `specs/` a branch's applied changes give on top of a base, for `clm rebase` (`sync/reapply`) and for `clm land`'s derived-specs check (`land/derived-specs`) and its regenerate of a head behind in `specs/` (`land/regenerates-behind-specs`), with the stale-delta merge and report that go with it. Crate `clm-regenerate` offers:

- `regenerated(root, onto, ids, merge) -> Result<Regenerated, Error>`: `Regenerated::Tree(Tree)`, or `Regenerated::Stopped { merged, left }`, the `merged` lines (the `took back` lines among them, in the order they were made) and the lines that stopped it (`regenerated-tree`, `stale-delta-diff`, `stale-delta-merge`). It returns `Error::Refused` for a change of `ids` that `HEAD` doesn't hold and, with `merge`, for an uncommitted edit to a delta file it could write; and `Error::Git` for an `onto` that names no commit and for a git or file failure.
- `Tree::path()`, the regenerated tree's CLM root (its `specs/`, `changes/` and `backlog/`), which `clm rebase` mirrors into `root`, and the public field `Tree::merged`, the `merged` lines, the `took back` lines among them in the order they were made.
- `Tree::other_deltas(&mut self, root, ids, onto) -> Result<(Vec<String>, Vec<String>), Error>`: the same merge for the other live changes' deltas, returning the `merged` lines, the `took back` lines among them in the order they were made, and what is left, empty when every other change applies, and `Error::Refused` for an uncommitted edit to a delta file it could write, as `regenerated` with `merge` gives (`stale-delta-merge`). The caller passes the `ids` and `onto` it gave `regenerated`, once it has mirrored and committed the tree's `specs/` when `ids` named a change.
- `git(root, args)` and `run(root, args, options)`: `clm-git`'s `git` and its `run` of `git`, retried on a held `index.lock`, each returning `clm-git`'s `Error` (`retry-index-lock`); `clm-sync` runs its `git` commands through them.
- `exists_at(root, rev, path)`, `extract_present(root, rev, paths)` and `io(path, e)`, helpers `clm-sync` calls rather than copies (`shared-helpers`).
- `Error`: `clm-git`'s `Failure`, `Refused` or `Git`.

It reaches no `clm-health`, so `clm-land` can depend on it (`clm/health-isolated`). It never writes `specs/` in `root`: mirroring the tree and the `Re-apply` commit are `clm rebase`'s.

## Design
It depends on `clm-model`, `clm-apply`, `clm-check` (for `format`, a violation as `clm check` prints it) and `clm-git`, and never on `clm-health`, `clm-ledger` or `clm-context` (`sync-regenerate-split`).

**The tree.** `regenerated` extracts `onto`'s `specs/` into one temporary directory and `HEAD`'s other paths into a second with `clm-git`'s `extract_paths`, each path the CLM root's, given from the repository's top as `<prefix><path>` (`git rev-parse --show-prefix`, empty at the top), and moves the entries of the second's `<prefix>` into the first's, so `clm_model::load` reads one tree, the first's `<prefix>`: without the human layers, every `serves` naming an entry would be broken on both sides of `apply`'s before-and-after comparison (`apply/resulting-graph`). Only the change's own files can keep it from applying, so the rest of the mixed tree is `clm check`'s to judge.

**Order** (`reapply-order-follows-bases`). `regenerated` sorts `ids` and drops repeats itself, and picks the next change from the graph it loads for that step, before **Merge**'s reload, so a change's bases are read from its delta as committed. A change whose `change.md` `HEAD` holds but the graph doesn't (one that doesn't parse) counts as having no unknown base, so ID order reaches it and its violations stop the run. A base `main` moved since is found in `onto`'s history, so a change stale only against `main` doesn't wait, and **Merge** takes it. The change picked last can still have an unknown base: a dependency the branch broke, or a base an earlier change produced before `main` moved the element.

**Stages** (`stale-delta-names-movers`, `stale-delta-shows-main`). The stages are the graphs the regenerate loads anyway. Taking the newest stage that holds a base finds a later change that restores an earlier hash at the later stage. The history walked is that of the component's `SPEC.md` as `clm-model` gives it at `onto`, without following renames, skipping a commit whose file doesn't parse. The diff has three lines of context. The merge line names the element's hash in the tree it was applied to rather than leaving it to `clm hashes`, which reads the working tree's `specs/`. The diff is the only place the author sees `main`'s new text when they rebase: updating the hashes alone would pass every check and drop that text when the branch lands.

**Merge** (`stale-delta-three-way-merge`). Each element is merged with `git merge-file` through `clm-git` on three texts: the element's text in the delta (a requirement's block as `clm-model` gives it, with its clean heading), the base's text and the element's text in the tree, each trimmed of leading and trailing blank lines. A conflict leaves the element as it is. The merged text replaces the element's text in the delta, located with `clm_model::outline`'s spans as `clm-apply` locates them, keeping the heading's slug and the surrounding blank lines. The base is replaced in place: `<key>: "<old base>"` in the frontmatter, so a flow map and a block map are both edited with the other keys untouched, and the `<!-- base: <hash> -->` comment in a requirement heading. A delta is rewritten line by line, so one with CRLF line endings is left alone. An archived change's deltas are under `changes/archive/<id>/deltas/`, which `gate/archive-immutable` lets the branch edit. The merge takes the text, not the judgment: the merged section is in the branch's diff, `clm check` judges the result (`judged-by-check`), and on a branch that needs review the changed fingerprint asks a human to approve again (`tree-diff-fingerprint`). The merge is worked in the temporary tree and written to `root` only for a file it then commits, and only after every delta file it could write is checked against `HEAD`.

**Taking a merge back** (`other-delta-merge`). A merge whose movers name a change, and a hand resolution of one, set a base that only this branch's tree has. The next `clm rebase` loses that tree: when `main` has moved that element since, the rebase keeps `main`'s side of `specs/` and drops the earlier `Re-apply` commit, and the regenerated tree no longer yields the base, so it has no text anywhere; and when the branch has since taken its edit back, nothing produces it again. Searching `HEAD`'s history for the base fails for the first reason, and starting again from `onto`'s copy of the delta would discard the branch's own edits to it. So an element whose base is unknown is taken back to the base it had before the newest commit on the branch that set it, found by the base the element records and never by a commit's message, so a commit the program made and one a person made are read alike. It works per element, as the merge writes per element: the element's block alone is merged three-way with `git merge-file` (now, in that commit, before it), and only its own base key or heading comment is set, located as **Merge** locates them, so a base edited by hand on another element of the same frontmatter line doesn't block it. The history walked is that of the delta's path, followed from `changes/archive/<id>/` back to `changes/<id>/`. In `regenerated` an element is judged when its change is reached in order, against the tree so far, as **Order** reads a base: so a base the report asked the author to set, the element's hash after an earlier change on the branch, is current there and the author's resolution stays. An element whose base is current or found in a stage or `onto`'s history is never taken back, so a rebase whose cause is unchanged writes nothing, and a merge whose movers are `main` alone, whose base is `main`'s, stays. What it leaves by hand: a hand resolution is taken back with the rest, so when its cause changes (the mover's edit changes or `main` moves the element) and the merge again conflicts, the author resolves it again; the `took back` line ahead of the report says which element that is.

**Other deltas.** An `applied` change is left alone because its bases record the text it was applied to. Only this branch's applied changes can have made such a delta stale, since one stale against `onto` itself fails `clm check` there, and the other change's author could do nothing about it until this branch lands.

**A transient `index.lock`** (`retry-index-lock`). Several sessions share one clone (`session-worktrees`, `build-places`), and Claude Code, hooks and the panel run `git` in it between a session's commands. The wrapper keys on the stderr text, not the exit code, which differs from one git command to another on the lock. `merge-file`, `diff --no-index` and `clm-git`'s `fetch`, `current_branch` and `extract_paths` take no lock or use a temporary index, so they don't go through it. It lives here rather than in `clm-git` because only `clm rebase`'s two crates need it (`shared-plumbing-in-git`).

## Requirements

### Requirement: regenerated-tree
`regenerated(root, onto, ids, merge)` SHALL build, in a temporary directory, `onto`'s `specs/` (none when `onto` has none) with `HEAD`'s `changes/`, `backlog/`, `docs/decisions/`, `ARCHITECTURE.md` and `CAPABILITIES.md` (every other path `clm check` reads), each taken under the CLM root's prefix in the repository when the root is a subdirectory of it, then re-apply the changes of `ids` one at a time as `apply/reapply` states, whatever their status, each onto the tree the ones before it left, the next being, of the changes left, the first in ID order none of whose recorded bases is unknown against the tree so far (`stale-delta-diff`), or the first in ID order when every change left has an unknown base, whatever order `ids` lists them in. It SHALL apply each change of `ids` once, however often `ids` names it, and with no `ids` it SHALL return `onto`'s `specs/` with `HEAD`'s other paths. It SHALL apply them stopping at the first change that has violations under its own directory (all the violations when the change doesn't load) or that doesn't apply. It SHALL return the tree, with the `merged` lines, or, as `Regenerated::Stopped`, the `merged` lines and the lines that stopped it: the violations as `check` prints them, followed by the stale-delta report (`stale-delta-diff`). With `merge` it SHALL first merge a stale delta as `stale-delta-merge` states; without it, it SHALL change nothing in `root`'s working tree, index or history. It SHALL return `Error::Refused` naming the change, having written nothing, when `ids` names a change of which `HEAD` holds no `change.md`, live or archived, and `Error::Git` naming `onto`, having written nothing, when `onto` names no commit. `regenerated` and `Tree::other_deltas` SHALL each return `Error::Git`, with git's message or with the path and the I/O error, when a git command or a file read or write fails, and `Error::Refused` only where this requirement and `stale-delta-merge` state it.

#### Scenario: regenerated without merging
- GIVEN a branch whose `HEAD` adds applied change `x`, whose delta adds requirement `a/r`, and an `onto` that has since added requirement `b/s`
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN the tree's `specs/a/SPEC.md` holds `a/r` and its `specs/b/SPEC.md` holds `b/s`, and `HEAD`, the index and the working tree are unchanged

#### Scenario: change built on another sorts first
- GIVEN a branch whose `HEAD` adds applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Design with base `h1`, `onto`'s hash, and `w`'s rewrites its second line with base `h2`, the Design's hash after `x` applies
- WHEN `regenerated(root, onto, ["w", "x"], false)` runs
- THEN it returns the tree, and the tree's Design holds both edits

#### Scenario: change that adds what another modifies
- GIVEN a branch whose `HEAD` adds applied changes `x`, whose delta adds requirement `a/r`, and `w`, sorting first, whose delta modifies `a/r` with base the hash of `a/r` as `x` adds it
- WHEN `regenerated(root, onto, ["w", "x"], false)` runs
- THEN it returns the tree, and the tree's `a/r` holds `w`'s text

#### Scenario: a change stale against main goes before the one built on it
- GIVEN a branch whose `HEAD` adds applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Contract with base `h0`, which `onto` has since moved by changing the same line, and `w`'s delta modifies `a`'s Contract with base the hash `x`'s delta gives on `h0`'s text
- WHEN `regenerated(root, onto, ["w", "x"], false)` runs
- THEN it returns `Stopped` whose lines are the `apply/stale-base` violation naming `x`'s delta followed by the report `main changed contract of a since h0`, with no violation naming `w`'s delta, and `HEAD`, the index and the working tree are unchanged

#### Scenario: independent changes keep ID order
- GIVEN a branch whose `HEAD` adds applied changes `x` and `y`, whose deltas both modify `a`'s Design with base `h1`, `onto`'s hash, each rewriting its first line differently
- WHEN `regenerated(root, onto, ["y", "x"], false)` runs
- THEN it returns `Stopped` with the `apply/stale-base` violation naming `y`'s delta and the report `x changed design of a since h1`

#### Scenario: human layers in the tree
- GIVEN a branch whose `HEAD` adds applied change `x` and `proposed` change `y`, whose delta adds to component `a` a requirement whose `#### Serves` names `architecture/e`, an entry of `HEAD`'s `ARCHITECTURE.md`, and on which `clm check` passes
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN the tree holds `HEAD`'s `ARCHITECTURE.md`, `CAPABILITIES.md` and `docs/decisions/`, the tree loaded as `clm check` loads it has the entry `architecture/e`, and validating `y` against it reports no `apply/resulting-graph` violation

#### Scenario: CLM root in a subdirectory
- GIVEN a repository whose CLM root is its subdirectory `sub`, a branch whose `HEAD` adds applied change `x`, whose delta adds requirement `a/r`, and an `onto` that has since added requirement `a/w` under `sub/specs/`
- WHEN `regenerated(sub, onto, ["x"], false)` runs
- THEN the tree's `specs/a/SPEC.md` holds both `a/r` and `a/w`

#### Scenario: every base unknown
- GIVEN a branch whose `HEAD` adds applied changes `w` and `x`, each of whose deltas modifies `a`'s Design with a base that no stage and no commit of `onto`'s history has
- WHEN `regenerated(root, onto, ["x", "w"], false)` runs
- THEN it returns `Stopped` on `w`'s `apply/stale-base` violation, and no line names `x`'s delta

#### Scenario: arguments regenerated is given
- GIVEN a branch whose `HEAD` adds applied change `x`, whose delta adds requirement `a/r`, and a `proposed` change `p`, whose delta adds requirement `a/q`
- WHEN `regenerated(root, onto, ids, false)` runs with each of these
- THEN:
  - `ids` `["x", "x"]` returns a tree whose `specs/` is byte for byte the one `["x"]` gives
  - `ids` `[]` returns a tree whose `specs/` is byte for byte `onto`'s
  - `ids` `["p"]` returns a tree whose `a` holds `a/q`
  - `onto` `no-such-ref` returns `Error::Git` naming `no-such-ref`
  - `ids` `["nope"]` returns `Error::Refused` naming `nope`
  - in each case `HEAD`, the index and the working tree are unchanged

#### Scenario: git failure
- GIVEN a directory outside any git repository
- WHEN `regenerated(dir, "HEAD", [], false)` runs
- THEN it returns `Error::Git` holding git's message

#### Serves
- architecture/reuse-first
- architecture/hash-based-staleness

### Requirement: stale-delta-diff
Only a base a delta records for a Contract, Design or requirement it MODIFIES or REMOVES counts here, never a frontmatter or asset base nor one `apply` accepts, such as that of a MODIFIED requirement without its own `#### Serves` block whose rule `onto` changed only in its `#### Serves` block. The stages of the tree are `onto`'s `specs/`, then the tree after each change applied before, in order. Such a base is found at the newest stage where the element hashes to it, an element absent from a stage never matching, or, when no stage has it, at the newest commit in `onto`'s history of the component's `SPEC.md` where the element hashes to it; it is unknown when neither has it.

When `regenerated` stops on a change with stale deltas, or `Tree::other_deltas` stops on one (`stale-delta-merge`), the lines it gives SHALL, after the stale-delta violations and for the change that stopped it, hold for each such element whose recorded base is stale against the tree the change was applied to as `apply/stale-base` judges it and `apply/stale-elements` lists it, and that the merge left stale, a line `<movers> changed <element> of <component> since <base>`, with `<element>` named as `clm hashes` names it and `<movers>` naming what moved the element since its base, joined with ` and `: `main` when no stage has the base, then each change applied after the stage where the base is found (after none when no stage has it) whose application changed the element's hash, in ID order. It SHALL NOT name `main` when a stage has the base, nor a change whose application left the element's hash as it was. The line SHALL be followed by the hunks of a unified diff from the element's text where the base is found to its text in that tree (empty when the element is gone), and then a line `merge the changes above into <delta path>, then set its base to <hash>`, naming the delta's repository-relative path and the element's hash in that tree, or, when the element is gone from the tree, `rework <delta path> by hand: <component> no longer has <element>`. When the base is unknown, it SHALL give the single line `no text for <element> of <component> at <base>; compare <delta path> with <element> of <component> after <movers> by hand` in place of the other three. It SHALL give nothing of this for a missing base or for a base that doesn't count.

#### Scenario: main changed a line the delta changed
- GIVEN a branch adding applied change `x`, whose delta rewrites the line `Do y.` of `a`'s Contract with base `h1`, and an `onto` that has since changed that line to `Do y. Also z.`
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns `Stopped` whose lines hold `main changed contract of a since h1`, then a hunk with the line `+Do y. Also z.`, then the merge line naming `x`'s delta path and the hash `clm hashes a` gives at `onto`

#### Scenario: base not in history
- GIVEN a branch adding applied change `x`, whose delta modifies `a`'s Design with a base that no commit of `onto`'s `specs/a/SPEC.md` hashes to
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN its lines hold `no text for design of a at <base>` with the instruction to compare by hand after `main`, and no hunk and no merge line

#### Scenario: element gone from the tree
- GIVEN a branch adding applied change `x`, whose delta modifies requirement `a/r` with base `h1`, and an `onto` that has since removed `a/r`
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN its lines hold `main changed requirement:r of a since h1`, then a hunk with every line of `a/r`'s old text as `-`, then `rework <delta path> by hand: a no longer has requirement:r` naming `x`'s delta path, and no merge line

#### Scenario: an earlier change on the branch moved it
- GIVEN a branch adding applied changes `x` and `y`, `x` before `y` in ID order, whose deltas both modify `a`'s Design with base `h1`, `onto`'s hash, each rewriting its first line differently, and an `onto` that hasn't changed `a` since
- WHEN `regenerated(root, onto, ["x", "y"], true)` runs
- THEN its lines hold the stale-base violation naming `y`'s delta, then `x changed design of a since h1` and no `main changed` line, then a hunk with `x`'s first line as `+`, then the merge line naming `y`'s delta path and the hash of `a`'s Design after `x` applies

#### Scenario: main and an earlier change moved it
- GIVEN the same branch, and an `onto` that has since changed the last line of `a`'s Design
- WHEN `regenerated(root, onto, ["x", "y"], true)` runs
- THEN its `merged` lines hold `merged design of a into changes/x/deltas/a.md`, and its lines hold `main and x changed design of a since h1`, with a hunk holding `main`'s last line and `x`'s first line as `+`

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness

#### Needs
- apply/stale-elements

### Requirement: stale-delta-merge
`regenerated` with `merge` SHALL, when a change of `ids` has stale deltas and before it reports them, merge three-way each Contract, Design or requirement a delta of that change MODIFIES whose recorded base is stale against the tree the change was applied to as `apply/stale-elements` lists it, when the element is still in that tree and its base isn't unknown (`stale-delta-diff`): the delta's text, the base's text and the tree's text. For each element that merges without conflict it SHALL replace the element's text in the delta file with the merged text, set the element's recorded base to its hash in that tree, change nothing else in the file, and give the line `merged <element> of <component> into <delta path>` as stated below; it SHALL commit in `root` the delta files it changed, and nothing else, as stated below, then apply the change again, keeping the commit when an element is left. `Tree::other_deltas(root, ids, onto)` SHALL merge the same way, against the regenerated tree, the deltas of each live change at `HEAD` whose status is `proposed` or `approved` and that `ids` doesn't name, whether the branch adds it or `onto` holds it, in ID order, committing at most one commit per change; when `apply`'s validation of such a change against the tree still refuses it after the merge, for a stale base or any other reason, it SHALL stop there and give the change's violations and the report `stale-delta-diff` gives for the elements left stale. Both SHALL leave unchanged, for `stale-delta-diff` to report, an element whose edits conflict, an element a delta REMOVES or that is gone from the tree (a requirement absent there, or a Contract or Design whose text there is empty), an element of a delta that removes its component, an element whose base is unknown after the take-back below, and every element of a delta file with CRLF line endings, and SHALL NOT merge a frontmatter or asset base, or an element whose base `apply` accepts. `regenerated` SHALL NOT edit the delta of a change `ids` doesn't name, and `Tree::other_deltas` SHALL NOT edit the delta of an `applied` or archived change.

When `regenerated` with `merge` reaches a change of `ids` in the order `regenerated-tree` states, and before `Tree::other_deltas` judges a change, each SHALL, for each Contract, Design or requirement a delta of that change MODIFIES whose recorded base `B` is unknown (`stale-delta-diff`) against the tree that change is judged on (for `regenerated` the tree so far, so a base an earlier change of `ids` produces is never unknown once that change is applied; for `Tree::other_deltas` the regenerated tree), take back the newest commit that `HEAD` holds and `onto` doesn't that changed that element's recorded base in that delta file to `B` from another base, whatever the commit's message, following the file from `changes/archive/<id>/` back to `changes/<id>/`: the element's text SHALL become the three-way merge of its text now, its text in that commit and its text before it, and its recorded base the one it had before that commit, and no other element's text or base SHALL change. While the base so restored is unknown and an older such commit set it, it SHALL take that commit back the same way. When a step conflicts, or no commit set the base, the element SHALL be left as it was before any take-back. A delta file with CRLF line endings SHALL be left alone. The merge above then works on the taken-back delta, so when the tree no longer moves an element that an earlier merge or hand resolution rebased onto this branch's text, the delta again holds the text and base it held before.

Each SHALL write a delta file in `root`, and commit it, only when its text then differs from `HEAD`'s, and SHALL give, for each element whose text or base then differs from `HEAD`'s copy and for no other, the line `merged <element> of <component> into <delta path>` when the merge above merged it, and otherwise `took back <element> of <component> in <delta path> to base <base>`, naming the base it now records. It SHALL commit a change's delta files in one commit, when it reaches that change: as `Merge <movers> into <change>` when a `merged` line names one of them, naming the merged elements' movers as `stale-delta-diff` names them, `main` first and then the changes in ID order; otherwise as `Take back what this branch merged into <change>`. A `regenerated` run that stops SHALL write and commit nothing for a change it didn't reach.

With `merge`, and in `Tree::other_deltas`, each SHALL write in `root`'s working tree only the delta files it commits, never `specs/`, and SHALL return `Error::Refused` naming the file, having written and committed nothing, when a delta file of a change it could merge or take back differs in `root`'s working tree or index from `HEAD`.

#### Scenario: edits apart
- GIVEN a branch adding applied change `x`, whose delta `changes/x/deltas/a.md` rewrites the first line of `a`'s Contract with base `h1`, and an `onto` that has since changed only the last line of `a`'s Contract
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns the tree, whose Contract of `a` holds both edits, with the `merged` line `merged contract of a into changes/x/deltas/a.md`; `HEAD` is a commit `Merge main into x` that changes only that delta, which now holds both edits and records as its contract base the hash `clm hashes a` gives at `onto`

#### Scenario: edits overlap
- GIVEN a branch adding applied change `x`, whose delta rewrites a line of `a`'s Contract with base `h1`, and an `onto` that has since changed the same line
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns `Stopped` with no `merged` line and lines holding `main changed contract of a since h1`, `HEAD` is unchanged, and the delta is unchanged

#### Scenario: one merged, one left
- GIVEN a branch adding applied change `x`, whose delta modifies requirements `a/p` and `a/q`, and an `onto` that has since changed a scenario line of `a/p` the delta didn't change and the same line of `a/q` the delta changed
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns `Stopped` with the `merged` line `merged requirement:p of a into changes/x/deltas/a.md` and lines holding `main changed requirement:q of a since <base>`, and `HEAD` is a commit `Merge main into x` in which `a/p`'s block and base are updated and `a/q`'s are unchanged

#### Scenario: base from an earlier change
- GIVEN a branch adding applied changes `x`, `y` and `z`, in that ID order, whose deltas all modify `a`'s Design: `x` with base `h1`, `onto`'s hash, rewriting its first line; `y` with base `h1`, rewriting its second line; and `z` with base `h2`, the Design's hash after `x` applies to `onto`'s, rewriting its third line; and an `onto` that hasn't changed `a` since
- WHEN `regenerated(root, onto, ["x", "y", "z"], true)` runs
- THEN it returns the tree, whose Design of `a` holds all three edits, with the `merged` lines `merged design of a into changes/y/deltas/a.md` and `merged design of a into changes/z/deltas/a.md`, and the branch has commits `Merge x into y` and `Merge y into z`

#### Scenario: element removed on one side
- GIVEN a branch adding applied change `x`, whose delta modifies requirement `a/r` and removes requirement `a/s`, and an `onto` that has since removed `a/r` and changed `a/s`
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns `Stopped` with no `merged` line, `HEAD` is unchanged, and the delta is unchanged

#### Scenario: serves edited on main
- GIVEN a branch adding applied change `x`, whose delta modifies `a/p`, its block without a `#### Serves` block, and `a/q`, each with its hash before `onto` moved it as its base, and an `onto` that has since edited only `a/p`'s `#### Serves` block and a scenario line of `a/q` the delta didn't change
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns the tree with the `merged` line `merged requirement:q of a into changes/x/deltas/a.md` and none naming `requirement:p`; the commit `Merge main into x` updates `a/q`'s block and base and leaves `a/p`'s as they were; and the tree's `a/p` holds the delta's text with `onto`'s `#### Serves` block

#### Scenario: another change's delta, edits apart
- GIVEN a branch adding applied change `x`, whose delta rewrites the first line of `a`'s Design, and a `proposed` change `o` at `HEAD` whose delta `changes/o/deltas/a.md` modifies `a`'s Design with base `h1`, `onto`'s hash, rewriting its last line, with the regenerated tree mirrored into `specs/` and committed
- WHEN `other_deltas(root, ["x"], onto)` runs on the tree `regenerated(root, onto, ["x"], true)` returned
- THEN it gives the `merged` line `merged design of a into changes/o/deltas/a.md` and nothing left, and `HEAD` is a commit `Merge x into o` that changes only that delta, which holds both edits and records as its design base the hash of `a`'s Design in the tree

#### Scenario: another change's delta, edits overlap
- GIVEN the same, where `o`'s delta rewrites the same first line
- WHEN `other_deltas(root, ["x"], onto)` runs
- THEN it gives no `merged` line, and what is left holds the stale-base violation naming `changes/o/deltas/a.md` and then `x changed design of a since h1` with a hunk holding `x`'s line as `+`; `HEAD` and `o`'s delta are unchanged

#### Scenario: applied change left alone
- GIVEN a branch adding applied change `x`, whose delta rewrites a line of `a`'s Design, and an `applied` change `p` at `HEAD` whose delta modifies `a`'s Design
- WHEN `other_deltas(root, ["x"], onto)` runs
- THEN it gives no `merged` line naming `changes/p/deltas/a.md`, nothing left, and that delta is unchanged

#### Scenario: CRLF delta left alone
- GIVEN the branch of "edits apart", with `changes/x/deltas/a.md` committed with CRLF line endings
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns `Stopped` with no `merged` line and lines holding `main changed contract of a since h1`, and `HEAD` and the delta are unchanged

#### Scenario: root's working tree
- GIVEN the branch of "edits apart"
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN:
  - on that branch as it is, `git status --porcelain` in `root` prints nothing afterwards
  - with an uncommitted edit to `changes/x/deltas/a.md`, it returns `Error::Refused` naming that file, and the file, the index and `HEAD` are as they were
  - with the same edit staged and not committed, it returns `Error::Refused` naming that file, and the file, the index and `HEAD` are as they were

#### Scenario: an edit taken back
- GIVEN a branch adding applied change `x`, and a `proposed` change `o` that `onto` holds, whose delta `changes/o/deltas/a.md` modifies `a`'s Contract and Design with bases `c1` and `h1`, `onto`'s hashes, on one frontmatter line, rewriting the Design's last line, where an earlier `other_deltas` on the branch committed `Merge x into o`, which merged `x`'s rewrite of the Design's first line into that delta and set its design base to `hx`, the hash of the Design with `x`'s edit, in a tree whose `Re-apply` commit `HEAD` no longer holds
- WHEN `other_deltas(root, ["x"], onto)` runs on the tree `regenerated(root, onto, ["x"], true)` returned, mirrored and committed
- THEN:
  - when `x`'s delta no longer edits `a`'s Design, it gives `took back design of a in changes/o/deltas/a.md to base h1` and nothing left, and `HEAD` is a commit `Take back what this branch merged into o` that changes only that delta, which is byte for byte `onto`'s copy
  - when, besides, a later commit of the branch rewrote another line of `o`'s Design in that delta, the delta is `onto`'s copy with that line's rewrite, and its design base is `h1`
  - when `x`'s delta also rewrites the first line of `a`'s Contract, which `o`'s delta rewrites too, so that `Merge x into o` merged only the Design, and a later commit of the branch merged the two Contract edits by hand and set the contract base, on the same frontmatter line, to the hash of `a`'s Contract after `x`; and `x`'s delta has since stopped editing the Design but still makes the same Contract edit: the delta's Contract and contract base are as that later commit left them, and its Design and design base are `onto`'s
  - when the merge was not made by `other_deltas` but by hand, in a commit `fix o` that rewrote the Design's first line and set the design base to `hx`, and `x`'s delta no longer edits `a`'s Design, the outcome is the first case's
  - when `x`'s delta still rewrites the Design's first line and `onto` has since changed a middle line of `a`'s Design, it gives `merged design of a into changes/o/deltas/a.md` and nothing left, and `HEAD` is a commit `Merge main and x into o`, whose delta holds `o`'s, `x`'s and `onto`'s edits and records as its design base the hash of `a`'s Design in the tree
  - when neither `x` nor `onto` has changed since that merge, it gives no `merged` or `took back` line and nothing left, and `HEAD` is unchanged
  - for an applied change `z` of `ids` whose delta a commit `Merge y into z` rebased onto the Design as `y` left it, where `y`'s delta no longer edits the Design, `regenerated(root, onto, ["y", "z"], true)` returns the tree with the line `took back design of a in changes/z/deltas/a.md to base h1`, and `HEAD` is a commit `Take back what this branch merged into z` whose delta is `z`'s as it was before `Merge y into z`; the same holds when `z` has since been archived, its delta then under `changes/archive/z/deltas/`
  - for applied changes `x` and `z` of `ids` whose deltas rewrite the same line of `a`'s Design with base `h1`, where a commit `fix z` merged the two edits into `z`'s delta by hand and set its design base to the Design's hash after `x`, as the report asked, and `x` unchanged since, `regenerated(root, onto, ["x", "z"], true)` run twice returns the tree each time with no `merged` or `took back` line, and `HEAD` and `z`'s delta are unchanged

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness

#### Needs
- apply/stale-elements

### Requirement: retry-index-lock
When a command that `clm-regenerate`'s `git` or `run` runs fails with stderr naming `index.lock`, it SHALL run the same command again after a wait, up to four more times with waits of 0.25, 0.5, 1 and 2 seconds, and SHALL return the last run's `clm-git` error unchanged when every run fails. It SHALL NOT retry a command that fails for another reason. Every git command `regenerated` and `Tree::other_deltas` run that writes `root`'s index or history, their commits included, SHALL be retried the same way.

#### Scenario: lock lifted
- GIVEN a work clone with a staged change, and its `index.lock` held by another process that removes it after half a second
- WHEN `git(root, ["commit", "-m", "c"])` runs
- THEN it returns the commit's output, and `HEAD` is a commit `c`

#### Scenario: lock kept
- GIVEN the same clone, with the lock held throughout
- WHEN `git(root, ["commit", "-m", "c"])` runs
- THEN it returns an error whose message names `index.lock`, after at least three seconds and within ten

#### Scenario: other failure not retried
- GIVEN a work clone
- WHEN `run(root, ["rev-parse", "--verify", "no-such-ref"], Options::default())` runs
- THEN it returns an error within a second

#### Scenario: merge commit waits out a lock
- GIVEN the branch of `stale-delta-merge`'s "edits apart", and first on `PATH` a `git` that, the first time it is given a `commit`, writes `fatal: Unable to create '<root>/.git/index.lock': File exists.` to stderr and exits 128 without running it, and passes every other call, and every later `commit`, to git unchanged, so the lock is met at the `Merge` commit itself and not at the `git add` before it
- WHEN `regenerated(root, onto, ["x"], true)` runs
- THEN it returns the tree, `HEAD` is a commit `Merge main into x` that changes only `changes/x/deltas/a.md`, and git was given `commit` twice

### Requirement: shared-helpers
`exists_at(root, rev, path)` SHALL return true when `rev`'s tree holds `path` and false when git reports that it doesn't, a `rev` that names no commit included, and SHALL return an error only when git can't be run. `extract_present(root, rev, paths)` SHALL return a temporary directory holding, as a checkout writes them, the files under those of `paths` that `rev` holds, and nothing else, empty when it holds none, leaving `root`'s working tree and index unchanged. `io(path, e)` SHALL return `Error::Git` whose message is `<path>: <e>`.

#### Scenario: helpers
- GIVEN a work clone whose `HEAD` holds `specs/a/SPEC.md` and no `backlog/`
- WHEN the helpers run
- THEN:
  - `exists_at(root, "HEAD", "specs/a/SPEC.md")` is true, `exists_at(root, "HEAD", "backlog")` is false, and `exists_at(root, "no-such-ref", "specs")` is false
  - `extract_present(root, "HEAD", ["specs", "backlog"])` returns a directory holding `specs/a/SPEC.md` as `HEAD` holds it and no `backlog`, and `git status --porcelain` is unchanged
  - `io("p", e)` for an I/O error `e` whose text is `not found` returns `Error::Git` whose message is `p: not found`

#### Serves
- architecture/reuse-first
