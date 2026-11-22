---
component: sync
base: { frontmatter: "4b43194c141b4136", contract: "4567491261391e06", design: "697b859e4950b113" }
---

## MODIFIED Frontmatter
`gate` joins `depends_on`: which branches are builds' and what component one has are read through `gate`'s call (`build-branches`). `gate` depends on nothing that depends on `sync`, and `clm-sync` already links `clm-gate` through `clm-check`'s default features.

```yaml
depends_on: [model, apply, check, check.coverage, check.health, context, gate, ledger, vcs, sync.regenerate, rename]
```

## MODIFIED Contract
`clm rebase` brings the current branch up to date with `origin/main`: it fetches, rebases, regenerates derived files and runs `clm check`. It exits 0 when the branch is up to date and passes, 1 when something is left for the caller to resolve (with what and where), and 2 when git fails. Running it again after resolving carries on. It never pushes. For every run whose arguments it parses, its last line of stdout states the outcome: `rebase ok`, `rebase incomplete` or `rebase failed`, for exit codes 0, 1 and 2, so a caller that can't read the exit code (an unattended session, `builds-in-subagents`) still knows it; `--help` and arguments it can't parse print no outcome line (`outcome-line`). A refusal, a git failure and each warning go to stderr as `clm rebase: <text>` (`rebase`, `warnings-name-the-command`).

On a branch that adds an applied change, or a change it archived itself after applying it (`archive-in-spec-pr`), it rebases a stale delta itself where it can (`sync.regenerate/stale-delta-merge`): each section of the delta whose edits don't overlap what moved the section since the delta's base, `main`'s edits or an earlier change's on the same branch, is merged three-way, written into the delta with its new base, and committed as `Merge <movers> into <change>`, naming what moved it, printing `merged <element> of <component> into <delta path>`. A section whose edits overlap, one the delta removes or that is absent from the tree, and one whose base text git doesn't have are left for the caller, with what moved in each and who moved it (`sync.regenerate/stale-delta-diff`). The delta of another live `proposed` or `approved` change that the branch's apply made stale, on `main` or on the branch, is merged the same way against the branch's applied text and committed on its own; an element an earlier merge or a hand resolution on the branch set to a base only the branch's tree had is taken back once nothing moves it, whether or not the branch still regenerates a change (`reapply`). A `git` command that fails on a transient `index.lock` is run again before the run fails (`sync.regenerate/retry-index-lock`).

On a branch that adds a change recording `renames` (`rename`), it regenerates the branch instead of rebasing it (`regenerate-renames`): the branch is one commit on `origin/main` holding what `clm rename` produces there, so a moved `main` never leaves a rename to merge by hand (`regenerate-dont-merge`).

On a build's branch, one the gate reads as `code`, `light` or `mixed`, or one named `impl/*` that the gate says needs no review (`build-branches`), it also archives complete changes (`archive-complete`, `archive-on-sync`) and files proposal items (`file-proposal-items`, `web-proposals-as-ledger-records`); for the branch's component, `<component>` of `impl/<component>` or else the one the gate reads from the branch's files (always for a `light` branch, and for a `code` branch where the root sets `wiring`), it reports a changed component spec (`spec-changed`) and files reconsider items (`file-reconsider-item`, `health-advises`). It files no item for a component over its size budget: the component shows as oversized in `clm status`, `clm size` and `clm health` (`oversized-shown-not-filed`).

`clm autosync` pushes a snapshot of the spec work on the current `spec/*` branch to `origin` as `autosync/<branch>` (`autosync`). It prints nothing and exits 0 whether or not it pushed; when git fails it prints `clm autosync: <why>` on stderr and exits 2.

`clm-sync` offers (`commands-in-crate`, `commands-live-in-their-crates`):
- `rebase_command(args, root, env, out, err, settings)` and `autosync_command(args, root, env, out, err)`, each a command as a call: it takes the command's arguments as the command line gives them, the repository root its caller found, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `settings` loads the `health` settings from a CLM root it is given, or says why they can't be, as a line to print (`judged-by-check`). `crates/clm` routes the command to it and adds nothing to what it writes.
- `RebaseArgs` and `AutosyncArgs`, the two commands' arguments (`--root`) as a parser the root's command line flattens in.
- `autosync(root)`, which does what `clm autosync` does for the CLM root it is given, in this process's environment, and returns the failure it would print; plugin.hooks's `Stop` hook calls it (`autosync`).

## MODIFIED Design
Code lives in crate `clm-sync` and uses the `git` CLI, not a git library. The regenerate's tree, its stages, the stale-delta report and both merges, and the `index.lock` retry every `git` command of the two crates goes through, are the child `sync.regenerate`'s, in crate `clm-regenerate`, which `clm-sync` calls (`sync-regenerate-split`); `clm-regenerate` reaches no `clm-health`, so `clm land` can call it (`clm/health-isolated`). Every `git` process goes through `clm-git` (`git-plumbing-layer`), and every one that can take the index lock (each rebase, `add`, `rm`, `checkout`, `commit`, `update-ref` and `worktree` command) through `clm-regenerate`'s retried `git` and `run`, so each waits out a held `index.lock` as `rebase` states; `clm-git`'s own calls (`fetch`, `current_branch`, and `extract_tree`, which checks out into its own extraction) take no lock on the repository's index. The remote's name is `clm-git`'s `REMOTE` (`shared-plumbing-in-git`).

**Build branches** (`build-branches`). Whether a branch is a build's, and its component, are asked once per run of `gate`'s `evaluate_at`, in-process, with `origin/main` as fetched and `HEAD` before the rebase, reading its kind, whether it needs review and its component (`gate/evaluate-at`, `gate/own-files`); the marker keeps the answer for a resumed run, which skips step 1. With no merge base the gate has nothing to judge and isn't asked: the branch is a build's by its name alone. The name `impl/*` is a claim, read only beside the gate's answer: a fresh claim and an `impl/archive/<id>` branch read `spec` with no review and are builds' by the name, while a branch that needs review is never a build's, whatever its name, since archiving or filing on it would change the diff a human approved; `clm land` judges whether the files agree with `impl/<component>`. A build's branch is so for the whole run, though the rebase may drop a commit that `main` already holds. Steps 1, 5, 6 and 7 read the answer. The gate reads a `code` branch's component only where the root sets `wiring`, and a `light` branch's always: in a project that sets none, a `code` branch not named `impl/<component>` has none, so on it `clm rebase` archives and files proposal items but reports no changed spec and files no reconsider item. A branch that applies a change beside its code reads `mixed` while the change is live and `light` once the branch archives it, and is a build's either way: it regenerates its change (step 4), reports what `main` moved under its component once it has one (step 5), and archives its change once its requirements are current (step 6), after which it reads `light` and step 4 regenerates the change as a new archived change.

**`clm rebase`.**
1. Unless a rebase that `clm rebase` started is in progress, it refuses on `main`, on a detached `HEAD`, and while a rebase or a `git am` it didn't start is in progress (git's `rebase-merge` or `rebase-apply` directory present, and no `clm-rebase` file in git's `rebase-merge` directory: when its own rebase stops, `clm rebase` writes that file there, and git removes the directory, and the file with it, when that rebase finishes or is aborted, however it is finished, so a rebase begun later by hand never holds it; what is compared is that file's presence alone, the marker under `.git/clm/` keeping only what a resume needs; `rebase-apply/applying` marks a `git am`), naming what to do (`rebase`). Otherwise it fetches `origin` and then refuses with uncommitted work (`rebase`, `sync-ignores-untracked`, `sync-carries-unrelated-edits`): an edit its own steps could lose or commit, or that the steps judging `HEAD` would read in its place. Why each path counts:
   - a staged change: `git rebase --autostash` would hand it back unstaged, and the `Re-apply`, `Archive` and `Activate` commits commit the whole index;
   - a tracked file modified or deleted under `specs/`, `changes/` or `backlog/`, the trees steps 4, 6 and 7 write;
   - a tracked file modified or deleted that `origin/main` changed since the merge base of `HEAD` and the freshly fetched `origin/main` (`git diff --name-only --no-renames <merge base> origin/main`, so a file main renamed counts under its old path too): the rebase writes it, so restoring the edit afterwards could conflict; with no merge base, every modified or deleted tracked file;
   - on a build's branch, any tracked file modified or deleted, since steps 6 and 7 judge `HEAD` by reading the working tree (its markers);
   - an untracked file that isn't ignored under `specs/` or `changes/`, the two trees steps 4 and 6 stage whole with `git add -A`.

   The tracked paths come from `git status --porcelain -z --no-renames --untracked-files=no` (any `X` other than a space is a staged change, unmerged entries included; any `Y` other than a space is a working-tree change, intent-to-add included), so a renamed file counts under both its old and its new path. Every branch refuses uncommitted work only after the fetch, so every branch takes one path. A refused run writes nothing under `.git/clm/`. The untracked files come from `git ls-files --others --exclude-standard -- specs changes`, which honours every ignore source and names files rather than directories. `uncommitted <path>` lines go on stdout before the outcome line, as `conflict <path>` does. Any other edit to a tracked file (a human's change to `.claude/settings.json` on a `spec/*` branch) is set aside and restored by the rebase (step 2), nothing below stages it, and step 8 judges `HEAD`'s commit, so it never counts toward the outcome. Any other untracked file (test output, another session's `.claude/worktrees/`) is never staged, removed or read; should the rebase need to write over one, git refuses with the path, which `rebase` states.
2. It runs `git rebase --autostash origin/main` (or `git rebase --continue` when its rebase is in progress) with its merge driver configured on the command line: `-c merge.<name>.driver=...` and `-c core.attributesFile=.git/clm/attributes`, a file it writes (`sync-owns-merge-drivers`). `--autostash` keeps the edits step 1 let through in the rebase's own state (`.git/rebase-merge/autostash`, per worktree, never the shared stash list) and restores them when the rebase finishes, so while the rebase stops on a conflict they are absent from the working tree, and `git rebase --abort` restores them too. None is on a path the rebase writes, so restoring one should never conflict; should git still report that applying the autostash resulted in conflicts (git then keeps the edit on the stash list as a new `autostash` entry), `rebase` states what it prints, so the caller recovers the edit by hand. When the branch adds a change, whatever its status (its `changes/<id>/change.md` present at `HEAD`, and absent both at `origin/main` and at the merge base recorded before step 2, or a *new archived change* as step 4 defines it), or applies a change as step 4 defines it, the attributes map `specs/**` to `clm-keep`, which keeps the side being rebased onto, and a conflict in `specs/` that the driver can't take, such as modify/delete, is resolved the same way: the path keeps `origin/main`'s side when it has one and is removed when it doesn't (`reapply`, `regenerate-dont-merge`, `keep-main-specs-on-spec-branches`). A change that isn't `applied` at `HEAD`, such as one set back to `proposed` to apply again after editing its deltas, still has its `Apply` commits in the branch's history, and replaying them over a `main` that changed the same `SPEC.md` must not stop the rebase on a file nobody wrote by hand. Which changes the branch adds, and which of them step 4 regenerates, are decided before the rebase starts and kept in the marker, so a resume uses the same driver and regenerates the same changes. On a branch that adds no change, such as a finish branch setting `status: active`, `specs/` is rebased like any other file.
3. When the rebase stops on a conflict that step 2 doesn't resolve, it prints each such file as `conflict <path>`, leaving the rebase in progress for the caller to resolve, `git add`, and run `clm rebase` again. On a branch that adds a change, it resolves the stop's `specs/` conflicts as step 2 says before it reports the rest.
4. **Regenerate** (`reapply`, `regenerate-applied-specs`): when the branch adds a change whose status is `applied` or a *new archived change* whose status is `done`, or applies a change, it replaces `specs/` with `origin/main`'s and applies the change's deltas and assets with `clm-apply`, leaving each `change.md` where and as it is. The branch adds a *new archived change* `<id>` when `changes/archive/<id>/change.md` is present at `HEAD` and absent both at `origin/main` and at the merge base recorded before step 2, and `changes/<id>/change.md` is absent at both of those too (`sync-new-archived-change`): a change archived on its own spec branch after `clm apply` (`archive-in-spec-pr`), whose deltas `main` never applied. That is the gate's *new archived change* (`gate`'s Design) less a change that is live on `main` and that the branch moves into the archive (`archive-move`) or edits: `main`'s `specs/` already holds its deltas, so applying them again would only report stale bases. The branch *applies* change `<id>` when `changes/<id>/change.md` is at `origin/main` with status `proposed` or `approved` and, at `HEAD`, has status `applied`, or is absent while `changes/archive/<id>/change.md` has status `done` (applied and archived on the branch, `archive-in-spec-pr`): a step left `proposed` on `main`, such as a deferred contract change (`contract-step-deferred`), that a later spec branch applies. `main` never applied its deltas, so they are regenerated as an added change's are, whether or not the branch adds a change of its own; a change only the branch adds, or one `main` holds `applied`, isn't one. Live and archived changes, added or applied, are regenerated together, in the order `clm-regenerate` chooses (`sync.regenerate`'s Design, **Order**): a change after the ones whose output its deltas' bases record, and otherwise in ID order (`reapply-order-follows-bases`). A new archived change whose status isn't `done` (a hand edit left it `applied`, or set it `abandoned`) is added, so step 2 resolves its `specs/` conflicts, but isn't regenerated, as a live change that isn't `applied` isn't. A stale delta is first merged where it can be (**Merge**, `sync.regenerate`'s Design); a delta still stale after that stops it before it writes anything to `specs/`, printing `check`'s stale-delta violations for the elements left stale and then the report (`stale-delta-diff`, `stale-delta-shows-main`, `stale-delta-names-movers`). This covers the change that stopped it, the first in that order with violations, and its deltas in node-ID order. For each delta, in `clm hashes` order, each element it MODIFIES or REMOVES (a Contract, a Design, a requirement, or the Contract and Design of a component the delta removes) that `apply/stale-base` judges stale against the tree the change was being applied to (`origin/main`'s, plus any earlier change re-applied before it) and that the merge left stale gets a report; a missing base, a base recorded for a section the delta doesn't modify, and frontmatter and asset bases get none. A MODIFIED Frontmatter replaces only the keys it lists, and an asset is replaced whole. The stale elements come from `clm-apply`'s `stale_bases` on that tree (`apply/stale-elements`), not from parsing the violation messages or comparing whole hashes: a MODIFIED requirement whose block has no `#### Serves` block of its own and whose base's first 8 hex digits are the rule's version is current, so an edit to only that rule's `#### Serves` block neither lists nor merges it, and `clm rebase` and `clm apply` agree on which elements are stale. The tree, the merge and the report are `clm-regenerate`'s `regenerated` with its merge (`sync.regenerate`'s Design: **Stages**, **Merge**, **Other deltas**). `clm-sync` mirrors the tree into `specs/` and, when the result differs from `HEAD`, commits it as `Re-apply <change>`; then it has `clm-regenerate` merge the other live changes' deltas against the tree (`Tree::other_deltas`, `sync.regenerate/stale-delta-merge`), and stops on what that leaves as it stops on a stale delta. On a branch that regenerates no change, when a commit in `origin/main..HEAD` changes a delta file of a live `proposed` or `approved` change, one the branch adds included (`git log --format=%H <onto>..HEAD -- <those changes' deltas/>`, read by path, never by a commit's message, so the program's `Merge` commits and a person's hand resolution count alike), it asks `regenerated` for `origin/main`'s tree with no change (`ids` empty), mirrors nothing, and calls `Tree::other_deltas` with that same empty list: each element a merge or hand resolution set to a base only the branch's tree had is taken back (a `Take back what this branch merged into <change>` commit) once nothing moves it. Such a triggered run judges every other live change against `origin/main`'s tree, so a stale delta `main` itself holds is merged where it can be (`Merge main into <change>`) and otherwise stops the run with its report; on a branch with no such commit nothing is extracted and that delta is left to `clm check`, as before.
5. **Spec changed** (`spec-changed`, `versioned-markers`): on a build's branch with a component (**Build branches**), it compares the component's bundle (`clm context <component>`, with the component's assets) between the merge base recorded before step 2 and `origin/main`. `CAPABILITIES.md` counts when the capability blocks `clm context` includes differ: those of the capabilities the component serves, or every capability's for the component that builds the capability tests, which then reports any capability's change, since each is its work. The last printed line tells the session to run `clm pending <component>` and `git diff <merge base> origin/main -- <paths>` and to adapt its work to the new text. The report holds the outcome for after step 8, so the session reads the change before going on. The report compares two trees that git has, so it needs no recorded basis, and it fires once: the next run's merge base is `origin/main`. The merge base is kept in `.git/clm/sync-base` from the run that takes it until a run completes step 8, so a run that stops earlier, or a rebase the caller finished with plain git, still owes the report. The session's markers cite the versions it built to, so a requirement that moved under it stays `behind` until it rebuilds and re-cites it; nothing has to be re-planned (`node-reconciliation`).
6. **Archive** (`archive-complete`, `archive-on-sync`): on a build's branch, for each change `archive-complete` names, with the markers scanned at `HEAD` and completeness as `check.coverage/status-report` defines it, it moves the change with `clm-apply`'s archive step as `clm archive` does. The archive and `Activate` commits ride on the build's branch, whose code makes it a code branch (`gate/archive-move`, `gate/status-only-exemption`). A change is complete only on a tree whose tests already cite its requirements, so the branch that completes it is the one that archives it, or, when two branches complete it between them, the next build's branch to sync. The `Activate` commit of a `unbuilt` component is made with the archive commit or alone. A destination already taken refuses that change's archive alone, before anything is written, since it is the one refusal `clm_apply::archive` can give once a status is `done`. **Proposal items** (`file-proposal-items`, `web-proposals-as-ledger-records`): in the same step, after archiving, it folds the ledger (`ledger`'s `fold`, over the `origin/clm/ledger/*` refs step 1 fetched and the clone's own). A change at `HEAD` under `changes/` or `changes/archive/` names the proposal in `resolves` (the graph's backlog items and changes, archived ones included). The file's frontmatter is `title`, the title as a JSON string (`serde_json::to_string`, which YAML's double-quoted form reads, as `clm defect` writes its title), `kind: idea` and `components` in the record's order with the components the graph at `HEAD` doesn't hold left out (`model/backlog-components` refuses them; `components: []` when none is left), and nothing else: no `priority`, `blocks` or `origin`. The file is built from the record alone, never from the clock or the branch, so two builds that file one proposal add the same bytes and their lands merge clean. A slug that isn't lowercase kebab case is skipped, since a hand-appended line could name any path. An item is judged to exist by the graph loaded for archiving, which reads the working tree, and also when an untracked `backlog/<slug>.md` is there, as a reconsider item is judged, so nothing is overwritten. The fold is `ledger`'s over the refs as the run fetched them; a resumed run, which skips the fetch, reads the refs the run that started the rebase fetched. Each item is committed alone as `File <slug>`, before step 8's temporary worktree, so the check judges the new items. A new item never makes the code PR mixed (`gate/backlog-additions`). An item deleted without a change resolving it comes back on the next build's sync, as a reconsider item does; a proposal is closed by a change that resolves it or by its proposer withdrawing it. An open proposal whose slug an item filed by other means took (a reconsider or defect item) is never filed and stays listed in the app until its proposer withdraws it.
7. **Reconsider item** (`file-reconsider-item`, `health-advises`): for a build's branch's component, whenever the run reaches this step (so also after a `spec changed` report, but not after a conflict or a stale delta, which stop earlier), it scores the component's requirements with `check.health`'s `reconsider`, from the graph and backlog items at `HEAD` (read in the temporary worktree) and the `health` settings the `clm` crate passes in: the closure returns `clm_health::Settings` alone, and its one caller, `crates/clm/src/main.rs`, is edited by this component's build, as the root's Design gives every build that changes a command (`clm/root-from-cwd`'s Design, `contract-removal-needs`). Items are filed in requirement-ID order. It files for the branch's own component only, so two builds never write the same item. The item is a `question`, a decision only a human makes, which `clm status` lists under `waiting on you`, and one deleted without a spec change comes back on the component's next build. No step measures the component's size or files an item for an oversized component (`oversized-shown-not-filed`); splitting waits until the re-founding's mapping and pruning have shrunk the components, when the AI decides splits without a human.
8. **Settings and check** (`judged-by-check`, `sync-carries-unrelated-edits`, `sync-judges-head`): on every branch that reaches step 5 (never after a conflict or a stale delta, which stop earlier), it judges the branch's commit, never the working tree. After step 6 and before step 7, it checks `HEAD` out detached into a fresh temporary worktree, `git worktree add --detach <dir> HEAD`, where `<dir>` is a new `sync-*` directory under the temporary directory `judged-by-check` names, made absolute, chosen as `clm land` chooses it, outside the repository as `clm land`'s are (`scratch-outside-git`); `clm-sync` has its own copy of that rule, since it doesn't depend on `clm-land`. Its CLM root is `<dir>` joined with the CLM root's path relative to the repository's top level (`git rev-parse --show-prefix`). The temporary worktree holds `HEAD`'s tracked files and nothing else, so an edit step 1 let through (to `clm.toml`, to a test file's `req:` marker, to a decision under `docs/decisions/`), an untracked file outside `specs/` and `changes/`, and an ignored file never decide the outcome: what it judges is what `clm land` merges. The clone's `clm.local.toml` and the user's `config.toml` are found from it as from the working tree, since `config` finds the local file through the clone's common git directory. It calls the closure the `clm` crate passes in (`FnOnce(&Path)`), once, with that CLM root, and step 7 takes the result, the `health` settings; step 7 also reads the graph and the backlog items in the temporary worktree, while it still looks for an existing `backlog/reconsider-<component>-<slug>.md` and writes and commits the items in the repository's working tree: the closure loads the settings from `HEAD` as the rebase and steps 4 and 6 left it, so it reads the settings `main` brought in. When they don't load, such as a `clm.toml` with a key the running `clm` doesn't know or a broken user `config.toml`, the closure's error, which the `clm` crate formats as a violation line (`clm/rebase-settings`), is printed where `judged-by-check` places it. `clm-sync` replaces a leading temporary CLM root, in the closure's error and in each violation's path, with the CLM root it was itself given (so the source prints as `./clm.toml` when the `clm` crate passes `.`), so the line names the file the caller would edit, never one under the temporary directory. `clm check` and `clm land` load the same settings and fail on them, so `rebase ok` never stands for a branch they would reject. When step 7 commits an item, it moves the temporary worktree to the new `HEAD` (`git checkout --detach <HEAD>` in it). Then it runs `clm check` on the temporary worktree's CLM root. It removes the temporary worktree (`git worktree remove --force <dir>`, then `git worktree prune`, as `clm land` does) on every exit path once it has made it, a git failure included; a removal that fails becomes a warning of the report, which the command call prints. A worktree a killed sync leaves behind stays in the temporary directory until the system clears it, and `git worktree prune` drops git's record of it once the directory is deleted; `clm rebase` sweeps nothing, as `clm land` doesn't. The check is the running binary's own, linked in from `clm-check`, never one built from the branch: like `clm land`, which runs `clm check` from its own executable (`land`), it judges the branch by the rules of the `clm` that runs it, which is `main`'s rule wherever the installed `clm` is a release or tracks `main` (`checks-relax-before-use`). So in a repository whose branch changes `clm check` itself, a branch that relaxes a check passes only if nothing in it relies on the relaxation; whatever only the relaxed check accepts, such as deleting a backlog item the old rule requires, goes in a later branch, once the relaxation has landed and the `clm` that runs is rebuilt from it. `clm rebase` never builds or runs another `clm`, and no prompt points it at one, since the branch would then pass sync and fail at `clm land`. The settings are the same: a `clm` that can't read `main`'s `clm.toml` fails the sync, and the fix is to rebuild it from `main`, not to point the session at another binary.

**Paths** (`paths-under-the-root`, `fingerprint-covers-the-clm-root`). Every path `clm rebase` matches, writes or prints is the CLM root's: the root may be a subdirectory of its git repository (`root-from-cwd`), and `git rev-parse --show-prefix`, run there once per run, gives its place, as it does for the gate and `rename/replay`. git prints some paths from the repository's top whatever directory it runs in: `git status --porcelain` (step 1's tracked paths), `git ls-files --full-name` (step 1's untracked files) and `git diff --name-only` (step 1's set of files `origin/main` changed, step 3's conflicts and step 5's `changed` lines). Each such path is first named relative to the root as the gate names one (`gate/paths-under-the-root`): under the prefix, with the prefix taken off; outside it, as the shortest path from the root, climbing out with `../`. Only then is it tested for `specs/`, `changes/` or `backlog/`, or compared with another such path, so a path outside the root, which starts `../`, is never under one of the root's trees, and a `specs/` directory at the repository's top is outside a root at `tool/`. A path handed back to git as a pathspec (step 2's `ls-files --stage`, `checkout --ours`, `add` and `rm` of a `specs/` conflict) is the root-relative one, which git reads from the directory it runs in; no path outside the root is ever handed back, since step 2 resolves only the root's `specs/` conflicts and reports the rest. git matches a pattern holding a slash in `core.attributesFile` from the repository's top, so the rule step 2 writes is `<prefix>specs/** merge=clm-keep`. Step 4's tree is `clm-regenerate`'s, which extracts and loads under the prefix (`sync.regenerate/regenerated-tree`); step 5 extracts `<prefix>specs`, `<prefix>ARCHITECTURE.md` and `<prefix>CAPABILITIES.md` with `clm-regenerate`'s `extract_present`, which runs no hook (`extract-through-layer`), and loads the graph and the bundle from `<prefix>` of the extraction, so a component's bundle in a subdirectory root is compared, not read as absent at both revisions. `git ls-tree` and `git ls-files` without `--full-name` already list paths relative to the directory they run in, and steps 6 and 7 read the root's files by root-relative paths, so they need nothing new. At a repository's top the prefix is empty and every path is as git prints it. `gate`'s Contract offers no call that names a path from the top relative to the root, so `clm-sync` holds one function for it, shared by steps 1, 3 and 5 and by the rename path (`regenerate-renames`), never one per step (`reuse-first`); it is a second copy of the gate's, which `shared-plumbing-in-git` would move to `clm-git` once a third crate needs it. git reads a blob named `<rev>:<path>` from the repository's top, so every such read of a root-relative path is written `<rev>:./<path>`, which git reads from the directory it runs in: the `change.md` reads by which step 2 decides which changes the branch adds or applies, at `HEAD` and at `origin/main`, among them. The rename path's own reads and lines are `regenerate-renames`'s.

**Autosync** (`autosync`, `autosync-policy`). The snapshot is built under a temporary `GIT_INDEX_FILE`, starting from `HEAD`'s tree, with `git add -A` of each spec path that exists or that `HEAD` tracks, so a deletion is recorded, and `git commit-tree`, which ignores `commit.gpgsign`. It pushes with `+<snapshot>:refs/heads/autosync/<branch>`, `GIT_TERMINAL_PROMPT=0` and, unless the environment sets `GIT_SSH_COMMAND`, `GIT_SSH_COMMAND=ssh -o BatchMode=yes -o ConnectTimeout=10`, then sets `refs/remotes/origin/autosync/<branch>` to it. Off a `spec/*` branch, on a detached `HEAD` and outside a repository it does nothing and says nothing, since plugin.hooks's `Stop` hook runs it every turn. The `spec/` name is the one name `clm rebase` and `clm autosync` read alone: it is the claim whose work autosync publishes for `clm proposals` and other sessions, and nothing about what may land rests on it.

## ADDED Requirements

### Requirement: build-branches
`clm rebase` SHALL ask the gate, as `gate/evaluate-at` states, for the current branch judged against `origin/main` before the rebase, and SHALL treat the branch as a build's exactly when the gate reads its kind as `code`, `light` or `mixed`, or when its name starts `impl/` and the gate says it needs no review; a branch the gate says needs review SHALL NOT be a build's, whatever its name. When the branch and `origin/main` have no merge base it SHALL NOT ask the gate, and the branch SHALL be a build's exactly when its name starts `impl/`. A build's branch SHALL have as its component `<component>` when it is named `impl/<component>` with no further `/`, and otherwise the component the gate reads from its files (`gate/own-files`), and none when the gate reads none, as for a `code` branch in a project whose root sets no `wiring` (a `light` branch always has its component); on a build's branch with no component it SHALL archive and file proposal items, and SHALL report no changed spec and file no reconsider item. Specs at the merge base that don't load SHALL NOT fail the run: the component SHALL be read from them as `gate/own-files` reads it, a component whose `SPEC.md` doesn't parse there owning no file and every other still read, and a root whose `SPEC.md` doesn't parse there naming no `wiring`, so that a `code` branch then has no component; whether it archives and files proposal items is judged by the graph at `HEAD` (`archive-complete`, `file-proposal-items`). Which uncommitted work stops a run (`rebase`), and whether it reports a changed spec (`spec-changed`), archives (`archive-complete`) and files proposal and reconsider items (`file-proposal-items`, `file-reconsider-item`), SHALL be decided by this and by nothing else of the branch's name.

#### Scenario: a build's branch of another name
- GIVEN a root that sets `wiring: []`, component `a` owning `crates/a/**`, and an `origin/main` that has since modified requirement `a/r`, holds a complete `applied` change `x`, and whose ledger holds an open proposal with slug `s`
- WHEN `clm rebase` runs
- THEN:
  - on a branch `fix/y` whose one commit edits `crates/a/src/lib.rs`, it prints `spec changed a`, `archived x` and `filed backlog/s.md`
  - on that branch with an uncommitted edit to `README.md`, which `origin/main` hasn't changed, it prints `uncommitted README.md` and exits 1
  - on a branch `docs/z` whose one commit edits only `docs/guide/x.md`, it prints no `spec changed`, `archived` or `filed` line
  - on a branch `impl/archive/x` that changes no code file, it prints `archived x` and no `spec changed` line
  - on a branch `impl/a` whose one commit edits `ARCHITECTURE.md`, which the gate says needs review, it prints no `archived` or `filed` line and `HEAD` holds no commit it made
  - in a project whose root sets no `wiring`, on `fix/y` as above, it prints `archived x` and `filed backlog/s.md` and no `spec changed` line
  - on `fix/y` as above, where the merge base's `specs/b/SPEC.md` doesn't parse and `HEAD`'s does, it prints `spec changed a`, `archived x` and `filed backlog/s.md`; where instead the merge base's root `specs/SPEC.md` doesn't parse and `HEAD`'s does, it prints `archived x` and `filed backlog/s.md` and no `spec changed` line
  - on a branch `light/y`, which the gate reads `mixed` while `y` is live and `light` once it is archived, that adds change `y`, `applied`, resolving nothing, whose delta adds a scenario to `a/q` of `a`, already `active`, with a commit editing `crates/a/src/lib.rs` whose test cites `a/q` at its new version, it prints `archived y`, `HEAD` holds `changes/archive/y/change.md` with `status: done`, `specs/a/SPEC.md` holds `y`'s scenario, and a second run prints no `archived` line and leaves `specs/a/SPEC.md` as it was

#### Needs
- gate/evaluate-at
- gate/own-files
- gate/light

## MODIFIED Requirements

### Requirement: rebase <!-- base: 3ba762e988e527ca -->
`clm rebase` SHALL exit 1 without changing anything but `origin`'s remote-tracking refs on `main`, on a detached `HEAD`, while a rebase or a `git am` it didn't start is in progress, or when the working tree has uncommitted work and no rebase started by `clm rebase` is in progress. A rebase it started is the one it began itself, until that rebase finishes or is aborted, however that is done: a rebase begun by hand after its own was finished with plain git is not one. On `main`, on a detached `HEAD` and while a rebase or `git am` it didn't start is in progress it SHALL print `clm rebase: <reason>` on stderr, the reason naming `main`, the detached `HEAD`, or the rebase in progress and that it is finished with `git rebase --continue` or given up with `git rebase --abort`, or for a `git am`, `git am --continue` and `git am --abort`. For uncommitted work it SHALL print on stdout `uncommitted <path>` for each file of that work, one line per path, sorted, a renamed file under both its old and its new path. Uncommitted work is a staged change; a tracked file modified or deleted under `specs/`, `changes/` or `backlog/`; a tracked file modified or deleted that `origin/main` changed since the merge base of `HEAD` and `origin/main`, or every one when they have none; on a build's branch (`build-branches`), any tracked file modified or deleted; and an untracked file that isn't ignored under `specs/` or `changes/`. Any other modified tracked file or untracked file, an ignored one under `specs/` or `changes/` included, SHALL NOT stop it. Otherwise it SHALL fetch `origin` and rebase the current branch onto `origin/main`, setting aside the modified tracked files it let through and restoring them, unchanged and uncommitted, when the rebase finishes; when restoring them conflicts it SHALL print on stderr that they are safe in the stash, naming the stash entry and `git stash apply <entry>`, and exit 2. When the rebase would write over an untracked file it SHALL print on stderr git's message naming the file, leave the file as it was, and exit 2. When the rebase stops on conflicts it SHALL print `conflict <path>` for each conflicted file, leave the rebase in progress, and exit 1. Run again with the conflicts resolved and staged, it SHALL continue the rebase. A git command it runs that meets an `index.lock` another process holds SHALL be run again as `sync.regenerate/retry-index-lock` states before the run fails. It SHALL NOT push.

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
- GIVEN branch `spec/x` with an uncommitted edit to `.claude/settings.json`, and a commit that conflicts with one `origin/main` has gained in `docs/guide.md`
- WHEN `clm rebase` runs, the conflict is resolved and staged, and `clm rebase` runs again
- THEN the first run prints `conflict docs/guide.md` and exits 1, and after the second `.claude/settings.json` holds the edit, uncommitted

#### Scenario: untracked output and worktrees
- GIVEN a branch behind `origin/main` with an untracked `test-results/report.txt` and an untracked `.claude/worktrees/other/README.md`, neither ignored, and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN it prints no `uncommitted` line, the branch is rebased onto `origin/main`, both untracked files are still there unchanged and untracked, and it exits 0

#### Scenario: untracked delta
- GIVEN a branch with an untracked `changes/x/deltas/a.md` and nothing else uncommitted
- WHEN `clm rebase` runs
- THEN it prints `uncommitted changes/x/deltas/a.md`, exits 1, and the branch and working tree are unchanged

#### Scenario: refused sync keeps its output
- GIVEN a checkout on `main`
- WHEN `clm rebase` runs
- THEN:
  - it prints `clm rebase: ` and the reason, naming `main`, on stderr, its last line of stdout is `rebase incomplete`, and it exits 1
  - on a detached `HEAD`, the reason names the detached `HEAD`, and the outcome is the same

#### Scenario: a rebase started by hand
- GIVEN a branch on which `git rebase origin/main`, run by hand, stopped on a conflict in `src/lib.rs`
- WHEN `clm rebase` runs
- THEN:
  - it prints `clm rebase: ` and a reason naming the rebase in progress, `git rebase --continue` and `git rebase --abort` on stderr, its last line of stdout is `rebase incomplete`, it exits 1, the rebase is still in progress, and the working tree and index are unchanged
  - the same holds when a rebase `clm rebase` started, stopped on a conflict, was finished with `git rebase --continue` and a rebase by hand then stopped
  - when a `git am` stopped on a patch that doesn't apply, the reason names `git am --continue` and `git am --abort`, and the rest is the same

#### Scenario: uncommitted work named path by path
- GIVEN a branch `topic` with `src/lib.rs` renamed to `src/moved.rs` and staged, a staged new `src/staged.rs`, a deleted tracked `specs/SPEC.md`, an untracked `specs/b/SPEC.md`, and an ignored `specs/a/scratch.tmp`
- WHEN `clm rebase` runs
- THEN:
  - it prints exactly `uncommitted specs/SPEC.md`, `uncommitted specs/b/SPEC.md`, `uncommitted src/lib.rs`, `uncommitted src/moved.rs` and `uncommitted src/staged.rs`, in that order, and exits 1
  - on a branch `spec/y` that shares no history with `origin/main`, an uncommitted edit to `src/lib.rs` prints `uncommitted src/lib.rs` and exits 1

#### Scenario: untracked file in the rebase's way
- GIVEN a branch with an untracked, unignored `test-results/report.txt`, and an `origin/main` that has since added `test-results/report.txt`
- WHEN `clm rebase` runs
- THEN its stderr names `test-results/report.txt`, the file holds what it held, its last line of stdout is `rebase failed`, and it exits 2

#### Scenario: set-aside edit can't be restored
- GIVEN a branch behind `origin/main` with an uncommitted edit that the rebase sets aside, and a working tree in which putting the edit back conflicts once the rebase has finished
- WHEN `clm rebase` runs
- THEN its stderr names the stash entry and `git stash apply` with it, the stash list holds the edit, its last line of stdout is `rebase failed`, and it exits 2

#### Scenario: lock met at the commit
- GIVEN an applied spec branch `spec/x` adding `x`, and an `origin/main` that has since added a requirement to the same `SPEC.md`, so the regenerate commits `Re-apply x`, where the worktree's `index.lock` is held from the end of the rebase and released half a second later
- WHEN `clm rebase` runs
- THEN `HEAD` is a commit `Re-apply x`, nothing is left staged, its last line is `rebase ok`, and it exits 0

#### Scenario: lock met at every commit it makes
- GIVEN a branch on which `clm rebase` makes a commit, and first on the `PATH` it runs with a stand-in `git` that, the first time it is given a `commit` command (its first argument after any `-c <key>=<value>` pairs being `commit`), writes to stderr `fatal: Unable to create '<git dir>/index.lock': File exists.` and exits 128 without running git, and hands every other call, and every later `commit`, to the real `git` unchanged
- WHEN `clm rebase` runs
- THEN, in each case, `HEAD` is the commit named, the stand-in was given that `commit` twice, nothing is left staged, and its last line is `rebase ok`:
  - on `impl/a` completing `applied` change `x`: `Archive x`
  - on `impl/a` where `a` is `unbuilt` with nothing pending: `Activate a`
  - on `impl/a` with an open proposal with slug `s`: `File s`
  - on an applied spec branch adding `x` behind an `origin/main` that has since changed `a`'s `SPEC.md`: `Re-apply x`
  - on that branch, with a `proposed` change `o` whose delta `x`'s Design edit makes stale and merges with: `Merge x into o`
  - on a spec branch whose earlier run committed `Merge x into o` and whose `x` no longer edits `a`'s Design: `Take back what this branch merged into o`
  - on the branch of *main moved under a rename* (`regenerate-renames`): `Rename r`

### Requirement: reapply <!-- base: 85e68c879c24509e -->
When the branch adds a change whose status is `applied`, adds a new archived change whose status is `done`, or applies a change, `clm rebase` SHALL, after rebasing, set `specs/` to `origin/main`'s `specs/` with the change's deltas and assets applied, leaving its `change.md` where and as it is, and commit the result when it differs from `HEAD`. The branch adds a change when its `changes/<id>/change.md` is at `HEAD` but neither at `origin/main` nor at the merge base of `HEAD` and `origin/main` taken before rebasing, so a change that `main` archived or removed since the branch was cut is never re-applied. It adds a new archived change when `changes/archive/<id>/change.md` is at `HEAD` but neither at `origin/main` nor at that merge base, and `changes/<id>/change.md` is at neither of them either, so a change `main` applied and the branch archives is never re-applied. It applies a change when that change's `changes/<id>/change.md` has status `proposed` or `approved` at `origin/main` and, at `HEAD`, has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, so the deltas of a change `main` holds unapplied are regenerated whether or not the branch adds a change too. Changes SHALL be re-applied live and archived together, in the order `sync.regenerate/regenerated-tree` states: a change whose deltas record bases another of them produces after that change, whatever their IDs, and otherwise in ID order. When the branch adds a change or a new archived change, whatever its status, or applies a change, `clm rebase` SHALL resolve each conflict in `specs/` during the rebase by keeping the side being rebased onto (`origin/main` with the branch's commits replayed so far), removing the path when that side has none, staging the result, and SHALL NOT report it as a conflict, also when the same stop has conflicts outside `specs/`. A `change.md` counts as added whether or not its frontmatter parses. When a delta is still stale against `origin/main` after `sync.regenerate/stale-delta-merge`, it SHALL write nothing to `specs/`, print the elements left stale, and exit 1. When it regenerated a change, after the `Re-apply` commit, it SHALL merge, as `sync.regenerate/stale-delta-merge` states, the deltas of every other live `proposed` or `approved` change, the branch's own unapplied ones included, as `sync.regenerate/stale-delta-merge` states, print the `merged` and `took back` lines, and, when a change is still refused after that, print what is left and exit 1, keeping the `Re-apply`, `Merge` and `Take back` commits. When it regenerated no change and a commit the branch holds since `origin/main` changes a delta file of a live `proposed` or `approved` change, one the branch adds included, whatever the commit's message, it SHALL still have the other live changes' deltas merged and taken back against the tree `sync.regenerate/regenerated-tree` gives for no change, `origin/main`'s `specs/`, as `sync.regenerate/stale-delta-merge` states, so an element a merge or a hand resolution set to a base only the branch's tree had is taken back once nothing moves it; it SHALL print the `merged` and `took back` lines, stop on what is left as above, and write nothing to `specs/`. Such a run SHALL judge every live `proposed` or `approved` change, the branch's own unapplied ones included, against that tree, merging a delta stale there, `origin/main`'s own among them, where it can, and otherwise stopping on it with its report. It SHALL NOT regenerate `specs/`, or resolve conflicts in `specs/` itself, on any other branch.

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
- GIVEN a branch adding only a `proposed` change `x`, which has no delta file, and an `origin/main` with a `proposed` change `o` whose delta modifies `a`'s Design with a base that isn't `a`'s Design hash there
- WHEN `clm rebase` runs
- THEN it prints no `merged` line, `o`'s delta is unchanged, and it prints `clm check`'s stale-base violation for it and exits 1

#### Scenario: an edit taken back
- GIVEN an `origin/main` with a `proposed` change `o` whose delta modifies `a`'s Design with base `origin/main`'s hash, rewriting its last line, and an applied spec branch adding `x`, whose delta rewrote the first line of `a`'s Design, on which an earlier `clm rebase` committed `Merge x into o`
- WHEN `clm rebase` runs
- THEN:
  - after a commit that edits `x`'s delta to leave `a`'s Design alone and applies `x` again, it prints `took back design of a in changes/o/deltas/a.md to base <origin/main's hash>`, `HEAD` is a commit `Take back what this branch merged into o`, `changes/o/deltas/a.md` is `origin/main`'s byte for byte, it prints no stale-base violation, and its last line is `rebase ok`
  - after a commit that removes `x` and sets `specs/` back to `origin/main`'s, so that the branch regenerates no change, the same holds
  - when `o`'s delta was instead rebased onto `x`'s Design by a hand commit `fix o`, and `x` was then removed as above, the same holds
  - on a branch that adds `proposed` change `p`, whose delta modifies `a`'s Design with base `origin/main`'s hash, and `x`, on which an earlier `clm rebase` committed `Merge x into p`, after a commit that sets `x` back to `proposed`, so the branch regenerates no change, it prints `took back design of a in changes/p/deltas/a.md to base <origin/main's hash>`, and `changes/p/deltas/a.md` is as the branch wrote it before that merge
  - with nothing changed since that earlier run, it prints no `merged` or `took back` line and `HEAD` is unchanged

#### Needs
- sync.regenerate/regenerated-tree
- sync.regenerate/stale-delta-merge

### Requirement: judged-by-check <!-- base: bfefb55e938b3d33 -->
After a rebase that completes and any regeneration, `clm rebase` SHALL load the settings on every branch, printing the error and exiting 1 when they don't load, and SHALL run every `clm check` rule on the result, print any violations, and exit 1 if there are any, 0 otherwise. The settings error SHALL be printed after the `archived`, `activated` and `filed` lines and before the check's violations, and the check SHALL still run. It SHALL load the settings and run the check on the branch's `HEAD` commit, checked out in a temporary worktree outside the repository, never on the working tree, so no uncommitted edit and no untracked or ignored file changes the outcome, and SHALL remove that worktree before it exits. It SHALL make the worktree under the directory `TMPDIR` names in the environment it is given, when set and not empty, and else under the system's temporary directory; when it can't make the worktree, or move it to a commit it filed, it SHALL print why on stderr, naming that directory when the worktree can't be made in it, and exit 2. When it can't remove the worktree it SHALL keep the outcome and exit status it had and print `clm rebase: couldn't remove the temporary worktree <dir>: <why>` on stderr, or, when the run had failed, print that failure with the same words joined to it. A file that a settings error or a violation names SHALL be printed by its path in the repository, never by its path in the temporary worktree.

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
- GIVEN branch `spec/x` whose `src/lib.rs`, as the merge base and `origin/main` both hold it, cites `req:a/missing`, a requirement no component has, with an uncommitted edit that removes the citation
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

#### Scenario: settings error before the violations
- GIVEN branch `spec/x` whose committed tree cites `req:a/missing`, and settings that don't load
- WHEN `clm rebase` runs
- THEN its first line of stdout is the settings error, a `check/dangling-markers` line follows it, its last line is `rebase incomplete`, and it exits 1

#### Scenario: no room for the temporary worktree
- GIVEN branch `impl/a` and an environment, given to `rebase_command`, whose `TMPDIR` names a file
- WHEN `clm rebase` runs
- THEN its stderr names that file, its stdout is `rebase failed`, and it exits 2

#### Scenario: worktree left behind
- GIVEN branch `spec/x` whose committed tree passes `clm check`, and a temporary worktree that can't be removed once it is made (a directory in it that its owner can't write, holding a file)
- WHEN `clm rebase` runs
- THEN:
  - its stderr holds `clm rebase: couldn't remove the temporary worktree <dir>: ` and why, `<dir>` the `sync-*` directory, its last line of stdout is `rebase ok`, and it exits 0
  - when the check also fails, its last line is `rebase incomplete` and it exits 1

#### Serves
- capability/catch-drift
- architecture/no-silent-drift

#### Needs
- vcs/git-follows-its-clone

### Requirement: autosync <!-- base: 9720eb37ddbb9826 -->
On a branch named `spec/*`, `clm autosync` SHALL push to `refs/heads/autosync/<branch>` on `origin` a snapshot of the spec work: `HEAD` itself when the spec paths hold nothing uncommitted, and otherwise a commit whose parent is `HEAD` and whose tree is `HEAD`'s with the spec paths taken from the working tree, untracked files included. The spec paths are `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `AGENTS.md`, each the CLM root's, as `gate/paths-under-the-root` reads paths. It SHALL take no uncommitted edit and no untracked file outside the spec paths into it, and SHALL leave the working tree, the index and the branch unchanged. It SHALL push nothing on any other branch, on a detached `HEAD`, outside a git repository, or when the snapshot's tree equals that of `origin/<branch>` or `origin/autosync/<branch>`. It SHALL NOT sign the snapshot, whatever the clone's settings, prompt on the terminal for a credential, or let ssh ask for a passphrase or a host key or wait more than ten seconds to connect, unless the environment sets `GIT_SSH_COMMAND`, which it SHALL hand to git as given; the environment is the lookup `autosync_command` is given, and this process's own for `autosync`. It SHALL print nothing and exit 0 whether or not it pushed; when git fails it SHALL print `clm autosync: <why>` on stderr, nothing on stdout, and exit 2. `autosync(root)` SHALL do what `clm autosync` does for the CLM root it is given, and return the failure `clm autosync` would print.

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

#### Scenario: uncommitted instructions
- GIVEN a spec branch's checkout with an uncommitted edit to `AGENTS.md` alone
- WHEN `clm autosync` runs
- THEN the snapshot it pushes holds that edit

#### Scenario: nothing uncommitted
- GIVEN branch `spec/x` whose spec work is all committed, and that `origin` doesn't hold
- WHEN `clm autosync` runs
- THEN `origin/autosync/spec/x` is `HEAD` itself

#### Scenario: nothing to answer
- GIVEN branch `spec/x` with an uncommitted delta
- WHEN `clm autosync` runs
- THEN:
  - with `commit.gpgsign` set to `true` and a `gpg.program` that fails, it pushes the snapshot and exits 0
  - with an `origin` reached over ssh and no `GIT_SSH_COMMAND` in the environment, the `ssh` git runs is given `-o BatchMode=yes` and `-o ConnectTimeout=10`
  - with `GIT_SSH_COMMAND`, in the environment `autosync_command` is given and not in the process's, set to a command that writes a file and fails, the file is written and it exits 2

#### Scenario: how it exits
- GIVEN branch `spec/x` with an uncommitted delta
- WHEN `clm autosync` runs
- THEN:
  - when it pushes, it prints nothing and exits 0
  - with an `origin` whose URL names a path that doesn't exist, it prints `clm autosync: ` and why on stderr, nothing on stdout, and exits 2
  - with `--root` naming a directory outside any git repository, it prints nothing and exits 0

#### Serves
- capability/work-concurrently
- capability/work-from-any-agent-harness

### Requirement: outcome-line <!-- base: 45f1fe6d173918ac -->
For every run whose arguments it parses, `clm rebase` SHALL end its stdout with exactly one line stating its outcome: `rebase ok` when it exits 0, `rebase incomplete` when it exits 1, and `rebase failed` when it exits 2. No earlier line SHALL be one of those three. Given `--help` it SHALL print the help and no outcome line, and given arguments it can't parse it SHALL print nothing on stdout.

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

#### Scenario: help and usage errors
- GIVEN any checkout
- WHEN `clm rebase --help` runs, and then `clm rebase --no-such-option`
- THEN the first prints the help, no line of which is `rebase ok`, `rebase incomplete` or `rebase failed`, and exits 0; the second prints nothing on stdout and exits 2

#### Serves
- capability/work-concurrently
- architecture/deterministic-enforcement

### Requirement: spec-changed <!-- base: 14245755a4650970 -->
On a build's branch that has a component (`build-branches`), `clm rebase` SHALL, after rebasing and regenerating, print `spec changed <component>` when the component's bundle differs between the branch's merge base before the rebase and `origin/main`, followed by `changed <path>` for each file the bundle is read from that differs (`ARCHITECTURE.md`, `CAPABILITIES.md` when the block of a capability in the component's bundle differs, one the component serves or, for the component that builds the capability tests, any (`context/bundle-contents`), the bundle's components' `SPEC.md`s and the component's assets), `requirement <id> <old> -> <new>` for each of the component's own requirements whose version changed, appeared or went, `<old>` being `added` for one that appeared and `<new>` being `removed` for one that went, and a line naming `clm pending <component>` and the `git diff` to read, and exit 1 after running `clm check`. A run that stops before the report, at a stale delta, a conflict or a git failure, and a rebase it started that the caller finished with plain git, SHALL leave the report owed: the next run that reaches the report SHALL give it against the merge base taken before the first of those rebases. It SHALL print nothing of the kind, and exit by the other rules, when the bundle is unchanged, on any other branch, or on the next run with nothing new.

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

#### Scenario: requirements that appeared or went
- GIVEN branch `impl/a`, and an `origin/main` that has since added requirement `a/n` at version `33333333` and removed requirement `a/o` at version `44444444`
- WHEN `clm rebase` runs
- THEN it prints `requirement a/n added -> 33333333` and `requirement a/o 44444444 -> removed`, and exits 1

#### Scenario: a stopped run still owes the report
- GIVEN branch `impl/a`, and an `origin/main` that has since modified requirement `a/r`
- WHEN `clm rebase` runs, stopping before the report, and then runs again
- THEN:
  - when the first run stops at a stale delta of the branch's applied change, and the change is then dropped, the second run prints `spec changed a` and `requirement a/r <old> -> <new>`
  - when the first run stops at a conflict, which the caller resolves and finishes with `git rebase --continue`, the second run prints the same lines

#### Serves
- capability/build-from-spec
- architecture/hash-based-staleness

#### Needs
- context/bundle-contents

### Requirement: archive-complete <!-- base: aaaf914b129830e8 -->
On a build's branch (`build-branches`), `clm rebase` SHALL, after rebasing and regenerating, set to `done` and move to `changes/archive/` each `applied` or `implementing` change at `HEAD` whose added and modified requirements are all current at `HEAD` and whose removed requirements no marker at `HEAD` cites, delete each tracked `backlog/<slug>.md` its `resolves` names, leaving an untracked one, commit the result as `Archive <id>`, and print `archived <id>`. A marker in an untracked file SHALL NOT count toward a change's completeness or a component's pending requirements. It SHALL also set `status: active` in the `SPEC.md` of each `unbuilt` component with no pending requirement at `HEAD`, print `activated <component>`, and commit it as `Activate <component>` when no archive commit carries it. When `changes/archive/<id>` already exists for a complete change, it SHALL leave that change where it is, print `changes/archive/<id>: sync/archive-complete: ` followed by `` `changes/archive/<id>` already exists ``, and exit 1, and SHALL still archive each other complete change and activate each component as above. When the graph at `HEAD` has violations it SHALL archive and activate nothing, leaving them to `clm check`. It SHALL NOT archive a change on any other branch, one with a pending requirement, or one that hasn't applied.

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
- THEN it prints `activated a`, and `specs/a/SPEC.md` at `HEAD` says `status: active`, committed as `Activate a`

#### Scenario: spec branch
- GIVEN branch `spec/y` and a complete change `x` outside the archive
- WHEN `clm rebase` runs
- THEN it prints no `archived` line

#### Scenario: untracked files left out
- GIVEN branch `impl/a` whose tests cite the last requirement of `applied` change `x`, where `x` has `resolves: [u]`
- WHEN `clm rebase` runs
- THEN:
  - with `backlog/u.md` untracked, it prints `archived x` and `backlog/u.md` is still there, untracked
  - when the only test citing that requirement at its current version is in an untracked file, and `a` is `unbuilt`, it prints no `archived` or `activated` line and `HEAD` is unchanged

#### Scenario: archive's place taken
- GIVEN branch `impl/a`, where complete changes `w` and `x` are under `changes/`, component `a` is `unbuilt` with nothing pending, and a file at `changes/archive/x` that `.git/info/exclude` ignores
- WHEN `clm rebase` runs
- THEN it prints `archived w`, `activated a` and `changes/archive/x: sync/archive-complete: ` followed by `` `changes/archive/x` already exists ``, `changes/x/` is still there, its last line is `rebase incomplete`, and it exits 1

#### Scenario: graph that doesn't load
- GIVEN branch `impl/a` whose tests complete `applied` change `x` and every requirement of `unbuilt` component `a`, and a component at `HEAD` whose `depends_on` names a component that doesn't exist
- WHEN `clm rebase` runs
- THEN it prints no `archived` or `activated` line, `changes/x/` is still there, `specs/a/SPEC.md` still says `status: unbuilt`, it prints the graph's violation, and it exits 1

#### Serves
- capability/wrap-up-session
- architecture/spec-is-current

### Requirement: extract-through-layer <!-- base: ccb1a4950e930e84 -->
`clm rebase` SHALL read a revision's files without checking the revision out: when it compares a component's spec at two revisions (`spec-changed`) and when it regenerates (`reapply`), it SHALL check out nothing in any directory but the repository's working tree and the temporary worktree it judges in (`judged-by-check`), so a `post-checkout` hook runs only there.

#### Scenario: no post-checkout hook when regenerating
- GIVEN a repository whose `post-checkout` hook appends the directory it runs in to a log file outside the repository, and a branch `spec/x` that adds an `applied` change `x` and is behind `origin/main`
- WHEN `clm rebase` runs
- THEN it regenerates `specs/` as before, its last line is `rebase ok`, the log isn't empty (the rebase's own checkouts), and every line of it names the repository's working tree or the `sync-*` worktree sync judges in (`sync-judges-head`), none another directory

#### Scenario: no post-checkout hook when comparing a node's spec
- GIVEN the same hook, and a branch `impl/a` that is behind an `origin/main` whose `specs/a/SPEC.md` changed a requirement since the branch was cut
- WHEN `clm rebase` runs
- THEN it prints `spec changed a`, the log isn't empty, and every line of it names the repository's working tree or a `sync-*` worktree, none another directory

#### Serves
- architecture/reuse-first

### Requirement: file-reconsider-item <!-- base: 1f67b16bbf238a7e -->
On a build's branch whose component is `<component>` (`build-branches`), after archiving and filing proposals, `clm rebase` SHALL, for each requirement of `<component>` whose reconsider score (`check.health/reconsider-score`) reaches the `reconsider` setting and for which no `backlog/reconsider-<component>-<slug>.md` exists in the working tree, tracked or not, write the item `check.health/reconsider-score` gives for it (`title: "Reconsider <component>/<slug>"`, `kind: question`, `components: [<component>]`, a body with the score, its evidence and the three ways out), commit it alone as `File reconsider-<component>-<slug>`, and print `filed backlog/reconsider-<component>-<slug>.md`. It SHALL NOT file an item on any other branch, for a requirement of another component, for a requirement below the threshold, or when the item exists, and SHALL NOT edit or delete an existing item. When the settings couldn't be loaded it SHALL file nothing, `judged-by-check` reporting why. When an item's commit fails, this item or one `file-proposal-items` files, it SHALL leave no item and no staged change, and exit 2. The score SHALL NOT change its outcome line or exit code.

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

#### Scenario: a commit that fails
- GIVEN a `pre-commit` hook that exits 1
- WHEN `clm rebase` runs
- THEN:
  - on branch `impl/a`, where requirement `a/x` has three `workaround` items, there is no `backlog/reconsider-a-x.md`, `git status --porcelain` prints nothing, its last line is `rebase failed`, and it exits 2
  - on branch `impl/a`, with an open proposal with slug `s` in the ledger fold, there is no `backlog/s.md`, `git status --porcelain` prints nothing, its last line is `rebase failed`, and it exits 2

#### Serves
- capability/know-health

#### Needs
- check.health/reconsider-score
- config/health-setting
- model/component-keys

### Requirement: file-proposal-items <!-- base: e75d388ff0dec85e -->
On a build's branch (`build-branches`), after archiving, `clm rebase` SHALL, for each open proposal in the ledger fold, a `proposed` record no `withdrawn` record names, in slug order, whose slug no backlog item at `HEAD` has and no change at `HEAD`, archived or not, names in `resolves`, write `backlog/<slug>.md` with the record's `title` written as a JSON string, `kind: idea`, under the key `components` (`model/component-keys`) those of the components it lists (`/nodes`) the graph at `HEAD` holds, and a body of the record's `body` trimmed of trailing line breaks followed by a blank line and the line `Proposed in the web app by <writer> at <at> (record <id>).`, the body and its blank line left out when the body is empty, built from the record alone; commit it alone as `File <slug>`; and print `filed backlog/<slug>.md`. Of open proposals sharing a slug it SHALL file the earliest by `at`, then by record ID, and leave the rest. It SHALL count an untracked `backlog/<slug>.md` as an existing item. It SHALL skip a proposal whose slug isn't lowercase kebab case (`[a-z0-9]+(-[a-z0-9]+)*`), writing nothing for it, with the warning `skipped proposal <id> by <writer>: its slug "<slug>" isn't lowercase kebab case`; when the ledger can't be read it SHALL file nothing and warn `no proposals filed: the ledger can't be read: <why>`; and it SHALL pass on each warning the fold gives as `ledger: <warning>`. When the graph at `HEAD` has violations it SHALL file nothing. A commit that fails is handled as `file-reconsider-item` states. It SHALL file nothing on any other branch, for a withdrawn proposal, or for one whose item exists or whose slug a change resolves, and SHALL NOT edit or delete an existing item. Filing SHALL NOT change its outcome line or exit code.

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

#### Scenario: how an item is written
- GIVEN branch `impl/a`
- WHEN `clm rebase` runs
- THEN:
  - for an open proposal whose title is `Say "hi": now`, the item's frontmatter holds `title: "Say \"hi\": now"`, which `clm check` accepts
  - for one whose body is empty, the item is its frontmatter, a blank line and the `Proposed in the web app by` line, and nothing else
  - for two open proposals with slug `s`, the item is written from the one with the earlier `at`, or of equal `at` the lower record ID, and one `filed backlog/s.md` line is printed

#### Scenario: proposals not filed
- GIVEN branch `impl/a`
- WHEN `clm rebase` runs
- THEN:
  - for an open proposal whose slug is `Not Kebab`, nothing is written for it, and its stderr holds `clm rebase: skipped proposal <id> by <writer>: its slug "Not Kebab" isn't lowercase kebab case`
  - for an open proposal with slug `u` and an untracked `backlog/u.md`, no `filed` line is printed and `backlog/u.md` is unchanged
  - when the ledger can't be read, no `filed` line is printed and its stderr holds `clm rebase: no proposals filed: the ledger can't be read: ` and why
  - for a ledger line the fold skips, its stderr holds `clm rebase: ledger: ` and the fold's warning
  - when the graph at `HEAD` has a violation, no `filed` line is printed
  - in each case its exit status is what it would be without the proposal

#### Serves
- capability/follow-from-browser
- architecture/git-is-truth

#### Needs
- ledger/proposal-records
- model/component-keys

### Requirement: regenerate-renames <!-- base: e831ba37ba1d13d3 -->
On a branch that adds exactly one change, live or new archived, whose `change.md` at `HEAD` sets `renames`, `clm rebase` SHALL, after step 1's refusals and the fetch and in place of the rebase and the re-apply, first ask `rename/replay` whether the branch replays against its merge base with `origin/main`. When it doesn't, it SHALL print `not a rename branch <path>` for each path whose content differs from the replay, change nothing, and exit 1. When it does, it SHALL make the branch one commit on `origin/main`, `Rename <change-id>`, holding what `clm rename <change-id>` produces from `origin/main`'s tree with that `change.md` placed live with status `proposed`, plus the files under `backlog/` that the branch added; when `clm rename` refuses, it SHALL print the refusal, leave the branch as it was, and exit 1. When a path that commit writes holds an uncommitted edit, or an untracked file that `HEAD` doesn't track, it SHALL print `uncommitted <path>` for each such path, change nothing, and exit 1. On a branch that is already that one commit it SHALL commit nothing and leave `HEAD` and its reflog as they were. It then judges the new `HEAD` as for any branch (`judged-by-check`). It SHALL NOT push. It SHALL produce each such tree, the one it compares to name the differing paths included, in a temporary directory that it makes no git repository and in which it runs no git command of its own, as `rename/replay` produces the tree it checks, so `clm rename` reads it as the replay does, and a `GIT_DIR` or `GIT_WORK_TREE` the environment sets, naming the clone, never makes the clone's git directory reinitialized or its index staged in place of that directory.

It SHALL read each revision as the CLM root's tree, as `rename/replay` reads one (`fingerprint-covers-the-clm-root`): the entries under the CLM root, the directory it runs in (`root-from-cwd`), each path relative to it, wherever that root sits in its git repository, so the change it finds, the `change.md` it reads, the `backlog/` files it keeps and the paths it compares and writes are the root's. It SHALL run `clm rename` in the root's directory of each tree it extracts. Outside the root the replay holds the merge base's entries, so each path outside the root where `HEAD`'s entry differs from the merge base's, by path, mode or object ID, SHALL be printed as a differing path, named relative to the root, as the shortest path from the root to it, climbing out with `../`, as the gate names one (`gate/paths-under-the-root`). The commit it makes SHALL hold `origin/main`'s entries outside the root, and each path outside the root where that commit differs from the branch's `HEAD` SHALL be left in the working tree as the commit holds it; no other path outside the root is written.

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

#### Scenario: a rename in a subdirectory root
- GIVEN a git repository whose CLM root is its subdirectory `tool/`, a branch holding the committed result of `clm rename r` run in `tool/`, renaming `a/x` to `a/y`, and an `origin/main` that has since added a test under `tool/` citing `a/x` at its current version and edited `README.md` at the repository's top
- WHEN `clm rebase` runs in `tool/`
- THEN the branch is one commit on `origin/main`, the new test cites `a/y`, `README.md` holds `origin/main`'s text, `git status --porcelain` run in `tool/` prints nothing, its last line is `rebase ok`, and it exits 0

#### Scenario: a file outside a subdirectory root
- GIVEN the branch of *a rename in a subdirectory root* with one more commit editing `README.md` at the repository's top
- WHEN `clm rebase` runs in `tool/`
- THEN it prints `not a rename branch ../README.md`, the branch is unchanged, and it exits 1

#### Scenario: work in the rename's way
- GIVEN the branch and `origin/main` of *main moved under a rename*
- WHEN `clm rebase` runs
- THEN:
  - with an untracked file at the path of the test `origin/main` added, it prints `uncommitted <that path>`, the branch and the file are unchanged, and it exits 1
  - in a subdirectory root `tool/`, with an untracked `notes.md` at the repository's top that `origin/main` adds there, it prints `uncommitted ../notes.md`, the branch and the file are unchanged, and it exits 1

#### Scenario: a second run
- GIVEN the branch of *main moved under a rename*, with a `backlog/d.md` it added, after a `clm rebase` that made it one commit on `origin/main`
- WHEN `clm rebase` runs again
- THEN `HEAD` and `git reflog show HEAD` are as they were, `backlog/d.md` is still there, and its last line is `rebase ok`

#### Needs
- rename/replay
- rename/finish

### Requirement: warnings-name-the-command <!-- base: f6774cd0d48335bc -->
Each warning `clm rebase` prints on stderr SHALL be printed as `clm rebase: <warning>`: that the ledger can't be read, each warning the ledger fold gives and each proposal it skips (`file-proposal-items`), and that it couldn't remove its temporary worktree (`judged-by-check`).

#### Scenario: a skipped proposal
- GIVEN branch `impl/a` and the ledger fold holding an open proposal whose slug is `Not Kebab`
- WHEN `clm rebase` runs
- THEN its stderr holds `clm rebase: skipped proposal` naming that proposal

#### Serves
- architecture/plain-terms

### Requirement: commands-in-crate <!-- base: 77d09cc0fa6479cc -->
`clm-sync` SHALL offer each of `clm rebase` and `clm autosync` as a call of its own, `rebase_command` and `autosync_command`, that takes the command's arguments as the command line gives them, the repository root its caller found, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error; `rebase_command` also takes a call that loads the `health` settings from a CLM root it is given, or says why they can't be loaded, and calls it at most once, as `judged-by-check` states. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help`, each call SHALL write to standard output a help whose first line is the command's description, "Rebase the branch onto origin/main, regenerate applied specs and run `clm check`; exit 1 with what is left to resolve" for `clm rebase` and "Push a snapshot of the spec work on a `spec/*` branch to origin as `autosync/<branch>`" for `clm autosync`, and whose `Usage:` line is `Usage: clm rebase [OPTIONS]` or `Usage: clm autosync [OPTIONS]`, listing `--root`, and SHALL return 0.

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `sync`'s own files
- WHEN it calls `clm-sync`'s call with the arguments of `clm rebase` and two buffers
- THEN the buffers hold what `clm rebase` prints to standard output and to standard error there, and the call returns the status `clm rebase` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm rebase` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: help
- GIVEN `rebase_command` and `autosync_command`
- WHEN each is called with `--help`
- THEN each returns 0 with nothing in the error buffer, and the output buffer's first line is that command's description as stated above, followed by a `Usage:` line naming `clm rebase [OPTIONS]` or `clm autosync [OPTIONS]` and an option `--root`

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

## REMOVED Requirements

### Requirement: git-through-layer <!-- base: 808d5bc4db72ee5f -->

### Requirement: shared-plumbing <!-- base: 0453c1ddcecfc478 -->

### Requirement: agents-md-spec-path <!-- base: 353e8eb7b7fb8cd9 -->
