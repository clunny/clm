---
title: Sync
status: active
owners: ["@clunny"]
depends_on: [model, apply, check, check.health, context, ledger, vcs, sync.regenerate, rename]
paths: ["crates/clm-sync/**"]
successor: ""
serves: [capability/work-concurrently, architecture/hash-based-staleness]
---

## Contract
`clm rebase` brings the current branch up to date with `origin/main`: it fetches, rebases, regenerates derived files and runs `clm check`. It exits 0 when the branch is up to date and passes, 1 when something is left for the caller to resolve (with what and where), and 2 when git fails. Running it again after resolving carries on. It never pushes. Its last line of stdout always states the outcome: `rebase ok`, `rebase incomplete` or `rebase failed`, for exit codes 0, 1 and 2, so a caller that can't read the exit code (an unattended session, `one-command-per-call`) still knows it.

On a branch that adds an applied change, or a change it archived itself after applying it (`archive-in-spec-pr`), it rebases a stale delta itself where it can (`stale-delta-merge`): each section of the delta whose edits don't overlap what moved the section since the delta's base, `main`'s edits or an earlier change's on the same branch, is merged three-way, written into the delta with its new base, and committed as `Merge <movers> into <change>`, naming what moved it, printing `merged <element> of <component> into <delta path>`. A section whose edits overlap, one the delta removes or that is absent from the tree, and one whose base text git doesn't have are left for the caller, with what moved in each and who moved it (`stale-delta-diff`). The delta of another live `proposed` or `approved` change that the branch's apply made stale, on `main` or on the branch, is merged the same way against the branch's applied text and committed on its own (`stale-delta-merge`). A `git` command that fails on a transient `index.lock` is run again before the run fails (`retry-index-lock`).

On a branch that adds a change recording `renames` (`rename`), it regenerates the branch instead of rebasing it (`regenerate-renames`): the branch is one commit on `origin/main` holding what `clm rename` produces there, so a moved `main` never leaves a rename to merge by hand (`regenerate-dont-merge`).

On `impl/*` branches it also reports a changed component spec (`spec-changed`), archives complete changes (`archive-complete`, `archive-on-sync`), and files reconsider and proposal items (`file-reconsider-item`, `health-advises`, `file-proposal-items`, `web-proposals-as-ledger-records`). It files no item for a component over its size budget: the component shows as oversized in `clm status`, `clm size` and `clm health` (`oversized-shown-not-filed`).

`clm autosync` pushes a snapshot of the spec work on the current `spec/*` branch to `origin` as `autosync/<branch>` (`autosync`).

## Design
Code lives in crate `clm-sync` and uses the `git` CLI, not a git library. The regenerate's tree, its stages, the stale-delta report and both merges, and the `index.lock` retry every `git` command of the two crates goes through, are the child `sync.regenerate`'s, in crate `clm-regenerate`, which `clm-sync` calls (`sync-regenerate-split`); `clm-regenerate` reaches no `clm-health`, so `clm land` can call it (`clm/health-isolated`).

**`clm rebase`.**
1. It refuses (exit 1) on `main` or on a detached `HEAD`, unless a rebase that `clm rebase` started is in progress. Otherwise it fetches `origin` and then refuses (exit 1) with uncommitted work (`rebase`, `sync-ignores-untracked`, `sync-carries-unrelated-edits`): an edit its own steps could lose or commit, or that the steps judging `HEAD` would read in its place. Why each path counts:
   - a staged change: `git rebase --autostash` would hand it back unstaged, and the `Re-apply`, `Archive` and `Activate` commits commit the whole index;
   - a tracked file modified or deleted under `specs/`, `changes/` or `backlog/`, the trees steps 4, 6 and 7 write;
   - a tracked file modified or deleted that `origin/main` changed since the merge base of `HEAD` and the freshly fetched `origin/main` (`git diff --name-only --no-renames <merge base> origin/main`, so a file main renamed counts under its old path too): the rebase writes it, so restoring the edit afterwards could conflict; with no merge base, every modified or deleted tracked file;
   - on `impl/*`, any tracked file modified or deleted, since steps 6 and 7 judge `HEAD` by reading the working tree (its markers);
   - an untracked file that isn't ignored under `specs/` or `changes/`, the two trees steps 4 and 6 stage whole with `git add -A`.

   The tracked paths come from `git status --porcelain -z --no-renames --untracked-files=no` (any `X` other than a space is a staged change, unmerged entries included; any `Y` other than a space is a working-tree change, intent-to-add included), so a renamed file counts under both its old and its new path. On `impl/*` it refuses only after the fetch, so every branch takes one path. A refused run writes nothing under `.git/clm/`. The untracked files come from `git ls-files --others --exclude-standard -- specs changes`, which honours every ignore source and names files rather than directories. `uncommitted <path>` lines go on stdout, one path per line, sorted, before the outcome line, as `conflict <path>` does. Any other edit to a tracked file (a human's change to `.claude/settings.json` on a `spec/*` branch) is set aside and restored by the rebase (step 2), nothing below stages it, and step 8 judges `HEAD`'s commit, so it never counts toward the outcome. Any other untracked file (test output, another session's `.claude/worktrees/`) is never staged, removed or read; should the rebase need to write over one, git refuses with the path, and `clm rebase` reports that and exits 2.
2. It runs `git rebase --autostash origin/main` (or `git rebase --continue` when its rebase is in progress) with its merge driver configured on the command line: `-c merge.<name>.driver=...` and `-c core.attributesFile=.git/clm/attributes`, a file it writes (`sync-owns-merge-drivers`). `--autostash` keeps the edits step 1 let through in the rebase's own state (`.git/rebase-merge/autostash`, per worktree, never the shared stash list) and restores them when the rebase finishes, so while the rebase stops on a conflict they are absent from the working tree, and `git rebase --abort` restores them too. None is on a path the rebase writes, so restoring one never conflicts; should git still report that applying the autostash resulted in conflicts (it then exits 0 and keeps the edit on the stash list), `clm rebase` prints git's message and exits 2, so the caller recovers the edit by hand. When the branch adds a change, whatever its status (its `changes/<id>/change.md` present at `HEAD`, and absent both at `origin/main` and at the merge base recorded before step 2, or a *new archived change* as step 4 defines it), or applies a change as step 4 defines it, the attributes map `specs/**` to `clm-keep`, which keeps the side being rebased onto, and a conflict in `specs/` that the driver can't take, such as modify/delete, is resolved the same way: the path keeps `origin/main`'s side when it has one and is removed when it doesn't (`reapply`, `regenerate-dont-merge`, `keep-main-specs-on-spec-branches`). A change that isn't `applied` at `HEAD`, such as one set back to `proposed` to apply again after editing its deltas, still has its `Apply` commits in the branch's history, and replaying them over a `main` that changed the same `SPEC.md` must not stop the rebase on a file nobody wrote by hand. Which changes the branch adds, and which of them step 4 regenerates, are decided before the rebase starts and kept in the marker, so a resume uses the same driver and regenerates the same changes. On a branch that adds no change, such as a finish branch setting `status: active`, `specs/` is rebased like any other file.
3. When the rebase stops on a conflict that step 2 doesn't resolve, it prints each such file as `conflict <path>` and exits 1, leaving the rebase in progress for the caller to resolve, `git add`, and run `clm rebase` again. On a branch that adds a change, it resolves the stop's `specs/` conflicts as step 2 says before it reports the rest.
4. **Regenerate** (`reapply`, `regenerate-applied-specs`): when the branch adds a change whose status is `applied` or a *new archived change* whose status is `done`, or applies a change, it replaces `specs/` with `origin/main`'s and applies the change's deltas and assets with `clm-apply`, leaving each `change.md` where and as it is. The branch adds a *new archived change* `<id>` when `changes/archive/<id>/change.md` is present at `HEAD` and absent both at `origin/main` and at the merge base recorded before step 2, and `changes/<id>/change.md` is absent at both of those too (`sync-new-archived-change`): a change archived on its own spec branch after `clm apply` (`archive-in-spec-pr`), whose deltas `main` never applied. That is the gate's *new archived change* (`gate`'s Design) less a change that is live on `main` and that the branch moves into the archive (`archive-move`) or edits: `main`'s `specs/` already holds its deltas, so applying them again would only report stale bases. The branch *applies* change `<id>` (`regenerate-applied-live-changes`) when `changes/<id>/change.md` is at `origin/main` with status `proposed` or `approved` and, at `HEAD`, has status `applied`, or is absent while `changes/archive/<id>/change.md` has status `done` (applied and archived on the branch, `archive-in-spec-pr`): a step left `proposed` on `main`, such as a deferred contract change (`contract-step-deferred`), that a later spec branch applies. `main` never applied its deltas, so they are regenerated as an added change's are, whether or not the branch adds a change of its own; a change only the branch adds, or one `main` holds `applied`, isn't one. Live and archived changes, added or applied, are regenerated together, in the order `clm-regenerate` chooses (`sync.regenerate`'s Design, **Order**): a change after the ones whose output its deltas' bases record, and otherwise in ID order (`reapply-order-follows-bases`). A new archived change whose status isn't `done` (a hand edit left it `applied`, or set it `abandoned`) is added, so step 2 resolves its `specs/` conflicts, but isn't regenerated, as a live change that isn't `applied` isn't. A stale delta is first merged where it can be (**Merge**, `sync.regenerate`'s Design); a delta still stale after that stops it before it writes anything to `specs/`, printing `check`'s stale-delta violations for the elements left stale and then the report (`stale-delta-diff`, `stale-delta-shows-main`, `stale-delta-names-movers`). This covers the change that stopped it, the first in that order with violations, and its deltas in node-ID order. For each delta, in `clm hashes` order, each element it MODIFIES or REMOVES (a Contract, a Design, a requirement, or the Contract and Design of a component the delta removes) that `apply/stale-base` judges stale against the tree the change was being applied to (`origin/main`'s, plus any earlier change re-applied before it) and that the merge left stale gets a report; a missing base, a base recorded for a section the delta doesn't modify, and frontmatter and asset bases get none. A MODIFIED Frontmatter replaces only the keys it lists, and an asset is replaced whole. The stale elements come from `clm-apply`'s `stale_bases` on that tree (`apply/stale-elements`), not from parsing the violation messages or comparing whole hashes: a MODIFIED requirement whose block has no `#### Serves` block of its own and whose base's first 8 hex digits are the rule's version is current, so an edit to only that rule's `#### Serves` block neither lists nor merges it, and `clm rebase` and `clm apply` agree on which elements are stale. The tree, the merge and the report are `clm-regenerate`'s `regenerated` with its merge (`sync.regenerate`'s Design: **Stages**, **Merge**, **Other deltas**). `clm-sync` mirrors the tree into `specs/` and, when the result differs from `HEAD`, commits it as `Re-apply <change>`; then it has `clm-regenerate` merge the other live changes' deltas against the tree (`Tree::other_deltas`, `sync.regenerate`'s `stale-delta-merge`), and stops on what that leaves as it stops on a stale delta.
5. **Spec changed** (`spec-changed`, `versioned-markers`): on a branch `impl/<component>`, it compares the component's bundle (`clm context <component>`, with the component's assets) between the merge base recorded before step 2 and `origin/main`. `CAPABILITIES.md` counts when the capability blocks `clm context` includes differ: those of the capabilities the component serves, or every capability's for the component that builds the capability tests, which then reports any capability's change, since each is its work. The last printed line tells the session to run `clm pending <component>` and `git diff <merge base> origin/main -- <paths>` and to adapt its work to the new text. It exits 1 after step 8 has run, so the session reads the change before going on. The report compares two trees that git has, so it needs no recorded basis, and it fires once: the next run's merge base is `origin/main`. The session's markers cite the versions it built to, so a requirement that moved under it stays `behind` until it rebuilds and re-cites it; nothing has to be re-planned (`node-reconciliation`).
6. **Archive** (`archive-complete`, `archive-on-sync`): on any branch `impl/*`, for each change `archive-complete` names, with the markers scanned at `HEAD` and completeness as `check.coverage/status-report` defines it, it moves the change with `clm-apply`'s archive step as `clm archive` does. `gate/archive-move` and `gate/backlog-additions` rate the archive commit as code, so the PR stays a code PR. A change is complete only on a tree whose tests already cite its requirements, so the branch that completes it is the one that archives it, or, when two PRs complete it between them, the next `impl/*` branch to sync. The `Activate` commit of a `unbuilt` component is made with the archive commit or alone, and `gate/status-only-exemption` rates it as code. **Proposal items** (`file-proposal-items`, `web-proposals-as-ledger-records`): in the same step, after archiving, it folds the ledger (`ledger`'s `fold`, over the `origin/clm/ledger/*` refs step 1 fetched and the clone's own). A change at `HEAD` under `changes/` or `changes/archive/` names the proposal in `resolves` (the graph's backlog items and changes, archived ones included). The file's frontmatter is `title`, the title as a JSON string (`serde_json::to_string`, which YAML's double-quoted form reads, as `clm defect` writes its title), `kind: idea` and `components` in the record's order with the components the graph at `HEAD` doesn't hold left out (`model/backlog-components` refuses them; `components: []` when none is left), and nothing else: no `priority`, `blocks` or `origin`. The body is the record's `body`, trimmed of trailing line breaks, then a blank line and the line `Proposed in the web app by <writer> at <at> (record <id>).`. The file is built from the record alone, never from the clock or the branch, so two builds that file one proposal add the same bytes and their lands merge clean. A record whose `slug` isn't lowercase kebab case (`[a-z0-9]+(-[a-z0-9]+)*`) is skipped with a warning naming the record, since a hand-appended line could name any path. An item is judged to exist by the graph loaded for archiving, which reads the working tree, and also when an untracked `backlog/<slug>.md` is there, as a reconsider item is judged, so nothing is overwritten; a graph with violations files nothing, as it archives nothing. The fold is `ledger`'s over the refs as the run fetched them; a resumed run, which skips the fetch, reads the refs the run that started the rebase fetched. Each item is committed alone as `File <slug>`, before step 8's temporary worktree, so the check judges the new items. A new item never makes the code PR mixed (`gate/backlog-additions`). An item deleted without a change resolving it comes back on the next build's sync, as a reconsider item does; a proposal is closed by a change that resolves it or by its proposer withdrawing it. An open proposal whose slug an item filed by other means took (a reconsider or defect item) is never filed and stays listed in the app until its proposer withdraws it. Filing never changes the outcome or the exit code.
7. **Reconsider item** (`file-reconsider-item`, `health-advises`): on a branch `impl/<component>`, whenever the run reaches this step (so also after a `spec changed` report, but not after a conflict or a stale delta, which stop earlier), it scores the component's requirements with `check.health`'s `reconsider`, from the graph and backlog items at `HEAD` (read in the temporary worktree) and the `health` settings the `clm` crate passes in: the closure returns `clm_health::Settings` alone, and its one caller, `crates/clm/src/main.rs`, is edited by this component's build, as the root's Design gives every build that changes a command (`clm/root-from-cwd`'s Design, `contract-removal-needs`). Items are filed in requirement-ID order. It files for the branch's own component only, so two builds never write the same item. The item is a `question`, a decision only a human makes, which `clm status` lists under `waiting on you`, and one deleted without a spec change comes back on the component's next build. A flag or a score never changes the outcome or the exit code. No step measures the component's size or files an item for an oversized component (`oversized-shown-not-filed`); splitting waits until the re-founding's mapping and pruning have shrunk the components, when the AI decides splits without a human.
8. **Settings and check** (`judged-by-check`, `sync-carries-unrelated-edits`, `sync-judges-head`): on every branch that reaches step 5 (never after a conflict or a stale delta, which stop earlier), it judges the branch's commit, never the working tree. After step 6 and before step 7, it checks `HEAD` out detached into a fresh temporary worktree, `git worktree add --detach <dir> HEAD` through `clm-git`, where `<dir>` is a new `sync-*` directory under the system's temporary directory, chosen as `clm land` chooses it (`TMPDIR` when set and not empty, made absolute, else `std::env::temp_dir()`), outside the repository as `clm land`'s are (`scratch-outside-git`); `clm-sync` has its own copy of that rule, since it doesn't depend on `clm-land`. Its CLM root is `<dir>` joined with the CLM root's path relative to the repository's top level (`git rev-parse --show-prefix`). The temporary worktree holds `HEAD`'s tracked files and nothing else, so an edit step 1 let through (to `clm.toml`, to a test file's `req:` marker, to a decision under `docs/decisions/`), an untracked file outside `specs/` and `changes/`, and an ignored file never decide the outcome: what it judges is what `clm land` merges. The clone's `clm.local.toml` and the user's `config.toml` are found from it as from the working tree, since `config` finds the local file through the clone's common git directory. It calls the closure the `clm` crate passes in (`FnOnce(&Path)`), once, with that CLM root, and step 7 takes the result, the `health` settings; step 7 also reads the graph and the backlog items in the temporary worktree, while it still looks for an existing `backlog/reconsider-<component>-<slug>.md` and writes and commits the items in the repository's working tree: the closure loads the settings from `HEAD` as the rebase and steps 4 and 6 left it, so it reads the settings `main` brought in. When they don't load, such as a `clm.toml` with a key the running `clm` doesn't know or a broken user `config.toml`, it prints the closure's error, which the `clm` crate formats as `clm check` prints a violation (`<source>: check.health/health-report: <error>`, the source being the file that failed, or `settings`), after the `archived` and `filed` lines and before the check's violations, and exits 1. `clm-sync` replaces a leading temporary CLM root, in the closure's error and in each violation's path, with the CLM root it was itself given (so the source prints as `./clm.toml` when the `clm` crate passes `.`), so the line names the file the caller would edit, never one under the temporary directory. The check still runs and prints. `clm check` and `clm land` load the same settings and fail on them, so `rebase ok` never stands for a branch they would reject. When step 7 commits an item, it moves the temporary worktree to the new `HEAD` (`git checkout --detach <HEAD>` in it). Then it runs `clm check` on the temporary worktree's CLM root and exits 1 on violations. It removes the temporary worktree (`git worktree remove --force <dir>`, then `git worktree prune`, as `clm land` does) on every exit path once it has made it, a git failure included; a removal that fails leaves the outcome as it was and adds a warning naming `<dir>` to the report, which the `clm` crate prints on stderr (`clm-sync` itself prints nothing). A failure to make or move the temporary worktree is a git failure, exit 2. A worktree a killed sync leaves behind stays in the temporary directory until the system clears it, and `git worktree prune` drops git's record of it once the directory is deleted; `clm rebase` sweeps nothing, as `clm land` doesn't. The check is the running binary's own, linked in from `clm-check`, never one built from the branch: like `clm land`, which runs `clm check` from its own executable (`land`), it judges the branch by the rules of the `clm` that runs it, which is `main`'s rule wherever the installed `clm` is a release or tracks `main` (`checks-relax-before-use`). So in a repository whose branch changes `clm check` itself, a branch that relaxes a check passes only if nothing in it relies on the relaxation; whatever only the relaxed check accepts, such as deleting a backlog item the old rule requires, goes in a later branch, once the relaxation has landed and the `clm` that runs is rebuilt from it. `clm rebase` never builds or runs another `clm`, and no prompt points it at one, since the branch would then pass sync and fail at `clm land`. The settings are the same: a `clm` that can't read `main`'s `clm.toml` fails the sync, and the fix is to rebuild it from `main`, not to point the session at another binary.

**Autosync** (`autosync`, `autosync-policy`). The spec paths are `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`. The snapshot is built under a temporary `GIT_INDEX_FILE` with `git commit-tree`; when nothing under the spec paths is uncommitted it is `HEAD` itself. It pushes with `+<snapshot>:refs/heads/autosync/<branch>` and `GIT_TERMINAL_PROMPT=0`.

## Requirements

### Requirement: rebase
`clm rebase` SHALL exit 1 without changing anything but `origin`'s remote-tracking refs on `main`, on a detached `HEAD`, or when the working tree has uncommitted work and no rebase started by `clm rebase` is in progress, printing `uncommitted <path>` for each file of that work. Uncommitted work is a staged change; a tracked file modified or deleted under `specs/`, `changes/` or `backlog/`; a tracked file modified or deleted that `origin/main` changed since the merge base; on a branch `impl/*`, any tracked file modified or deleted; and an untracked file that isn't ignored under `specs/` or `changes/`. Any other modified tracked file or untracked file SHALL NOT stop it. Otherwise it SHALL fetch `origin` and rebase the current branch onto `origin/main`, setting aside the modified tracked files it let through and restoring them, unchanged and uncommitted, when the rebase finishes. When the rebase stops on conflicts it SHALL print `conflict <path>` for each conflicted file, leave the rebase in progress, and exit 1. Run again with the conflicts resolved and staged, it SHALL continue the rebase. It SHALL NOT push.

#### Scenario: behind main
- GIVEN a branch with one commit, and an `origin/main` that has gained an unrelated commit
- WHEN `clm rebase` runs
- THEN the branch is its commit on top of `origin/main`, `origin` is unchanged, and it exits 0

#### Scenario: real conflict
- GIVEN a branch and `origin/main` that both changed the same line of `src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints `conflict src/lib.rs`, the rebase is still in progress, and it exits 1

#### Scenario: resume after resolving
- GIVEN a rebase left in progress by `clm rebase`, with the conflict resolved and staged
- WHEN `clm rebase` runs again
- THEN the rebase completes and it exits 0

#### Scenario: dirty tree on a build branch
- GIVEN branch `impl/a` with an uncommitted edit to `src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints `uncommitted src/lib.rs`, exits 1, and the branch and working tree are unchanged

#### Scenario: edit to a file main changed
- GIVEN branch `spec/x` with an uncommitted edit to `src/lib.rs`, and an `origin/main` that has since changed `src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints `uncommitted src/lib.rs`, exits 1, and the branch and working tree are unchanged

#### Scenario: unrelated edit carried through
- GIVEN branch `spec/x` behind `origin/main`, with an uncommitted edit to `.claude/settings.json`, which `origin/main` hasn't changed, and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN it prints no `uncommitted` line, the branch is rebased onto `origin/main`, `.claude/settings.json` still holds the edit, unstaged, no commit holds it, the stash list is as before, and it exits 0

#### Scenario: uncommitted spec edit
- GIVEN branch `spec/x` with an uncommitted edit to the tracked `backlog/q.md`
- WHEN `clm rebase` runs
- THEN it prints `uncommitted backlog/q.md`, exits 1, and the branch and working tree are unchanged

#### Scenario: staged change
- GIVEN branch `spec/x` with a staged edit to `.claude/settings.json`
- WHEN `clm rebase` runs
- THEN it prints `uncommitted .claude/settings.json`, exits 1, and the branch, index and working tree are unchanged

#### Scenario: edit set aside during a conflict
- GIVEN branch `spec/x` with an uncommitted edit to `.claude/settings.json`, and a commit that conflicts with one `origin/main` has gained in `src/lib.rs`
- WHEN `clm rebase` runs, the conflict is resolved and staged, and `clm rebase` runs again
- THEN the first run prints `conflict src/lib.rs` and exits 1, and after the second `.claude/settings.json` holds the edit, uncommitted

#### Scenario: untracked output and worktrees
- GIVEN a branch behind `origin/main` with an untracked `test-results/report.txt` and an untracked `.claude/worktrees/other/README.md`, neither ignored, and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN it prints no `uncommitted` line, the branch is rebased onto `origin/main`, both untracked files are still there unchanged and untracked, and it exits 0

#### Scenario: untracked delta
- GIVEN a branch with an untracked `changes/x/deltas/a.md` and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN it prints `uncommitted changes/x/deltas/a.md`, exits 1, and the branch and working tree are unchanged

### Requirement: reapply
When the branch adds a change whose status is `applied`, adds a new archived change whose status is `done`, or applies a change, `clm rebase` SHALL, after rebasing, set `specs/` to `origin/main`'s `specs/` with the change's deltas and assets applied, leaving its `change.md` where and as it is, and commit the result when it differs from `HEAD`. The branch adds a change when its `changes/<id>/change.md` is at `HEAD` but neither at `origin/main` nor at the merge base of `HEAD` and `origin/main` taken before rebasing, so a change that `main` archived or removed since the branch was cut is never re-applied. It adds a new archived change when `changes/archive/<id>/change.md` is at `HEAD` but neither at `origin/main` nor at that merge base, and `changes/<id>/change.md` is at neither of them either, so a change `main` applied and the branch archives is never re-applied. It applies a change when that change's `changes/<id>/change.md` has status `proposed` or `approved` at `origin/main` and, at `HEAD`, has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, so the deltas of a change `main` holds unapplied are regenerated whether or not the branch adds a change too. Changes SHALL be re-applied live and archived together, in the order `sync.regenerate/regenerated-tree` states: a change whose deltas record bases another of them produces after that change, whatever their IDs, and otherwise in ID order. When the branch adds a change or a new archived change, whatever its status, or applies a change, `clm rebase` SHALL resolve each conflict in `specs/` during the rebase by keeping the side being rebased onto (`origin/main` with the branch's commits replayed so far), removing the path when that side has none, staging the result, and SHALL NOT report it as a conflict, also when the same stop has conflicts outside `specs/`. A `change.md` counts as added whether or not its frontmatter parses. When a delta is still stale against `origin/main` after `stale-delta-merge`, it SHALL write nothing to `specs/`, print the elements left stale, and exit 1. Once it has committed the regenerated `specs/`, and only when it regenerated a change, it SHALL have `clm-regenerate` merge the deltas of the other live `proposed` or `approved` changes the regenerated tree made stale (`sync.regenerate/stale-delta-merge`), print the `merged` lines, and, when a change is still refused after that, print what is left and exit 1, keeping the `Re-apply` and `Merge` commits. It SHALL NOT regenerate `specs/`, or resolve conflicts in `specs/` itself, on any other branch.

#### Scenario: branch applies a change main holds
- GIVEN an `origin/main` holding change `y` with `status: proposed` and a delta adding requirement `a/x`, and a spec branch that adds change `z`, which has no delta, and applies `y`, both `applied`, with an `origin/main` that has since added requirement `a/w` to the same `SPEC.md`
- WHEN `clm rebase` runs
- THEN it prints no `conflict` line, `specs/a/SPEC.md` has both `a/x` and `a/w`, `y`'s status is `applied`, and it exits 0

#### Scenario: change built on another sorts first
- GIVEN an applied spec branch adding changes `x` and `w`, `w` sorting first, where `x`'s delta rewrites the first line of `a`'s Design with base `h1`, `origin/main`'s hash, and `w`'s rewrites its second line with base `h2`, the Design's hash after `x` applies, and an `origin/main` that has since added a requirement to `specs/b/SPEC.md`
- WHEN `clm rebase` runs
- THEN it prints no `apply/stale-base` violation and no `changed design of a` line, `specs/a/SPEC.md`'s Design holds both edits, `specs/b/SPEC.md` holds `origin/main`'s requirement, and its last line is `rebase ok`

#### Scenario: main changed the same file
- GIVEN an applied spec branch whose delta adds requirement `a/x`, and an `origin/main` that has since added requirement `a/y` to the same `SPEC.md`
- WHEN `clm rebase` runs
- THEN `specs/a/SPEC.md` has both `a/x` and `a/y`, `clm check` passes, and it exits 0

#### Scenario: change set back to proposed
- GIVEN a spec branch with a commit that applied change `y`, adding requirement `a/x`, and a later commit that set `specs/` back to the merge base's and `y`'s status to `proposed`, and an `origin/main` that has since added requirement `a/w` to the same `SPEC.md`
- WHEN `clm rebase` runs
- THEN it prints no `conflict` line, the rebase is complete, `specs/a/SPEC.md` is `origin/main`'s, `y`'s status is still `proposed`, and it exits 0

#### Scenario: specs conflict beside a real one
- GIVEN an applied spec branch whose delta adds requirement `a/x`, with a commit that both applied it and changed `src/lib.rs`, and an `origin/main` that has since added requirement `a/y` to the same `SPEC.md` and changed the same line of `src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints `conflict src/lib.rs` and no `conflict` line under `specs/`, no path under `specs/` is unmerged, and it exits 1; once `src/lib.rs` is resolved and staged and `clm rebase` runs again, `specs/a/SPEC.md` has both `a/x` and `a/y` and it exits 0

#### Scenario: specs conflict on a branch that adds no change
- GIVEN a branch that adds no change and edits the line of `specs/a/SPEC.md` holding `a`'s Contract, and an `origin/main` that has since changed the same line
- WHEN `clm rebase` runs
- THEN it prints `conflict specs/a/SPEC.md`, the rebase is still in progress, and it exits 1

#### Scenario: stale delta
- GIVEN an applied spec branch whose delta modifies `a`'s Contract, and an `origin/main` that has since changed the same line of `a`'s Contract
- WHEN `clm rebase` runs
- THEN it exits 1 naming the stale delta, and `specs/` is unchanged by the regenerate step

#### Scenario: main archived a change
- GIVEN a branch cut while `changes/y/` was `applied`, and an `origin/main` that has since moved `changes/y/` to `changes/archive/y/` with `status: done`
- WHEN `clm rebase` runs
- THEN it doesn't re-apply `y`'s deltas, reports no `apply/` violation, and exits 0

#### Scenario: finish branch
- GIVEN a branch that sets component `a`'s `status` to `active` and adds no change
- WHEN `clm rebase` runs after `origin/main` changed `specs/a/SPEC.md` elsewhere
- THEN its status edit is kept

#### Scenario: change archived on its spec branch
- GIVEN a spec branch `spec/x` with a commit that applied change `x`, adding requirement `a/x`, and a later commit that set `x`'s status to `done` and moved `changes/x/` to `changes/archive/x/`, and an `origin/main` that has since added requirement `a/y` to the same `SPEC.md`
- WHEN `clm rebase` runs
- THEN it prints no `conflict` line, `specs/a/SPEC.md` has both `a/x` and `a/y`, `changes/archive/x/change.md` has `status: done`, there is no `changes/x/`, and it exits 0

#### Scenario: archived change merged
- GIVEN a spec branch `spec/x` whose change `x` is archived on the branch as above, with a delta `changes/archive/x/deltas/a.md` that rewrites the first line of `a`'s Contract, and an `origin/main` that has since changed only the last line of `a`'s Contract
- WHEN `clm rebase` runs
- THEN it prints `merged contract of a into changes/archive/x/deltas/a.md`, `specs/a/SPEC.md`'s Contract holds both edits, and it exits 0

#### Scenario: branch archives a change main applied
- GIVEN an `origin/main` with an `applied` change `y` whose delta modifies `a`'s Contract (so its recorded base is no longer `a`'s Contract on `main`), a branch `impl/b` that sets `y` to `done` and moves `changes/y/` to `changes/archive/y/`, and an `origin/main` that has since added a requirement to `specs/a/SPEC.md`
- WHEN `clm rebase` runs
- THEN it prints no `main changed` line and no `apply/` violation, makes no `Re-apply` or `Merge main into` commit, `specs/a/SPEC.md` is `origin/main`'s, and it exits 0


#### Scenario: another change's delta merged
- GIVEN an `origin/main` with a `proposed` change `o` whose delta modifies `a`'s Design with base `origin/main`'s hash, rewriting its last line, and an applied spec branch adding `x`, whose delta rewrites the first line of `a`'s Design
- WHEN `clm rebase` runs
- THEN it prints `merged design of a into changes/o/deltas/a.md`, `HEAD` is a commit `Merge x into o` whose parent is `Re-apply x`, `clm check` passes, and its last line is `rebase ok`

#### Scenario: another change's delta left stale
- GIVEN the same, where `o`'s delta rewrites the same first line
- WHEN `clm rebase` runs
- THEN it prints the stale-base violation naming `changes/o/deltas/a.md` and `x changed design of a since <base>`, `HEAD` is the commit `Re-apply x`, `o`'s delta is unchanged, and its last line is `rebase incomplete`

#### Scenario: branch that regenerates nothing
- GIVEN a branch adding only a `proposed` change `x`, and an `origin/main` with a `proposed` change `o` whose delta modifies `a`'s Design with a base that isn't `a`'s Design hash there
- WHEN `clm rebase` runs
- THEN it prints no `merged` line, `o`'s delta is unchanged, and it prints `clm check`'s stale-base violation for it and exits 1

#### Needs
- sync.regenerate/regenerated-tree

### Requirement: judged-by-check
After a rebase that completes and any regeneration, `clm rebase` SHALL load the settings on every branch, printing the error and exiting 1 when they don't load, and SHALL run every `clm check` rule on the result, print any violations, and exit 1 if there are any, 0 otherwise. It SHALL load the settings and run the check on the branch's `HEAD` commit, checked out in a temporary worktree outside the repository, never on the working tree, so no uncommitted edit and no untracked or ignored file changes the outcome, and SHALL remove that worktree before it exits. A file that a settings error or a violation names SHALL be printed by its path in the repository, never by its path in the temporary worktree.

#### Scenario: merged into a broken state
- GIVEN a branch whose test cites `req:a/x`, and an `origin/main` that removed requirement `a/x`
- WHEN `clm rebase` runs
- THEN the rebase completes, it prints the dangling marker, and it exits 1

#### Scenario: settings can't be loaded
- GIVEN branch `spec/x`, and an `origin/main` whose `clm.toml` has a key the running `clm` doesn't know
- WHEN `clm rebase` runs
- THEN the branch is rebased, it prints a line naming `clm.toml` by its path in the repository (no part of it under the temporary directory) and the error, its last line is `rebase incomplete`, and it exits 1

#### Scenario: uncommitted settings edit
- GIVEN branch `spec/x` behind `origin/main`, with an uncommitted edit to `clm.toml` that adds a key the running `clm` doesn't know, which `origin/main` hasn't changed, and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN the branch is rebased, it prints no line naming `clm.toml`, its last line is `rebase ok`, it exits 0, and `clm.toml` still holds the edit, uncommitted

#### Scenario: uncommitted marker
- GIVEN branch `spec/x` whose committed tree passes `clm check`, with an uncommitted edit to the tracked `src/lib.rs` that adds a test citing `req:a/missing`, a requirement no component has, which `origin/main` hasn't changed
- WHEN `clm rebase` runs
- THEN it prints no `check/dangling-markers` violation, its last line is `rebase ok`, and it exits 0

#### Scenario: committed fault, uncommitted fix
- GIVEN branch `spec/x` whose committed `src/lib.rs` cites `req:a/missing`, a requirement no component has, with an uncommitted edit that removes the citation, and an `origin/main` that hasn't changed `src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints a `check/dangling-markers` violation whose path is `src/lib.rs`, its last line is `rebase incomplete`, it exits 1, and `src/lib.rs` still holds the edit, uncommitted

#### Scenario: untracked test file
- GIVEN branch `spec/x` whose committed tree passes `clm check`, with an untracked, unignored `tests/new.rs` citing `req:a/missing`, and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN it prints no violation, its last line is `rebase ok`, it exits 0, and `tests/new.rs` is still there, untracked

#### Scenario: no temporary worktree left
- GIVEN `TMPDIR` set to an empty directory, and branch `spec/x` whose committed tree has a violation
- WHEN `clm rebase` runs
- THEN it prints the violation and exits 1, `TMPDIR` is empty again, and `git worktree list` names only the repository's own worktrees

#### Scenario: a clone the environment names
- GIVEN branch `spec/x` up to date with `origin/main`, whose committed tree passes `clm check`, with an untracked, unignored `tests/new.rs` citing `req:a/missing`, in a work tree `w` whose git directory `g` is elsewhere, with `GIT_DIR=g` and `GIT_WORK_TREE=w` set in the environment `clm rebase` runs in
- WHEN `clm rebase` runs
- THEN it prints no violation, its last line is `rebase ok`, it exits 0, and, run with those variables, `git rev-parse HEAD` names the commit it named before and `git status --porcelain` prints only `?? tests/new.rs`

#### Serves
- capability/catch-drift
- architecture/no-silent-drift

#### Needs
- vcs/git-follows-its-clone

### Requirement: autosync
On a branch named `spec/*`, `clm autosync` SHALL push to `refs/heads/autosync/<branch>` on `origin` a commit whose parent is `HEAD` and whose tree is `HEAD`'s with the spec paths taken from the working tree, untracked files included, leaving the working tree, the index and the branch unchanged. It SHALL include no file outside the spec paths. It SHALL push nothing on any other branch, or when the snapshot's tree equals that of `origin/<branch>` or `origin/autosync/<branch>`.

#### Scenario: uncommitted delta
- GIVEN branch `spec/x` with an uncommitted new file `changes/x/deltas/a.md` and an untracked `notes.txt` at the root
- WHEN `clm autosync` runs
- THEN `origin/autosync/spec/x` holds `changes/x/deltas/a.md` and not `notes.txt`, and `git status` is as before

#### Scenario: nothing new
- GIVEN branch `spec/x` just after a `clm autosync` with no edits since
- WHEN `clm autosync` runs
- THEN it pushes nothing

#### Scenario: other branch
- GIVEN branch `impl/x/a` with uncommitted spec edits
- WHEN `clm autosync` runs
- THEN it pushes nothing

#### Serves
- capability/work-concurrently

### Requirement: outcome-line
`clm rebase` SHALL end its stdout with exactly one line stating its outcome: `rebase ok` when it exits 0, `rebase incomplete` when it exits 1, and `rebase failed` when it exits 2. No earlier line SHALL be one of those three.

#### Scenario: up to date
- GIVEN a branch with one commit on top of `origin/main` that passes `clm check`
- WHEN `clm rebase` runs
- THEN its last line of stdout is `rebase ok` and it exits 0

#### Scenario: conflict
- GIVEN a branch whose commit conflicts with a commit `origin/main` has gained
- WHEN `clm rebase` runs
- THEN it prints `conflict <path>`, its last line of stdout is `rebase incomplete`, and it exits 1

#### Scenario: git fails
- GIVEN a branch whose `origin` remote points at a path that doesn't exist
- WHEN `clm rebase` runs
- THEN its last line of stdout is `rebase failed` and it exits 2

#### Serves
- capability/work-concurrently
- architecture/deterministic-enforcement

### Requirement: spec-changed
On a branch `impl/<component>`, `clm rebase` SHALL, after rebasing and regenerating, print `spec changed <component>` when the component's bundle differs between the branch's merge base before the rebase and `origin/main`, followed by `changed <path>` for each file the bundle is read from that differs (`ARCHITECTURE.md`, `CAPABILITIES.md` when the block of a capability in the component's bundle differs, one the component serves or, for the component that builds the capability tests, any (`context/bundle-contents`), the bundle's components' `SPEC.md`s and the component's assets), `requirement <id> <old> -> <new>` for each of the component's own requirements whose version changed, appeared or went, and a line naming `clm pending <component>` and the `git diff` to read, and exit 1 after running `clm check`. It SHALL print nothing of the kind, and exit by the other rules, when the bundle is unchanged, on any other branch, or on the next run with nothing new.

#### Scenario: requirement changed on main
- GIVEN branch `impl/a`, and an `origin/main` that has since modified requirement `a/r` from version `11111111` to `22222222`
- WHEN `clm rebase` runs
- THEN the branch is rebased, it prints `spec changed a`, `changed specs/a/SPEC.md`, `requirement a/r 11111111 -> 22222222` and a line naming `clm pending a`, and it exits 1

#### Scenario: quiet on the next run
- GIVEN the same branch after that run, with nothing else changed
- WHEN `clm rebase` runs again
- THEN it prints no `spec changed` line and exits 0

#### Scenario: dependency contract changed
- GIVEN branch `impl/a`, where `a` depends on `b`, and an `origin/main` that has since changed `b`'s Contract
- WHEN `clm rebase` runs
- THEN it prints `spec changed a` and `changed specs/b/SPEC.md`, no `requirement` line, and exits 1

#### Scenario: asset changed
- GIVEN branch `impl/a` and an `origin/main` that has since edited `specs/a/assets/m.html`
- WHEN `clm rebase` runs
- THEN it prints `spec changed a` and `changed specs/a/assets/m.html`, and exits 1

#### Scenario: served capability changed
- GIVEN branch `impl/a`, where `a` serves `capability/c`, and an `origin/main` that has since reworded `c` in `CAPABILITIES.md`
- WHEN `clm rebase` runs
- THEN it prints `spec changed a` and `changed CAPABILITIES.md`, and exits 1

#### Scenario: capability node
- GIVEN branch `impl/capability`, where `capability` is the graph's capability component, and an `origin/main` that has since reworded capability `d`
- WHEN `clm rebase` runs
- THEN it prints `spec changed capability` and `changed CAPABILITIES.md`, and exits 1

#### Scenario: unserved capability changed
- GIVEN branch `impl/a`, which serves no capability, and an `origin/main` that has since reworded a capability
- WHEN `clm rebase` runs
- THEN it prints no `spec changed` line and exits 0

#### Scenario: unserved capability changed with the node
- GIVEN branch `impl/a`, where `a` serves `capability/c`, and an `origin/main` that has since changed `a`'s Contract and reworded capability `d`
- WHEN `clm rebase` runs
- THEN it prints `spec changed a` and `changed specs/a/SPEC.md`, and no `changed CAPABILITIES.md`

#### Scenario: unrelated change on main
- GIVEN branch `impl/a` and an `origin/main` that has since changed only component `b`, which isn't in `a`'s bundle
- WHEN `clm rebase` runs
- THEN it prints no `spec changed` line and exits 0

#### Scenario: spec branch
- GIVEN branch `spec/x` and an `origin/main` that changed `specs/a/SPEC.md`
- WHEN `clm rebase` runs
- THEN it prints no `spec changed` line

#### Serves
- capability/build-from-spec
- architecture/hash-based-staleness

#### Needs
- context/bundle-contents

### Requirement: archive-complete
On a branch `impl/*`, `clm rebase` SHALL, after rebasing and regenerating, set to `done` and move to `changes/archive/` each `applied` or `implementing` change at `HEAD` whose added and modified requirements are all current at `HEAD` and whose removed requirements no marker at `HEAD` cites, delete each `backlog/<slug>.md` its `resolves` still names, commit the result as `Archive <id>`, and print `archived <id>`. It SHALL also set `status: active` in the `SPEC.md` of each `unbuilt` component with no pending requirement at `HEAD`, committing it as `Activate <component>` when no archive commit carries it. It SHALL NOT archive a change on any other branch, one with a pending requirement, or one that hasn't applied.

#### Scenario: branch completes a change
- GIVEN branch `impl/a` whose tests cite the last requirement of `applied` change `x` at its current version, where `x` has `resolves: [q]` and `backlog/q.md` exists
- WHEN `clm rebase` runs
- THEN it prints `archived x`, `HEAD` has `changes/archive/x/change.md` with `status: done`, no `changes/x/` and no `backlog/q.md`, and exits 0

#### Scenario: obsolete marker holds it
- GIVEN the same branch, where a test still cites a requirement `x` removed
- WHEN `clm rebase` runs
- THEN it prints no `archived` line and `changes/x/` is still there

#### Scenario: completed by another branch's merge
- GIVEN branch `impl/b`, and an `origin/main` in which change `x` became complete through other PRs but is still under `changes/`
- WHEN `clm rebase` runs
- THEN it prints `archived x` and `HEAD` has `changes/archive/x/`

#### Scenario: unapplied change left alone
- GIVEN branch `impl/a` and a `proposed` change `y` whose delta modifies `a/p`, cited at its current version
- WHEN `clm rebase` runs
- THEN it prints no `archived` line and `changes/y/` is still there

#### Scenario: node activated
- GIVEN branch `impl/a`, where component `a` is `unbuilt` and its tests cite every requirement of `a` at its current version
- WHEN `clm rebase` runs
- THEN `specs/a/SPEC.md` at `HEAD` says `status: active`, committed as `Activate a`

#### Scenario: spec branch
- GIVEN branch `spec/y` and a complete change `x` outside the archive
- WHEN `clm rebase` runs
- THEN it prints no `archived` line

#### Serves
- capability/wrap-up-session
- architecture/spec-is-current

### Requirement: git-through-layer
`clm-sync` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `gh` or its repo queries (`fetch`, `current_branch`, `extract_tree`, which take no index lock or use a temporary index), or through `clm-regenerate`'s `git` and `run`, which retry a held `index.lock` (`git-plumbing-layer`, `sync.regenerate/retry-index-lock`), and its own wrapper is gone (`no-dead-code`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-sync` reads `crates/clm-sync/Cargo.toml` and every file under `crates/clm-sync/src`
- THEN `clm-git` and `clm-regenerate` are dependencies, and no file holds `Command::new("git")`, `Command::new("gh")`, `clm_git::git(` or `clm_git::run(`, nor imports `git` or `run` from `clm_git`

#### Scenario: lock met at the commit
- GIVEN an applied spec branch `spec/x` adding `x`, and an `origin/main` that has since added a requirement to the same `SPEC.md`, so the regenerate commits `Re-apply x`, where the worktree's `index.lock` is held from the end of the rebase and released half a second later
- WHEN `clm rebase` runs
- THEN `HEAD` is a commit `Re-apply x`, nothing is left staged, its last line is `rebase ok`, and it exits 0

#### Serves
- architecture/reuse-first

### Requirement: shared-plumbing
`clm-sync`'s `Error` SHALL be `clm-git`'s `Failure`, re-exported under the name `Error`, and the crate SHALL take the remote's name from `clm-git`'s `REMOTE`: no file under `crates/clm-sync/src` SHALL hold a string literal that begins `"origin` (`shared-plumbing-in-git`). `clm rebase` and `clm autosync` SHALL keep their exit codes, messages and outcome line. `crates/clm/src/main.rs` SHALL map their `Error` through one helper, `failed(cmd, &Failure) -> ExitCode`, which prints `clm <cmd>: <message>` on stderr and returns the `Failure`'s exit code, in place of `synced()` and `clm rebase`'s own two arms; `clm rebase` SHALL choose its outcome line from the exit code before calling it.

#### Scenario: the shared error
- GIVEN the crate
- WHEN a test assigns `clm_sync::Error::Refused("x".into())` to a binding of type `clm_git::Failure`
- THEN it compiles, and the value's exit code is 1

#### Scenario: no copy of the remote's name
- GIVEN the workspace
- WHEN a test in `clm-sync` reads every file under `crates/clm-sync/src`
- THEN no file holds `"origin`

#### Scenario: one mapping in the dispatcher
- GIVEN the workspace
- WHEN a test reads `crates/clm/src/main.rs`
- THEN it defines `fn failed(` and holds no `fn synced(`, `clm_sync::Error::Refused` or `clm_sync::Error::Git`

#### Scenario: refused sync keeps its output
- GIVEN a checkout on `main`
- WHEN `clm rebase` runs
- THEN it prints `clm rebase: ` and the reason on stderr, its last line of stdout is `rebase incomplete`, and it exits 1

#### Serves
- architecture/reuse-first
- architecture/no-dead-code

#### Needs
- vcs/remote-name
- vcs/failure

### Requirement: extract-through-layer
`clm-sync` SHALL read a revision's files from the temporary directories `clm-git`'s `extract_paths(root, <rev>, <paths>)` returns, with the same top-relative paths it reads today: `specs/`, `ARCHITECTURE.md` and `CAPABILITIES.md` at a revision when it compares a component's spec there (`spec-changed`), and, when it regenerates (`regenerate-applied-specs`), `specs/` at `origin/main` and `changes/`, `backlog/`, `docs/decisions/`, `ARCHITECTURE.md` and `CAPABILITIES.md` at `HEAD`, each path only when the revision has it (an empty `paths` gives an empty directory). Regenerating SHALL load one tree holding them all: it SHALL extract `specs/` at `origin/main` into one directory, which stays the temporary tree the rest of regeneration reads and writes, and the present ones of those paths at `HEAD` into a second, in one call, and move each top-level entry of the second into the first (`fs::rename`; both are made in the system's temporary directory), so `clm_model::load` reads them together as it does today. Extracting SHALL run no hook, since `extract_paths` runs none, where `git checkout <rev> -- <paths>` runs `post-checkout`. Its own `extract` is gone (`no-dead-code`).

#### Scenario: no post-checkout hook when regenerating
- GIVEN a repository whose `post-checkout` hook appends the directory it runs in to a log file outside the repository, and a branch `spec/x` that adds an `applied` change `x` and is behind `origin/main`
- WHEN `clm rebase` runs
- THEN it regenerates `specs/` as before, its last line is `rebase ok`, the log isn't empty (the rebase's own checkouts), and every line of it names the repository's working tree or the `sync-*` worktree sync judges in (`sync-judges-head`), none another directory

#### Scenario: no post-checkout hook when comparing a node's spec
- GIVEN the same hook, and a branch `impl/a` that is behind an `origin/main` whose `specs/a/SPEC.md` changed a requirement since the branch was cut
- WHEN `clm rebase` runs
- THEN it prints `spec changed a`, the log isn't empty, and every line of it names the repository's working tree or a `sync-*` worktree, none another directory

#### Scenario: no extract of its own
- GIVEN the workspace
- WHEN a test under `crates/clm-sync/tests/` (outside `src/`, so its own text isn't scanned) reads every file under `crates/clm-sync/src`
- THEN no file holds `fn extract(` or `--ignore-skip-worktree-bits`

#### Serves
- architecture/reuse-first

### Requirement: file-reconsider-item
On a branch `impl/<component>`, after archiving and filing proposals, `clm rebase` SHALL, for each requirement of `<component>` whose reconsider score (`check.health/reconsider-score`) reaches the `reconsider` setting and for which no `backlog/reconsider-<component>-<slug>.md` exists in the working tree, tracked or not, write the item `check.health`'s `reconsider` returns (`title: "Reconsider <component>/<slug>"`, `kind: question`, `components: [<component>]`, a body with the score, its evidence and the three ways out), commit it alone as `File reconsider-<component>-<slug>`, and print `filed backlog/reconsider-<component>-<slug>.md`. It SHALL NOT file an item on any other branch, for a requirement of another component, for a requirement below the threshold, or when the item exists, and SHALL NOT edit or delete an existing item. When the settings couldn't be loaded it SHALL file nothing, `judged-by-check` reporting why. The score SHALL NOT change its outcome line or exit code.

#### Scenario: requirement at the threshold
- GIVEN branch `impl/a`, the default `reconsider` of 3, and requirement `a/x` with three committed `workaround` items naming it at its current version
- WHEN `clm rebase` runs
- THEN it prints `filed backlog/reconsider-a-x.md`, `HEAD` is a commit `File reconsider-a-x` adding `backlog/reconsider-a-x.md` with `kind: question` and `components: [a]` whose body holds `score 3` and the three items' paths, and it exits 0

#### Scenario: item already filed
- GIVEN the same branch, with `backlog/reconsider-a-x.md` at `HEAD`
- WHEN `clm rebase` runs
- THEN it prints no `filed backlog/reconsider-a-x.md` line and the item is unchanged

#### Scenario: below the threshold
- GIVEN branch `impl/a` and requirement `a/x` with two `workaround` items
- WHEN `clm rebase` runs
- THEN it prints no `filed` line for `reconsider-a-x`

#### Scenario: another node's requirement
- GIVEN branch `impl/a`, where `b/y` has three `workaround` items
- WHEN `clm rebase` runs
- THEN it prints no `filed` line and `HEAD` has no `backlog/reconsider-b-y.md`

#### Scenario: spec branch
- GIVEN branch `spec/y` and requirement `a/x` with three `workaround` items
- WHEN `clm rebase` runs
- THEN it prints no `filed` line

#### Serves
- capability/know-health

#### Needs
- check.health/reconsider-score
- config/health-setting
- model/component-keys

### Requirement: file-proposal-items
On any branch `impl/*`, after archiving, `clm rebase` SHALL, for each open proposal in the ledger fold, a `proposed` record no `withdrawn` record names, in slug order, whose slug no backlog item at `HEAD` has and no change at `HEAD`, archived or not, names in `resolves`, write `backlog/<slug>.md` with the record's `title`, `kind: idea`, under the key `components` (`model/component-keys`) those of the components it lists (`/nodes`) the graph at `HEAD` holds, and a body of the record's `body` followed by the line `Proposed in the web app by <writer> at <at> (record <id>).`, built from the record alone; commit it alone as `File <slug>`; and print `filed backlog/<slug>.md`. It SHALL file nothing on any other branch, for a withdrawn proposal, or for one whose item exists or whose slug a change resolves, and SHALL NOT edit or delete an existing item. Filing SHALL NOT change its outcome line or exit code.

#### Scenario: filed
- GIVEN branch `impl/a`, and `origin/clm/ledger/alice/web` holding `proposed` `p1` with slug `export-csv`, title "Export CSV", components `[a, gone]` and body "Let me export.", where component `gone` isn't in the graph
- WHEN `clm rebase` runs
- THEN it prints `filed backlog/export-csv.md`, `HEAD` is a commit `File export-csv` adding `backlog/export-csv.md` with `title: "Export CSV"`, `kind: idea`, `components: [a]` and a body holding "Let me export." and the line `Proposed in the web app by alice/web at <the record's at> (record p1).`, `clm check` accepts it, and it exits 0

#### Scenario: same bytes on two branches
- GIVEN the same proposal, filed by `clm rebase` on `impl/a` and, in a second clone, on `impl/b`
- WHEN the two files are compared
- THEN they are identical

#### Scenario: withdrawn, filed or resolved
- GIVEN branch `impl/a`, and a fold holding `proposed` `p1` with slug `s1` named by a `withdrawn` record, `proposed` `p2` with slug `s2` where `backlog/s2.md` exists at `HEAD`, and `proposed` `p3` with slug `s3` named in the `resolves` of `changes/archive/20260101-x/change.md`
- WHEN `clm rebase` runs
- THEN it prints no `filed` line and `HEAD` is unchanged

#### Scenario: spec branch
- GIVEN branch `spec/y` and an open proposal with slug `s4`
- WHEN `clm rebase` runs
- THEN it prints no `filed` line and `HEAD` has no `backlog/s4.md`

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/proposal-records
- model/component-keys

### Requirement: regenerate-renames
On a branch that adds exactly one change, live or new archived, whose `change.md` at `HEAD` sets `renames`, `clm rebase` SHALL, after step 1's refusals and the fetch and in place of the rebase and the re-apply, first ask `rename/replay` whether the branch replays against its merge base with `origin/main`. When it doesn't, it SHALL print `not a rename branch <path>` for each path whose content differs from the replay, change nothing, and exit 1. When it does, it SHALL make the branch one commit on `origin/main`, `Rename <change-id>`, holding what `clm rename <change-id>` produces from `origin/main`'s tree with that `change.md` placed live with status `proposed`, plus the files under `backlog/` that the branch added; when `clm rename` refuses, it SHALL print the refusal, leave the branch as it was, and exit 1. It then judges the new `HEAD` as for any branch (`judged-by-check`). It SHALL NOT push. It SHALL produce each such tree, the one it compares to name the differing paths included, in a temporary directory that it makes no git repository and in which it runs no git command of its own, as `rename/replay` produces the tree it checks, so `clm rename` reads it as the replay does, and a `GIT_DIR` or `GIT_WORK_TREE` the environment sets, naming the clone, never makes the clone's git directory reinitialized or its index staged in place of that directory.

#### Scenario: main moved under a rename
- GIVEN a branch holding the committed result of `clm rename r`, renaming `a/x` to `a/y`, and an `origin/main` that has since added a test citing `a/x` at its current version
- WHEN `clm rebase` runs
- THEN the branch is one commit on `origin/main`, the new test cites `a/y`, its last line is `rebase ok`, and it exits 0

#### Scenario: a clone the environment names
- GIVEN the branch and `origin/main` of *main moved under a rename*, in a work tree `w` whose git directory `g` is elsewhere, with `GIT_DIR=g` and `GIT_WORK_TREE=w` set in the environment `clm rebase` runs in
- WHEN `clm rebase` runs
- THEN it ends as in *main moved under a rename*, `git status --porcelain`, run with those variables, prints nothing, and `g/config` is byte for byte as it was

#### Scenario: a hand edit on a rename branch
- GIVEN a branch holding the committed result of `clm rename r` and a commit editing `crates/a/src/lib.rs`
- WHEN `clm rebase` runs
- THEN it prints `not a rename branch crates/a/src/lib.rs`, the branch is unchanged, and it exits 1

#### Needs
- rename/replay
- rename/finish

### Requirement: warnings-name-the-command
The warnings the rebase prints on stderr that name the command (those `file-proposal-items` prints when the ledger can't be read, for each ledger warning and for each proposal it skips, and the error for a temporary worktree it couldn't remove) SHALL name the command as it was run: `clm rebase: <warning>` when run as `clm rebase`, and, while the old subcommand `sync` still runs (`clm/renamed-commands`), the old name followed by `: ` and the same warning, exactly as before. `clm-sync` SHALL hold no string literal that names the command: the warnings its report returns carry no prefix, and the root, which knows the name it was run by, prints each with that prefix (`clm/rebase-warnings-prefixed`).

#### Scenario: a skipped proposal's warning
- GIVEN branch `impl/a` and the ledger fold holding an open proposal whose slug is `Not Kebab`
- WHEN `clm_sync::sync` runs
- THEN its report's warnings hold one starting `skipped proposal` and naming that proposal, and none starts with `clm`

#### Scenario: no command name in the crate
- GIVEN the workspace
- WHEN a test reads every file under `crates/clm-sync/src`
- THEN no string literal holds `clm` followed by a space and `rebase` or the old subcommand `sync`

#### Serves
- architecture/plain-terms

### Requirement: agents-md-spec-path
`clm autosync` SHALL count `AGENTS.md` at the top of the repository among the spec paths it snapshots, as it counts `CLAUDE.md`.

#### Scenario: uncommitted instructions
- GIVEN a spec branch's checkout with an uncommitted edit to `AGENTS.md` alone
- WHEN `clm autosync` runs
- THEN the snapshot it pushes holds that edit

#### Serves
- capability/work-from-any-agent-harness
