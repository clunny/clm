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
The regenerated `specs/` a branch's applied changes give on top of a base, built in one place for `clm rebase` (`sync`) and `clm land`'s derived-specs check (`land/derived-specs`): `onto`'s `specs/` with each change re-applied in ID order (`regenerate-dont-merge`). On the way it merges a stale delta three-way where the edits don't overlap and reports, with a diff and who moved it, each element left stale (`stale-delta-merge`, `stale-delta-diff`); after `clm rebase` has mirrored the tree, it merges the deltas of the other live changes the tree made stale (`other-delta-merge`). A `git` command it or `clm-sync` runs that meets a held `index.lock` is run again (`retry-index-lock`). Crate `clm-regenerate`, which reaches no `clm-health`.

## Design
Crate `clm-regenerate`, carved out of `clm-sync` (`sync-regenerate-split`). It depends on `clm-model`, `clm-apply`, `clm-check` (for `format`, a violation as `clm check` prints it) and `clm-git`, and never on `clm-health`, `clm-ledger` or `clm-context`, so `clm-land` can depend on it without reaching `clm-health` (`clm/health-isolated`). It offers `regenerated`, `Regenerated` and `Tree` (`regenerated-tree`); `Tree::other_deltas(&mut self, root, ids, onto)`, the other-deltas merge against the tree, which reads and extends the tree's stages (`Tree::path` and `Tree::merged` stay public for `clm-sync`'s mirror), returning the `merged` lines and what is left (`other-delta-merge`); and `git` and `run`, `clm-git`'s, retried on a held `index.lock` (`retry-index-lock`), which every `git` command `clm-sync` runs in the repository goes through too; and the helpers both crates use, `extract_present`, `exists_at` and `io` (with whatever they call), which `clm-sync` calls rather than copies (`reuse-first`). `clm-sync`'s `succeeded` stays `clm-sync`'s unless `exists_at` needs it. Its `Error` is `clm-git`'s `Failure`, as `clm-sync`'s is (`shared-plumbing-in-git`). `clm-sync` re-exports nothing of it: `clm-land` depends on `clm-regenerate` itself, and a re-export nothing calls would be dead code (`no-dead-code`).

**The tree** (`regenerated-tree`, `extract-through-layer`). `regenerated` extracts `onto`'s `specs/` into one temporary directory and `HEAD`'s `changes/`, `backlog/`, `docs/decisions/`, `ARCHITECTURE.md` and `CAPABILITIES.md`, the rest of what `clm_model::load` reads, into a second with `clm-git`'s `extract_paths`, each path under the CLM root's prefix and only when the revision has it, and moves the second's top-level entries into the first, so `clm_model::load` reads one tree whose graph has the human layers' entries and the decisions, as `clm check`'s does. Without them every `serves` and `#### Serves` naming an entry is broken on both sides of `apply`'s before-and-after comparison, so only a rule a change adds reads as new (`apply/resulting-graph`), and `clm rebase` refused a proposed change that `clm check` passed. For each change of `ids` in order it loads the tree, keeps the graph as a stage, filters the load's violations to those under the change's own directory (only the change's own files can keep it from applying; `clm check` judges the rest), and applies the change with `clm-apply`'s `reapply`.

**Stages.** The regenerate keeps, for the report and the merges, each component's `clm hashes` and element texts at each *stage* of the tree: `origin/main`'s `specs/` as extracted, then the tree after each change it applied, in order (the graphs the regenerate loads anyway). A recorded base's *origin stage* is the newest stage where the element hashes to it (so when a later change restores an earlier hash, the base is found at the later stage); the base's text is the element's text there. Stages are compared one to the next by `clm hashes`, an element absent at a stage (a component the change creates, a requirement it removes) counting as a hash of its own. Only when no stage has the base does it look in `origin/main`'s history: it walks `git log --format=%H origin/main -- <path>`, newest first, where `<path>` is the component's `SPEC.md` path as `clm-model` gives it at `origin/main` (no `--follow`). It parses the file at each commit with `clm_model::load_files`, skipping a commit whose file doesn't parse, hashes it with `hashes`, and takes the element's text at the first commit where it hashes to the recorded base; a git failure during the walk is a git failure, exit 2. The *movers* of a stale element are `main`, when no stage has the base (the base is older than `origin/main`'s text, or was never in the tree), and each change applied after the origin stage (every one when there is none) whose application changed the element's hash. On a fold, where several changes on one branch modify one Design and each delta's base is `origin/main`'s hash, the movers are the earlier changes and `main` isn't among them. It prints `<movers> changed <element> of <component> since <base>`, `<movers>` being `main` and then the changes' IDs in ID order, joined with ` and `, then the hunks (lines from the first `@@` on) of `git diff --no-index --no-color -U3` from the base's text to the element's text in the tree it was applied to (empty when the element is absent there, so every line shows as `-`), run through `clm-git` with both texts ending in one newline so git prints no `\ No newline` line, then, when the element is in the tree, `merge the changes above into <delta path>, then set its base to <hash>`, where `<delta path>` is repository-relative and `<hash>` the element's hash in that tree, printed rather than left to `clm hashes <component>`, which reads the working tree's `specs/` (on a fold, not the tree the stopped change was being applied to); when the element is absent from the tree, `rework <delta path> by hand: <component> doesn't have <element>` instead. The diff helper is `clm-regenerate`'s own. When neither a stage nor a commit has the base (history rewritten, a base typed by hand, or a base an earlier change produced whose tree no stage reaches because `main` has since moved the element and the earlier delta was merged with it), it prints the single line `no text for <element> of <component> at <base>; compare <delta path> with <element> of <component> after <movers> by hand` in place of the header, hunks and merge line. The diff is the only place the author sees `main`'s new text at the moment they rebase; without it, updating the hashes alone passes every check and drops that text from `main` when the branch lands.

**Merge** (`stale-delta-merge`, `stale-delta-three-way-merge`). When `clm-apply` refuses a change for stale bases, before reporting, it takes each delta of the change in node-ID order and, in `clm hashes` order, each element the delta MODIFIES (never one it REMOVES, nor anything of a `remove: true` delta, nor a frontmatter or asset base) that `stale_bases` names for the tree the change was being applied to, whose element is still in that tree (a requirement present, a Contract or Design whose text there isn't empty), and whose base text is found as the report finds it: at the base's origin stage, else by the history walk. For each it runs `git merge-file -p --quiet -L delta -L base -L main <delta> <base> <main>` through `clm-git` on three temporary files: the element's text in the delta (a requirement's block as `clm-model` gives it, with its clean heading), the base's text, and the element's text in the tree, each with leading and trailing blank lines trimmed and ending in one newline. Exit 0 is a clean merge. An exit from 1 to 127 is the number of conflicts, and the element is left as it was. Any other exit, or git failing to start, is a git failure, exit 2. For each clean merge it writes the merged text in place of the element's text in the delta file (the section's body under `## MODIFIED Contract` or `## MODIFIED Design`, or the requirement's block, both located with `clm_model::outline`'s spans as `clm-apply` locates them, so a block runs from its `### Requirement:` heading to the next such heading or the end of its section; the heading line keeps its slug, and the blank lines that surrounded the old text are kept), and sets the element's recorded base to its hash in the tree: in the delta's frontmatter it replaces `<key>: "<old base>"` (`contract` or `design`, so a flow map `base: { ... }` and a block map are both edited in place, with the other keys untouched), and in a requirement heading the `<!-- base: <hash> -->` comment. A new archived change's deltas are under `changes/archive/<id>/deltas/`, which `gate/archive-immutable` lets the branch edit, since the merge base doesn't have them. When it merged anything, it writes the delta files to the working tree and to the temporary tree, stages only those files, commits them as `Merge <movers> into <change>`, where `<movers>` are the movers of the merged elements taken together (`main` first, then change IDs in ID order, joined with ` and `), then loads the temporary tree again and applies the change once more. With nothing left stale the regenerate carries on to the next change; otherwise the elements left stale get the report above. The `merged` lines are printed on every outcome, before any stale-delta violation, report or check violation, and the run exits 1 with the merge commit kept, so the caller resolves only what is left and runs `clm rebase` again. A delta the merge didn't touch is never rewritten, and a delta with CRLF line endings is left as it is. The merge takes the text, not the judgment: two edits on different lines can still contradict each other, so the merged section is in the branch's diff, `clm check` judges the result (`judged-by-check`), and on a branch that needs review the changed fingerprint asks a human to approve the merged text again (`tree-diff-fingerprint`).

**Other deltas** (`other-delta-merge`). Once every change it regenerates has applied and the regenerated `specs/` is mirrored and committed as `Re-apply` (so the delta edits below are never swept into that commit, which adds `specs/` and commits the whole index), and only when it regenerated one, it takes each live change at `HEAD` whose status is `proposed` or `approved` (the statuses `clm check` validates with `clm_apply::validate`) and that it didn't regenerate, in ID order, stopping at the first one still refused as `clm rebase`'s step 4 stops at the first change with violations: a change `main` holds, or one the branch adds without applying it. Such a delta is stale when the regenerated tree moved an element it MODIFIES, which only this branch's applied changes can have done (a delta stale against `origin/main` itself fails `clm check` there), and `clm rebase`'s step 8 would stop the branch on it while the other change's author can do nothing about it until this branch lands. So it merges each such delta as **Merge** does, against the regenerated tree, the last stage; the files it changed are staged alone and committed as `Merge <movers> into <change>`, one commit per change, the movers being the regenerated changes that moved the elements (and `main`, should a base be older than `origin/main`'s text). A change `clm_apply::validate` still refuses on the regenerated tree after the merge stops the run as `clm rebase`'s step 4's stop does: it prints the change's violations (`validate`'s, and `load`'s under the change's directory; a refusal that isn't a stale base, such as a requirement the branch removed that the change modifies, stops it the same way, moving earlier the violation `clm check` would print at `clm rebase`'s step 8), then the report for the elements left stale, and exits 1, keeping the `Re-apply` and `Merge` commits. An `applied` change on `main`, whose bases record the text it was applied to and so differ from the tree by nature, and an archived change are never read here. `clm check` judges the result (`judged-by-check`), and an edited delta makes the branch need review only when its change declares `human_burdens` (`yes-no-review`), as rebasing that delta by hand on this branch would too.

**A transient `index.lock`** (`retry-index-lock`). Several sessions share one clone by design (`session-worktrees`, `build-places`), and Claude Code, hooks and the panel run `git` in it between a session's commands. Every `git` command `clm-sync` and `clm-regenerate` run in the repository through `clm-git`'s `git` and `run` goes through one wrapper, `clm-regenerate`'s (`clm-git`'s helpers `fetch`, `current_branch` and `extract_paths` take no index lock or use a temporary index, so they need none). The waits are fixed, with no override, so a test whose lock never lifts takes about 4 seconds. The wrapper returns `clm-git`'s error unchanged, so the callers that read a failure's stderr (the rebase's conflict path, `succeeded`) see it. It keys on the stderr text, not the exit code: `git rebase` exits 1 on the lock (`error: Unable to create '<path>/index.lock': File exists`, then `could not detach HEAD`, leaving no rebase in progress), `git add` and `git commit` 128 (`fatal: Unable to create ...`). A test proves the retry without racing a timer by creating the lock from a hook that runs before the command takes it (`pre-rebase`, or `post-checkout` after the rebase's last checkout for the `Re-apply` commit) and removing it on the retried attempt, so the first attempt fails whatever the load. The rebase is covered too: git takes the lock before it does anything, so a retried start is a clean start; a lock met mid-rebase leaves the rebase in progress, and the retry then fails on that. `merge-file` and `diff --no-index` take no lock and never say so, so the wrapper never retries them. The wrapper lives in `clm-regenerate`, the lower of `clm rebase`'s two crates, not `clm-git`, since the lock is this command's concern alone: `clm claim` retries a rejected push by a rule of its own (`orchestrator`); should a third crate need the lock retry, it moves to `clm-git` (`shared-plumbing-in-git`).

## Requirements

### Requirement: regenerated-tree
`clm-regenerate` SHALL offer `regenerated(root, onto, ids, merge)`, the one place that builds a regenerated `specs/`, which `clm rebase`'s regenerate step (`reapply`) and `land`'s derived-specs check both call: in a temporary directory, `onto`'s `specs/` (none when `onto` has none) with `HEAD`'s `changes/`, `backlog/`, `docs/decisions/`, `ARCHITECTURE.md` and `CAPABILITIES.md` (every other path `clm_model::load` reads, so the graphs it loads hold what `clm check`'s does), each taken under the CLM root's prefix in the repository when the root is a subdirectory of it, then each change of `ids` in order re-applied with `clm-apply`'s `reapply`, stopping at the first change that has violations under its own directory (all the violations when the change doesn't load) or that doesn't apply. It SHALL return the tree, with the stages (`stale-delta-names-movers`) and the `merged` lines that `clm rebase`'s mirror, `Re-apply` commit and `other-delta-merge` go on to read, or the lines that stopped it: the violations as `check` prints them, followed by the stale-delta report (`stale-delta-diff`), as `clm rebase` prints them today. With `merge` it SHALL first merge a stale delta as `stale-delta-merge` states; without it, it SHALL change nothing in `root`'s working tree, index or history.

#### Scenario: regenerated without merging
- GIVEN a branch whose `HEAD` adds applied change `x`, whose delta adds requirement `a/r`, and an `onto` that has since added requirement `b/s`
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN the tree's `specs/a/SPEC.md` holds `a/r` and its `specs/b/SPEC.md` holds `b/s`, and `HEAD`, the index and the working tree are unchanged

#### Scenario: stale delta without merging
- GIVEN the same branch, with `x`'s delta also modifying a requirement of `a` that `onto` has changed since the delta's base
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN it returns the `apply/stale-base` violation followed by the report saying `main changed` the requirement, and `HEAD`, the index and the working tree are unchanged

#### Scenario: human layers in the tree
- GIVEN a branch whose `HEAD` adds applied change `x` and `proposed` change `y`, whose delta adds to component `a` a requirement whose `#### Serves` names `architecture/e`, an entry of `HEAD`'s `ARCHITECTURE.md`, and on which `clm check` passes
- WHEN `regenerated(root, onto, ["x"], false)` runs
- THEN the tree holds `HEAD`'s `ARCHITECTURE.md`, `CAPABILITIES.md` and `docs/decisions/`, the graph `clm_model::load` reads from it has the entry `architecture/e`, and `clm-apply`'s `validate` of `y` against that graph reports no `apply/resulting-graph` violation

#### Serves
- architecture/reuse-first
- architecture/regenerate-dont-merge

### Requirement: stale-delta-diff
When the regenerate step stops on a stale delta, or the other-deltas step does (`other-delta-merge`), `clm rebase` SHALL, after the stale-delta violations and for the change that stopped it, print for each element a delta MODIFIES or REMOVES (a Contract, a Design or a requirement) whose recorded base is stale against the tree the change was applied to as `apply/stale-base` judges it, taken from `clm-apply`'s `stale_bases` (`apply/stale-elements`) rather than by comparing hashes itself, and that the merge left stale, a line `<movers> changed <element> of <component> since <base>`, with `<element>` named as `clm hashes` names it and `<movers>` naming what moved the element since its base, joined with ` and `: `main` when no stage of the tree (`origin/main`'s `specs/`, then the tree after each change applied before, in order) has the element at the base, then each change applied after the newest stage that has it (after none when no stage does) whose application changed the element's hash, in ID order. It SHALL NOT name `main` when a stage has the base, nor a change whose application left the element's hash as it was. The line SHALL be followed by the hunks of a unified diff from the element's text at that stage, or, when no stage has the base, at the newest commit in `origin/main`'s history of the component's `SPEC.md` where the element hashes to the base, to its text in that tree (empty when the element is gone), and then a line telling the caller to merge those changes into the delta's path and then set its base to the element's hash in that tree, which it names, or, when the element is gone from the tree, to rework the delta by hand. When neither a stage nor a commit in that history has the base, it SHALL print the single line `no text for <element> of <component> at <base>`, telling the caller to compare the delta with the element after the movers by hand, in place of the other three. It SHALL print nothing of this for a missing base, for a frontmatter or asset base, or for an element whose base `apply` accepts, such as a MODIFIED requirement without its own `#### Serves` block whose rule `main` changed only in its `#### Serves` block. It SHALL still exit 1, and the regenerate step SHALL still write nothing to `specs/`.

#### Scenario: main changed a line the delta changed
- GIVEN an applied spec branch whose delta rewrites the line `Do y.` of `a`'s Contract with base `h1`, and an `origin/main` that has since changed that line to `Do y. Also z.`
- WHEN `clm rebase` runs
- THEN it prints `main changed contract of a since h1`, then a hunk with the line `+Do y. Also z.`, then the merge instruction naming the delta's path and the hash `clm hashes a` gives on `origin/main`, exits 1, and `specs/` is unchanged by the regenerate step

#### Scenario: requirement changed on main
- GIVEN an applied spec branch whose delta modifies requirement `a/x` with base `h1`, and an `origin/main` that has since changed a scenario line of `a/x` that the delta also changed
- WHEN `clm rebase` runs
- THEN it prints `main changed requirement:x of a since h1`, then a hunk with the old scenario line as `-` and the new one as `+`, and exits 1

#### Scenario: base not in history
- GIVEN an applied spec branch whose delta modifies `a`'s Design with a base that no commit of `origin/main`'s `specs/a/SPEC.md` hashes to
- WHEN `clm rebase` runs
- THEN it prints `no text for design of a at <base>` with the instruction to compare by hand after `main`, prints no hunk and no merge line, and exits 1

#### Scenario: element gone from the tree
- GIVEN an applied spec branch whose delta modifies requirement `a/r` with base `h1`, and an `origin/main` that has since removed `a/r`
- WHEN `clm rebase` runs
- THEN it prints `main changed requirement:r of a since h1`, then a hunk with every line of `a/r`'s old text as `-`, then `rework <delta path> by hand: a no longer has requirement:r` naming the delta's path, and no merge instruction, and exits 1

#### Scenario: an earlier change on the branch moved it
- GIVEN an applied spec branch adding changes `x` and `y`, `x` before `y` in ID order, whose deltas both modify `a`'s Design with base `h1`, `origin/main`'s hash, each rewriting its first line differently, and an `origin/main` that hasn't changed `a` since
- WHEN `clm rebase` runs
- THEN it prints the stale-base violation naming `y`'s delta, then `x changed design of a since h1` and no `main changed` line, then a hunk with `x`'s first line as `+`, then the merge instruction naming `y`'s delta path and the hash of `a`'s Design after `x` applies, and exits 1

#### Scenario: main and an earlier change moved it
- GIVEN the same branch, and an `origin/main` that has since changed the last line of `a`'s Design
- WHEN `clm rebase` runs
- THEN it prints `merged design of a into changes/x/deltas/a.md`, then `main and x changed design of a since h1`, with a hunk holding `main`'s last line and `x`'s first line as `+`, and exits 1

#### Scenario: serves edited on main
- GIVEN an applied spec branch whose delta modifies `a/p`, its block without a `#### Serves` block, and `a/q`, each with its hash before `origin/main` moved it as its base, and an `origin/main` that has since edited only `a/p`'s `#### Serves` block and the line of `a/q` that the delta changed
- WHEN `clm rebase` runs
- THEN it prints the stale-base violation for `a/q` and `main changed requirement:q of a since <base>`, prints no violation or line naming `requirement:p`, and exits 1

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness

#### Needs
- apply/stale-elements

### Requirement: stale-delta-merge
`clm rebase`'s regenerate step SHALL, when a change the branch adds has stale deltas and before it reports them, merge three-way each Contract, Design or requirement a delta of that change MODIFIES whose recorded base is stale against the tree the change was applied to as `clm-apply`'s `stale_bases` judges it (`apply/stale-elements`), when the element is still in that tree and the base's text is found as `stale-delta-diff` finds it: the delta's text, the base's text and the tree's text. For each element that merges without conflict it SHALL replace the element's text in the delta file with the merged text, set the element's recorded base to its hash in that tree, change nothing else in the file, and print `merged <element> of <component> into <delta path>`; it SHALL commit the delta files it changed, and nothing else, as `Merge <movers> into <change>`, naming the movers of the merged elements as `stale-delta-diff` names them, then apply the change again. It SHALL leave unchanged, for `stale-delta-diff` to report, an element whose edits conflict, an element the delta REMOVES or that is gone from the tree (a requirement absent there, or a Contract or Design whose text there is empty), an element of a delta that removes its component, and an element whose base has no text. It SHALL keep the merge commit when an element is left, and SHALL NOT merge a frontmatter or asset base, or an element whose base `apply` accepts, or edit the delta of a change the branch doesn't regenerate, which `other-delta-merge` covers.

#### Scenario: edits apart
- GIVEN an applied spec branch `spec/x` whose delta `changes/x/deltas/a.md` rewrites the first line of `a`'s Contract with base `h1`, and an `origin/main` that has since changed only the last line of `a`'s Contract
- WHEN `clm rebase` runs
- THEN it prints `merged contract of a into changes/x/deltas/a.md`; a commit `Merge main into x` changes only that delta, which now holds both edits and records as its contract base the hash `clm hashes a` gives on `origin/main`; `specs/a/SPEC.md`'s Contract holds both edits; and it exits 0

#### Scenario: edits overlap
- GIVEN an applied spec branch whose delta rewrites a line of `a`'s Contract with base `h1`, and an `origin/main` that has since changed the same line
- WHEN `clm rebase` runs
- THEN it prints no `merged` line, makes no `Merge main into` commit, leaves the delta unchanged, prints `main changed contract of a since h1`, and exits 1

#### Scenario: one merged, one left
- GIVEN an applied spec branch `spec/x` whose delta modifies requirements `a/p` and `a/q`, and an `origin/main` that has since changed a scenario line of `a/p` the delta didn't change and the same line of `a/q` the delta changed
- WHEN `clm rebase` runs
- THEN it prints `merged requirement:p of a into changes/x/deltas/a.md` and `main changed requirement:q of a since <base>`, `HEAD` is a commit `Merge main into x` in which `a/p`'s block and base are updated and `a/q`'s are unchanged, `specs/a/SPEC.md` is `origin/main`'s, and it exits 1

#### Scenario: base from an earlier change
- GIVEN an applied spec branch adding changes `x`, `y` and `z`, in that ID order, whose deltas all modify `a`'s Design: `x` with base `h1`, `origin/main`'s hash, rewriting its first line; `y` with base `h1`, rewriting its second line; and `z` with base `h2`, the Design's hash after `x` applies to `origin/main`'s, rewriting its third line; and an `origin/main` that hasn't changed `a` since
- WHEN `clm rebase` runs
- THEN it prints `merged design of a into changes/y/deltas/a.md` and `merged design of a into changes/z/deltas/a.md`, the branch has commits `Merge x into y` and `Merge y into z`, `specs/a/SPEC.md`'s Design holds all three edits, and it exits 0

#### Scenario: section gone from main
- GIVEN an applied spec branch whose delta modifies requirement `a/r`, and an `origin/main` that has since removed `a/r`
- WHEN `clm rebase` runs
- THEN it prints no `merged` line, leaves the delta unchanged, and exits 1

#### Scenario: removed requirement
- GIVEN an applied spec branch whose delta removes requirement `a/r`, and an `origin/main` that has since changed `a/r`
- WHEN `clm rebase` runs
- THEN it prints no `merged` line, leaves the delta unchanged, and exits 1

#### Scenario: serves edited on main
- GIVEN an applied spec branch `spec/x` whose delta modifies `a/p`, its block without a `#### Serves` block, and `a/q`, each with its hash before `origin/main` moved it as its base, and an `origin/main` that has since edited only `a/p`'s `#### Serves` block and a scenario line of `a/q` the delta didn't change
- WHEN `clm rebase` runs
- THEN it prints `merged requirement:q of a into changes/x/deltas/a.md` and no `merged` line naming `requirement:p`; the commit `Merge main into x` updates `a/q`'s block and base and leaves `a/p`'s as they were; `specs/a/SPEC.md`'s `a/p` holds the delta's text with `origin/main`'s `#### Serves` block; and it exits 0

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness

#### Needs
- apply/stale-elements

### Requirement: other-delta-merge
When the regenerate step has applied every change it regenerates, `clm rebase` SHALL, for each live change at `HEAD` whose status is `proposed` or `approved` and that it didn't regenerate, in ID order, merge three-way into that change's deltas, as `stale-delta-merge` does and against the regenerated tree, each Contract, Design or requirement a delta MODIFIES whose recorded base `clm-apply`'s `stale_bases` judges stale against that tree (`apply/stale-elements`), when the element is in the tree and the base's text is found as `stale-delta-diff` finds it; print `merged <element> of <component> into <delta path>` for each; and commit the delta files it changed, and nothing else, as `Merge <movers> into <change>`, one commit per change, naming the movers as `stale-delta-diff` names them. When `apply`'s validation of such a change against that tree still refuses it after the merge, it SHALL print the change's violations and the report `stale-delta-diff` gives for the elements left stale, keep the `Re-apply` and `Merge` commits, and exit 1 without the steps that follow. It SHALL merge nothing on a branch that regenerates no change, and SHALL NOT edit the delta of an `applied` or archived change.

#### Scenario: edits apart
- GIVEN an `origin/main` with a `proposed` change `o` whose delta `changes/o/deltas/a.md` modifies `a`'s Design with base `h1`, `origin/main`'s hash, rewriting its last line, and an applied spec branch `spec/x` adding `x`, whose delta rewrites the first line of `a`'s Design
- WHEN `clm rebase` runs
- THEN it prints `merged design of a into changes/o/deltas/a.md`; a commit `Merge x into o` changes only that delta, which now holds both edits and records as its design base the hash of `a`'s Design in `HEAD`'s `specs/`; `clm check` passes; and it exits 0

#### Scenario: edits overlap
- GIVEN the same, where `o`'s delta rewrites the same first line
- WHEN `clm rebase` runs
- THEN it prints no `merged` line, the branch has no `Merge x into o` commit and `o`'s delta is unchanged, it prints the stale-base violation naming `changes/o/deltas/a.md` and then `x changed design of a since h1` with a hunk holding `x`'s line as `+`, and it exits 1

#### Scenario: proposed change the branch adds
- GIVEN an applied spec branch `spec/x` adding `x`, whose delta rewrites the first line of `a`'s Design, and a `proposed` change `y` whose delta modifies `a`'s Design with base `h1` rewriting its last line
- WHEN `clm rebase` runs
- THEN it prints `merged design of a into changes/y/deltas/a.md`, the branch has a commit `Merge x into y`, and it exits 0

#### Scenario: applied change on main left alone
- GIVEN an `origin/main` with an `applied` change `p` whose delta modifies `a`'s Design, and an applied spec branch `spec/x` adding `x`, whose delta rewrites a line of `a`'s Design
- WHEN `clm rebase` runs
- THEN it prints no `merged` line naming `changes/p/deltas/a.md`, that delta is unchanged, and it exits 0

#### Scenario: branch that regenerates nothing
- GIVEN a branch `spec/x` adding only a `proposed` change `x`, and an `origin/main` with a `proposed` change `o` whose delta modifies `a`'s Design with a base that isn't `a`'s Design hash there
- WHEN `clm rebase` runs
- THEN it prints no `merged` line, `o`'s delta is unchanged, and `clm check`'s stale-base violation for it is printed as today, exit 1

#### Serves
- capability/work-concurrently
- architecture/hash-based-staleness

#### Needs
- apply/stale-elements

### Requirement: retry-index-lock
When a `git` command `clm rebase` runs in the repository fails with stderr naming `index.lock`, `clm rebase` SHALL run the same command again after a wait, up to four more times with waits of 0.25, 0.5, 1 and 2 seconds, and SHALL report the last failure as a git failure, exit 2, when every run fails. It SHALL NOT retry a command that fails for another reason.

#### Scenario: lock lifted
- GIVEN branch `spec/x` behind `origin/main`, and the worktree's `index.lock` held by another process that removes it after half a second
- WHEN `clm rebase` runs
- THEN the branch is rebased onto `origin/main`, its last line is `rebase ok`, and it exits 0

#### Scenario: lock met at the commit
- GIVEN an applied spec branch `spec/x` adding `x`, and an `origin/main` that has since added a requirement to the same `SPEC.md`, so the regenerate commits `Re-apply x`, where the worktree's `index.lock` is held from the end of the rebase and released half a second later
- WHEN `clm rebase` runs
- THEN `HEAD` is a commit `Re-apply x`, nothing is left staged, its last line is `rebase ok`, and it exits 0

#### Scenario: lock kept
- GIVEN branch `spec/x` behind `origin/main`, with the lock held throughout
- WHEN `clm rebase` runs
- THEN it prints git's message naming `index.lock` on stderr, its last line is `rebase failed`, it exits 2, and it ends within ten seconds

#### Scenario: other failure not retried
- GIVEN a branch whose `origin` remote points at a path that doesn't exist
- WHEN `clm rebase` runs
- THEN its last line is `rebase failed`, it exits 2, and it ends within a second

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
