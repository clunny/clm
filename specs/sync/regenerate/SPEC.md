---
title: Regenerate
status: active
owners: ["@clunny"]
depends_on: [model, apply, check, vcs]
paths: ["crates/clm-regenerate/**"]
successor: ""
serves: [capability/work-concurrently, architecture/regenerate-dont-merge]
---

## Contract
The regenerated `specs/` a branch's applied changes give on top of a base, built in one place for `clm rebase` (`sync`) and `clm land`'s derived-specs check (`land/derived-specs`), with the stale-delta merge and report that go with it. Crate `clm-regenerate` offers:

- `regenerated(root, onto, ids, merge) -> Result<Regenerated, Error>`: `Regenerated::Tree`, or `Regenerated::Stopped` with what stopped it (`regenerated-tree`, `regenerate-dont-merge`, `reapply-order-follows-bases`).
- `Tree::other_deltas(&mut self, root, ids, onto)`: the same merge for the other live changes' deltas, once the caller has mirrored and committed the tree (`stale-delta-merge`).
- `git` and `run`: `clm-git`'s, retried on a held `index.lock` (`retry-index-lock`); `clm-sync` runs its `git` commands through them.
- `exists_at`, `extract_present` and `io`, helpers `clm-sync` calls rather than copies, and `Error`, `clm-git`'s `Failure`.

It reaches no `clm-health`, so `clm-land` can depend on it (`clm/health-isolated`). It never writes `specs/` in `root`: mirroring the tree and the `Re-apply` commit are `clm rebase`'s.

## Design
It depends on `clm-model`, `clm-apply`, `clm-check` (for `format`, a violation as `clm check` prints it) and `clm-git`, and never on `clm-health`, `clm-ledger` or `clm-context` (`sync-regenerate-split`). `Tree::path` and `Tree::merged` are public for `clm-sync`'s mirror. `clm-sync` re-exports nothing of it: `clm-land` depends on `clm-regenerate` itself (`no-dead-code`).

**Tests.** The crate's tests call `regenerated`, `Tree::other_deltas`, `git` and `run` on a work clone of a bare `origin`, with `onto` the fetched `origin/main`.

**The tree** (`sync/extract-through-layer`). `regenerated` extracts `onto`'s `specs/` into one temporary directory and `HEAD`'s other paths into a second with `clm-git`'s `extract_paths`, and moves the second's top-level entries into the first, so `clm_model::load` reads one tree: without the human layers, every `serves` naming an entry would be broken on both sides of `apply`'s before-and-after comparison (`apply/resulting-graph`). For each change it loads the tree, keeps the graph as a stage, filters the load's violations to those under the change's own directory (only the change's own files can keep it from applying; `clm check` judges the rest), and applies the change with `clm-apply`'s `reapply`.

**Order** (`reapply-order-follows-bases`). `regenerated` sorts `ids` and drops repeats itself, and picks the next change from the graph it loads for that step, before **Merge**'s reload, so a change's bases are read from its delta as committed. A change the graph doesn't hold (a `change.md` that doesn't parse) counts as having no unknown base, so ID order reaches it and its violations stop the run. The lookup of a base reuses **Stages**' code and caches the history walk per component, so a component's history is walked at most once per run. A base `main` moved since is found by the walk, so a change stale only against `main` doesn't wait, and **Merge** takes it. When the change picked last still has an unknown base, a dependency the branch broke, or a base an earlier change produced before `main` moved the element, it is merged and reported, and stays the caller's to resolve.

**Stages** (`stale-delta-names-movers`, `stale-delta-shows-main`). The stages are the graphs the regenerate loads anyway. The newest stage holding a base is taken, so a later change that restores an earlier hash is found at the later stage. The history walk is `git log --format=%H <onto> -- <path>`, newest first, `<path>` being the component's `SPEC.md` as `clm-model` gives it at `onto` (no `--follow`), parsing the file at each commit with `clm_model::load_files` and skipping a commit whose file doesn't parse. A git failure during the walk is a git failure. The diff is `git diff --no-index --no-color -U3` run through `clm-git`, from the first `@@` on, with both texts ending in one newline so git prints no `\ No newline` line. The merge line names the element's hash in the tree it was applied to rather than leaving it to `clm hashes`, which reads the working tree's `specs/`. The diff is the only place the author sees `main`'s new text when they rebase: updating the hashes alone would pass every check and drop that text when the branch lands.

**Merge** (`stale-delta-three-way-merge`). Each element is merged with `git merge-file -p --quiet -L delta -L base -L main <delta> <base> <main>` through `clm-git` on three temporary files: the element's text in the delta (a requirement's block as `clm-model` gives it, with its clean heading), the base's text and the element's text in the tree, each trimmed of leading and trailing blank lines and ending in one newline. Exit 0 is a clean merge; an exit from 1 to 127 is the number of conflicts; any other exit, or git failing to start, is a git failure. The merged text replaces the element's text in the delta, located with `clm_model::outline`'s spans as `clm-apply` locates them, keeping the heading's slug and the surrounding blank lines. The base is replaced in place: `<key>: "<old base>"` in the frontmatter, so a flow map and a block map are both edited with the other keys untouched, and the `<!-- base: <hash> -->` comment in a requirement heading. A delta with CRLF line endings is never merged. An archived change's deltas are under `changes/archive/<id>/deltas/`, which `gate/archive-immutable` lets the branch edit. The `merged` lines are given on every outcome, before anything left. The merge takes the text, not the judgment: the merged section is in the branch's diff, `clm check` judges the result (`judged-by-check`), and on a branch that needs review the changed fingerprint asks a human to approve again (`tree-diff-fingerprint`).

**Other deltas.** An `applied` change is never read, since its bases record the text it was applied to. `Tree::other_deltas` stops at the first change still refused after the merge, as `clm rebase`'s regenerate step stops at the first change with violations. Only this branch's applied changes can have made such a delta stale, since one stale against `onto` itself fails `clm check` there, and the other change's author could do nothing about it until this branch lands.

**A transient `index.lock`** (`retry-index-lock`). Several sessions share one clone (`session-worktrees`, `build-places`), and Claude Code, hooks and the panel run `git` in it between a session's commands. The error comes back unchanged so a caller that reads a failure's stderr sees it. The wrapper keys on the stderr text, not the exit code: `git rebase` exits 1 on the lock and `git add` and `git commit` 128. `merge-file` and `diff --no-index` take no lock, and `clm-git`'s `fetch`, `current_branch` and `extract_paths` take none or use a temporary index, so none of them goes through it. It lives here rather than in `clm-git` because only `clm rebase`'s two crates need it (`shared-plumbing-in-git`).

## Requirements

### Requirement: regenerated-tree
`clm-regenerate` SHALL offer `regenerated(root, onto, ids, merge)`, the one place that builds a regenerated `specs/`, which `clm rebase`'s regenerate step (`reapply`) and `land`'s derived-specs check both call: in a temporary directory, `onto`'s `specs/` (none when `onto` has none) with `HEAD`'s `changes/`, `backlog/`, `docs/decisions/`, `ARCHITECTURE.md` and `CAPABILITIES.md` (every other path `clm_model::load` reads, so the graphs it loads hold what `clm check`'s does), each taken under the CLM root's prefix in the repository when the root is a subdirectory of it, then the changes of `ids` re-applied one at a time with `clm-apply`'s `reapply`, each onto the tree the ones before it left, the next being, of the changes left, the first in ID order none of whose recorded bases is unknown against the tree so far (`stale-delta-diff`), or the first in ID order when every change left has an unknown base, whatever order `ids` lists them in. It SHALL apply them stopping at the first change that has violations under its own directory (all the violations when the change doesn't load) or that doesn't apply. It SHALL return the tree, with its stages and the `merged` lines, or, as `Regenerated::Stopped`, the `merged` lines and the lines that stopped it: the violations as `check` prints them, followed by the stale-delta report (`stale-delta-diff`). With `merge` it SHALL first merge a stale delta as `stale-delta-merge` states; without it, it SHALL change nothing in `root`'s working tree, index or history.

#### Scenario: regenerated without merging
- GIVEN a branch whose `HEAD` adds applied change `x`, whose delta adds requirement `a/r`, and an `onto` that has since added requirement `b/s`
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN the tree's `specs/a/SPEC.md` holds `a/r` and its `specs/b/SPEC.md` holds `b/s`, and `HEAD`, the index and the working tree are unchanged

#### Scenario: change built on another sorts first
- GIVEN a branch whose `HEAD` adds applied changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Design with base `h1`, `onto`'s hash, and `w`'s rewrites its second line with base `h2`, the Design's hash after `x` applies
- WHEN `regenerated(root, onto, ["w", "x"], false)` runs
- THEN it returns the tree, the stages are `onto`'s, then the tree after `x`, then the tree after `w`, and the tree's Design holds both edits

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
- THEN the tree holds `HEAD`'s `ARCHITECTURE.md`, `CAPABILITIES.md` and `docs/decisions/`, the graph `clm_model::load` reads from it has the entry `architecture/e`, and `clm-apply`'s `validate` of `y` against that graph reports no `apply/resulting-graph` violation

#### Serves
- architecture/reuse-first
- architecture/regenerate-dont-merge

### Requirement: stale-delta-diff
Only a base a delta records for a Contract, Design or requirement it MODIFIES or REMOVES counts here, never a frontmatter or asset base nor one `apply` accepts, such as that of a MODIFIED requirement without its own `#### Serves` block whose rule `onto` changed only in its `#### Serves` block. The stages of the tree are `onto`'s `specs/`, then the tree after each change applied before, in order. Such a base is found at the newest stage where the element hashes to it, an element absent from a stage never matching, or, when no stage has it, at the newest commit in `onto`'s history of the component's `SPEC.md` where the element hashes to it; it is unknown when neither has it.

When `regenerated` stops on a change with stale deltas, or `Tree::other_deltas` stops on one (`stale-delta-merge`), the lines it gives SHALL, after the stale-delta violations and for the change that stopped it, hold for each such element whose recorded base is stale against the tree the change was applied to as `apply/stale-base` judges it, taken from `clm-apply`'s `stale_bases` (`apply/stale-elements`) rather than by comparing hashes itself, and that the merge left stale, a line `<movers> changed <element> of <component> since <base>`, with `<element>` named as `clm hashes` names it and `<movers>` naming what moved the element since its base, joined with ` and `: `main` when no stage has the base, then each change applied after the stage where the base is found (after none when no stage has it) whose application changed the element's hash, in ID order. It SHALL NOT name `main` when a stage has the base, nor a change whose application left the element's hash as it was. The line SHALL be followed by the hunks of a unified diff from the element's text where the base is found to its text in that tree (empty when the element is gone), and then a line `merge the changes above into <delta path>, then set its base to <hash>`, naming the delta's repository-relative path and the element's hash in that tree, or, when the element is gone from the tree, `rework <delta path> by hand: <component> no longer has <element>`. When the base is unknown, it SHALL give the single line `no text for <element> of <component> at <base>; compare <delta path> with <element> of <component> after <movers> by hand` in place of the other three. It SHALL give nothing of this for a missing base or for a base that doesn't count.

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
`regenerated` with `merge` SHALL, when a change of `ids` has stale deltas and before it reports them, merge three-way each Contract, Design or requirement a delta of that change MODIFIES whose recorded base is stale against the tree the change was applied to as `clm-apply`'s `stale_bases` judges it (`apply/stale-elements`), when the element is still in that tree and its base isn't unknown (`stale-delta-diff`): the delta's text, the base's text and the tree's text. For each element that merges without conflict it SHALL replace the element's text in the delta file with the merged text, set the element's recorded base to its hash in that tree, change nothing else in the file, and give the line `merged <element> of <component> into <delta path>`; it SHALL commit in `root` the delta files it changed, and nothing else, as `Merge <movers> into <change>`, naming the movers of the merged elements as `stale-delta-diff` names them, then apply the change again, keeping the commit when an element is left. `Tree::other_deltas(root, ids, onto)` SHALL merge the same way, against the regenerated tree, the deltas of each live change at `HEAD` whose status is `proposed` or `approved` and that `ids` doesn't name, in ID order, committing one `Merge <movers> into <change>` per change; when `apply`'s validation of such a change against the tree still refuses it after the merge, for a stale base or any other reason, it SHALL stop there and give the change's violations and the report `stale-delta-diff` gives for the elements left stale. Both SHALL leave unchanged, for `stale-delta-diff` to report, an element whose edits conflict, an element a delta REMOVES or that is gone from the tree (a requirement absent there, or a Contract or Design whose text there is empty), an element of a delta that removes its component, and an element whose base is unknown, and SHALL NOT merge a frontmatter or asset base, or an element whose base `apply` accepts. `regenerated` SHALL NOT edit the delta of a change `ids` doesn't name, and `Tree::other_deltas` SHALL NOT edit the delta of an `applied` or archived change.

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

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness

#### Needs
- apply/stale-elements

### Requirement: retry-index-lock
When a command that `clm-regenerate`'s `git` or `run` runs fails with stderr naming `index.lock`, it SHALL run the same command again after a wait, up to four more times with waits of 0.25, 0.5, 1 and 2 seconds, and SHALL return the last run's `clm-git` error unchanged when every run fails. It SHALL NOT retry a command that fails for another reason.

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

### Requirement: git-through-layer
`clm-regenerate` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `run` or `extract_paths`, the first two through the wrapper `retry-index-lock` states. No file under `crates/clm-regenerate/src` SHALL hold `fn extract(` or `--ignore-skip-worktree-bits`, extracting only through `extract_paths` (`extract-through-layer`). Its `Error` SHALL be `clm-git`'s `Failure`, and no file under `crates/clm-regenerate/src` SHALL hold a string literal that begins `"origin`, the base being the caller's `onto` (`shared-plumbing-in-git`). No file under `crates/clm-sync/src` SHALL define `fn regenerated(`, `fn retried(`, `fn merge_file(` or `fn other_deltas(`, nor declare `mod merge`, `mod stale`, `mod stages` or `mod lock`: those are `clm-regenerate`'s alone (`no-dead-code`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-regenerate` reads `crates/clm-regenerate/Cargo.toml` and every file under `crates/clm-regenerate/src`
- THEN `clm-git` is a dependency, and no file holds `Command::new("git")`, `Command::new("gh")`, `"origin`, `fn extract(` or `--ignore-skip-worktree-bits`

#### Scenario: no copy left in clm-sync
- GIVEN the workspace
- WHEN the same test reads every file under `crates/clm-sync/src` through `CARGO_MANIFEST_DIR`
- THEN no file holds `fn regenerated(`, `fn retried(`, `fn merge_file(`, `fn other_deltas(`, `mod merge`, `mod stale`, `mod stages` or `mod lock`

#### Serves
- architecture/reuse-first
- architecture/no-dead-code
