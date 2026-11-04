---
title: Land
status: active
owners: ["@clunny"]
depends_on: [ledger, host, gate, check, check.affected, config, vcs, model, apply, effort, land.approval, sync.regenerate]
paths: ["crates/clm-land/**"]
successor: ""
serves: [capability/land-safely, architecture/land-locally]
---

## Contract
Landing a branch on `main`, on the machine that runs the command (`land-locally`). `clm land [<branch>]` runs the gate against `origin/main`, merges the branch in a scratch tree, runs the project's check command, handing it the tests the change can break as `clm affected` selects them (`CLM_SELECTION`, `graph-test-selection`), and `clm check` on the result unless the ledger says that code was already checked, records what it did in the ledger, and pushes the merge commit to `main` without force, retrying when `main` moved. It is the only thing in CLM that pushes `main`. What it keeps in a clone, the land tree, the shared target `.clm/land`, its lock and failing checks' logs, is under the `.clm/` of the main worktree's CLM root, which `clm init` has the repository ignore, so a land leaves nothing untracked whatever the project's language (`land-places-ignored`). It prints `landed <sha> <kind>`, `<kind>` being `code`, `spec` or `spec review` as the gate's first line gives it, and exits 0; it exits 1 with the reason when it pushed nothing, and 2 on a git failure. Whether the branch is approved or held by a request for changes, and the signing of approvals and lands, are `land.approval`'s, as is `clm approve`, which `clm-land` re-exports. There is no after-merge review: a land that needs review lands only once approved (`yes-no-review`). `clm reconnect` pushes a `main` landed offline when it fast-forwards `origin/main`, with the local ledgers. `clm land --check-only` runs the same check command and `clm check` on the current working tree, and lands and records nothing, so a session learns the land check's verdict through a command that names no path (`land-check-command`).

## Design
Crate `clm-land`, called from `crates/clm`'s `land`, `approve` (re-exported from `clm-approval`) and `reconnect` subcommands. Every git call goes through `clm-git`, the remote's name taken from `REMOTE` (`git-plumbing-layer`).

**`clm land [<branch>]`**, default the current branch, refused on `main`. Each attempt, up to ten:
1. Fetch `origin` (`main`, the branch, `clm/ledger/*`) and fold the ledger. Take the head, its fingerprint and change IDs from `land.approval`. In a signed repository its key check comes first, then its hold check, then whether the fingerprint is approved under the policy at the fetched base: all before the gate, the lock, the merge and the checks.
2. Run the gate on the head against `origin/main` in the tree this attempt checks in (**Places**, below), adding `--approved` when approved. A violation prints as the gate printed it and exits 1; `gate/approval` tells the caller to ask the human for `clm approve <branch>`.
3. `git merge-tree --write-tree origin/main <head>`. A conflict, or `specs/` changed on both sides, exits 1 telling the caller to run `clm rebase`.
4. The merge commit, `git commit-tree <merged tree> -p origin/main -p <head>`, message `Merge <branch> into main`, with `Change: <id>` on a following line for a spec land that adds or modifies live changes (the first by ID), committed as the repository's git user. A head that already contains `main` is still merged with a merge commit, so a claim's tip reaches `main` only through its land (`merged-claims-released`). The commit is checked out detached into the tree, so the gate and the checks run on a commit. Then **Derived specs** (below).
5. The merged tree's code key (`ledger`). When a `checked` record names it with the same command, only `clm check --no-detectors` runs; otherwise **Selected tests** (below), `[land] check` through `sh -c` with `CI` unset (so a script that installs a browser only in CI doesn't) and `CARGO_TARGET_DIR` set to `<home>/.clm/land` unless the caller set it non-empty, and then `CARGO_INCREMENTAL=0` unless the caller set that non-empty (`shared-land-target`), then `clm check`, and on success a `checked` record. `<main>` is the first worktree `git worktree list --porcelain` lists and `<home>` the CLM root in it, `<main>/<prefix>` with `<prefix>` what `git rev-parse --show-prefix` prints in the CLM root, computed once; a land from a build's worktree under `.claude/worktrees/` so shares one target (`build-places`). The gate then runs again on the merge commit, since a file both sides touched can change whether it needs review, and in a signed repository `land.approval` signs the `landed` record.
6. `git push origin <merge>:refs/heads/main --force-with-lease=refs/heads/main:<the base step 1 fetched>`. A refusal isn't always `main` moving (`land-push-retry`), so it fetches `origin`'s `main` again, directly rather than through step 1's offline refusal, and judges it: the merge commit means the push went through; neither the merge commit nor the base means `main` moved and the next attempt starts, its `checked` record standing, so a retry with the same code key runs only `clm check --no-detectors`; still the base means a refusal for another reason, such as `[remote rejected] main (failure)` while other sessions push, and the same commit is pushed again with the same lease at most twice more, after fixed waits of 1 and then 2 seconds with no override, these pushes not counting as attempts. Still the base after the last is exit 2 with git's message, including the remote's lines (the push drops `--quiet` if it hides them), and a line saying the failure may be transient and that running `clm land <branch>` again is safe. When the refetch fails the push may have gone through, so it exits 2 with the push's message alone. Ten moved attempts exit 1 saying `main` keeps moving.
7. Append, in one commit (`ledger/append-all`), the `effort` records `clm effort record --branch <branch>` would append for the branch's sessions on this machine, read once before the first attempt so neither the reading nor a second ledger push lengthens the time `main` has moved with no `landed` record (`effort-accounted`; the reader's warnings print prefixed `effort: `, and when it can't read at all it warns `effort not recorded: <why>`), then `landed`: commit, branch, `land_kind` and `review` from the gate's first line, fingerprint, the step-5 code key as `code_key`, the change IDs. When `append` reports `landed` unpushed and `host` isn't `none`, it warns `the landed record isn't on origin yet: git push origin clm/ledger/<person>/<clone>` and still exits 0, since `main` has moved. Then `host`'s `branch delete <branch> --lease <head>` for an `impl/*`, `spec/*` or `propose/*` branch (a failure warns), the local branch too when it is checked out nowhere (`clm wrap-up` tidies one that is), and for `impl/*`, **Freeing a build's disk** (below).
8. Print `landed <sha> <kind>`.

**Derived specs** (`derived-specs-fingerprint`). When the head adds a change (`clm-gate`'s `added_changes` over the merge base's and the head's `ls-tree -r -z` listings, the question `gate.fingerprint/fingerprint` asks), or applies one (a change whose `change.md` parses with status `proposed` or `approved` at the base, `origin/main`'s, and in the merge commit has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, as `sync`'s Design defines it: `regenerate-applied-live-changes`), the IDs of the added changes whose `change.md` in the merge commit parses with status `applied` (live) or `done` (new archived), and of the applied ones, sorted, go to `sync.regenerate`'s `regenerated` (crate `clm-regenerate`, which reaches no `clm-health`: `clm/health-isolated`) without its stale-delta merge (`sync.regenerate/regenerated-tree`), run with the tree holding the merge commit as `root` and the fetched base's commit SHA as `onto`, so the scratch `specs/`, the copied `changes/` and `backlog/` and the re-apply are `clm rebase`'s own. A stop exits 1 with rule `land/derived-specs`, the lines it returned and `clm rebase`; otherwise the regenerated `specs/` and the merge commit's are compared byte for byte, a symlink by its target, over the union of their paths in byte order, a path on one side only being a difference, and the first difference exits 1 naming it. It runs before step 5, so before the `--no-detectors` run too. The fingerprint skips `specs/` on a head that adds a change, so this is what keeps an approval from covering a hand edit there. A head that applies a change and adds none keeps `specs/` in its fingerprint, which reads listings alone and can't see a status, so a sync that regenerates its `specs/` onto a `main` that changed them asks for its approval again.

**Places** (`scratch-outside-git`, `land-tree`, `fixed-land-tree`). Cargo names a workspace's outputs by its path and judges them fresh by file times, so two lands that overlap on one target can check each other's code, and a fresh path per land rebuilt everything (43 GB a day on one machine). Before step 2 an attempt computes the merged tree and its code key (`git merge-tree --write-tree` writes only objects); when its check will compile into `<home>/.clm/land`, it takes the exclusive advisory lock on `<home>/.clm/land.lock` (`land-target-lock`, `<home>/.clm/` created when missing, released by the system when a land is killed), printing `waiting for another land's check on <home>/.clm/land` on stderr once when it waits, with no time limit, and holds it until the push has gone through and the cap is applied, or the attempt ends; the append, the deletes and the print run outside it. **The cap** (`land-target-cap`): right after a push that went through, still under the lock, it sums the lengths of the regular files under `.clm/land` (no symlink followed, so a sparse file counts its length) and, above `[land] target_cap_gb` gigabytes, removes the directory and prints `cleared .clm/land (<n> GB, cap <cap> GB)`; an unreadable entry or a failed removal only warns. Incremental data and units for feature sets no land builds again are what grow there, and Cargo prunes neither, so the cap trades one cold land for a bounded directory, and each attempt takes it afresh. A merge that conflicts, a checked code key and a caller's `CARGO_TARGET_DIR` take no lock. A locked attempt checks out in the land tree `<home>/.clm/land-tree` (checks of a root in a subdirectory run in `<home>/.clm/land-tree/<prefix>`): when `git worktree list --porcelain` lists it (both canonicalised) and it exists, `git checkout --quiet --force --detach <commit>` and then `git clean --quiet -fdx`; otherwise, or when either fails, it removes the path, runs `git worktree prune` and `git worktree add --quiet --detach <path> <commit>`, a failed add being exit 2. The tree stays when the attempt ends; `clm land` and `clm wrap-up` never remove it, and removing it by hand starts the next land cold. Any other attempt uses a fresh `land-*` worktree under `TMPDIR` (read through the injected environment lookup) or else `std::env::temp_dir()`, removed on every exit path, never one under `.claude/worktrees/`, and outside the repository, so no tool that refuses paths under `.git/` (Vite's `server.fs.deny`) or walks up to project files trips on it; a killed land's tree stays until the system clears it.

**Its own binary** (`land-binary-copy`). A check that rebuilds `clm` can replace the file `clm land` runs from. So before its first subcommand it copies `std::env::current_exe` into a fresh `land-bin-*` directory beside its temporary worktrees, keeping permissions, and runs the gate, `clm affected` and `clm check` from the copy, removed on every exit path; when the copy can't be made it exits 1 naming the path. The lander's binary is `main`'s rule, which the merged tree lives under. `CLM_GATE`, `CLM_CHECK` and `CLM_AFFECTED` override the three commands, run as given, so tests fake them.

**Selected tests** (`graph-test-selection`). Before `[land] check` it runs `clm affected --base <sha>`, `<sha>` the fetched base's commit (the merge commit's merge base with it, so the selection is what the branch brings), in the tree on the merge commit and under the lock when the check holds it, and writes its stdout unchanged to `selection` in a temporary directory, which `CLM_SELECTION` names. When it fails, `clm affected failed: <the first non-empty line of its stderr, or the spawn error>; the check runs every test`, and the check runs without `CLM_SELECTION`: a failed selection costs time, never a skipped test.

**The check's output** (`land-check-log`). One pipe (`std::io::pipe`) carries `[land] check`'s stdout and stderr in the order written, copied to stderr and to `<home>/.clm/land-logs/<branch>.log`, the branch's slashes as directories, created and truncated first; the last 40 lines (split on `\n`) are kept in memory. It reads until the pipe closes, so a check must end what it starts. A failing check's refusal, printed last as `clm land: <why>`, is `last 40 lines of `<check>`:`, those lines and `` `<check>` fails on the merge with main; its output is at <path> ``, and the log stays; a passing check removes it. A log that can't be created warns naming the path, and the refusal then has no `; its output is at` clause. `clm check`'s output isn't logged.

**Freeing a build's disk** (`land-frees-build-target`). After the push and the append of an `impl/*` land it deletes the `target/` at `git rev-parse --show-toplevel` when that is a linked worktree whose `HEAD` is the landed branch, printing `removed <path>`; a missing one is skipped and a failed delete warns. `clm run --finish` removes the worktree itself (`orchestrator.run`).

**`--check-only`** (`land-check-command`). Given a branch as well it exits 1 (by hand: clap's `conflicts_with` would exit 2) and runs nothing. On any branch, in the CLM root (`--root`, else found from the current directory), it runs `[land] check` as step 5 does but with `CARGO_TARGET_DIR` as the caller has it, so a build compiles into its own `target/`, then `clm check` with its detectors (`CLM_CHECK` overrides it), on the working tree as it stands, output on stderr. It prints `checks pass`; otherwise exits 1 with `` `<command>` fails `` or `` `clm check` fails ``, the failure messages being a parameter of the helpers a land shares; a `clm.toml` that doesn't load is exit 1 with `config`'s message. It reads no ledger and a working tree has no code key, so it never lets a land skip its check. It exists for a build's pre-land check (`orchestrator.run`), which Claude Code's worktree isolation refused as a script path.

**Offline** (`no-local-land-with-remote`). With `host = "none"` the base is the local `main`, there is no fetch or push, the records stay local, and it prints `landed <sha> <kind> (local)`. With any other host `main` has one copy, the remote's, so a failed step-1 fetch exits 1 with `offline: origin can't be reached; push and land when it can`. **`clm reconnect`**, for a repository that gains a remote, pushes `main` with a lease when `origin/main` is missing or an ancestor, then every local `clm/ledger/*` branch, and prints `reconnected <n> lands`; when `origin/main` has moved it pushes only the ledgers, prints `land again <branch>` for each local land (the merge commit's second parent) and exits 1.

## Requirements

### Requirement: land
`clm land [<branch>]` SHALL fetch `origin`, run the gate on the branch's head against `origin/main` with `--approved` when the ledger holds an `approved` record for the head's fingerprint that the `approval` policy accepts, merge the head into `origin/main` in a scratch tree, run `[land] check` and then `clm check` on the merged tree unless a `checked` record names its code key with the same command (then `clm check --no-detectors` only), run the gate on the merged tree, make a merge commit of the head into `origin/main`, push it to `main` without force and with a lease on the fetched `origin/main`, append `landed` naming the merged tree's code key and whether the land needed review, and print `landed <sha> <kind>`. When the push is refused because `main` moved it SHALL start again from the fetch, up to ten attempts. It SHALL push nothing and exit 1, naming why, when the gate blocks, the merge conflicts, a check fails, or every attempt is refused; and SHALL append `checked` whenever the check command and `clm check` pass on a tree, even when the push is later refused.

#### Scenario: code branch lands
- GIVEN `origin/main`, a branch `impl/a` with a passing code change, no `checked` record, and fake gate and check commands that record the tree they ran on
- WHEN `clm land impl/a` runs
- THEN `main` on `origin` is a merge commit whose parents are the old `main` and the head, the check command ran on that commit's tree, the ledger has `checked` for its code key and `landed` for the commit with `land_kind` `code` and that code key, and it printed `landed <sha> code`

#### Scenario: already checked code
- GIVEN a branch that changes only `specs/x/SPEC.md`, and a `checked` record for `origin/main`'s code key with the configured command
- WHEN `clm land` runs
- THEN the check command never ran, `clm check --no-detectors` ran on the merged tree, and the branch landed

#### Scenario: unapproved branch that needs review
- GIVEN a `spec/x` branch whose gate prints `spec review` and no `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `gate/approval` and naming `clm approve spec/x`, and `main` on `origin` is unchanged

#### Scenario: approved branch that needs review
- GIVEN the same branch and an `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it prints `landed <sha> spec review`, and the `landed` record has `land_kind` `spec`, `review` true and `changes` `[x]`

#### Scenario: stale approval
- GIVEN an `approved` record for fingerprint `f`, and a branch whose fingerprint is now `g` after a content change
- WHEN `clm land` runs
- THEN it exits 1 printing `gate/approval`

#### Scenario: main moves
- GIVEN a check command that, the first time it runs, pushes another commit to `origin/main`
- WHEN `clm land` runs
- THEN it starts again, `main` on `origin` ends as a merge commit whose parents are that commit and the head, and the check command's last run was on that merge's tree

#### Scenario: conflict
- GIVEN a branch that conflicts with `origin/main`
- WHEN `clm land` runs
- THEN it exits 1 naming `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: check fails
- GIVEN a check command that fails
- WHEN `clm land` runs
- THEN it exits 1, `main` on `origin` is unchanged, and the ledger has no `checked` or `landed` record for the attempt

#### Scenario: from main
- GIVEN the checkout on `main`
- WHEN `clm land` runs with no argument
- THEN it exits 1 saying `main` can't be landed

#### Needs
- clm/renamed-commands

### Requirement: landed-branch-cleanup
After a successful push, `clm land` SHALL delete the landed branch on `origin` through `host`'s `branch delete`, leased on the head it landed, when the branch is `impl/*`, `spec/*` or `propose/*`, warn without failing when that delete fails, and leave any other branch.

#### Scenario: impl branch removed
- GIVEN a landed `impl/a` whose tip on `origin` is the head it landed
- WHEN `clm land impl/a` finishes
- THEN `origin` has no `impl/a` and it exits 0

#### Scenario: moved head kept
- GIVEN a check command that pushes a new commit to `origin/impl/a` while the land runs
- WHEN `clm land impl/a` finishes
- THEN `origin/impl/a` is still that commit, it printed a warning, and it exits 0

### Requirement: offline-land
With `host` set to `none`, `clm land` SHALL land on the local `main` with the same gate and checks, push nothing, append to the local ledger, and print `landed <sha> <kind> (local)`. With any other `host`, when `origin` can't be reached, `clm land` SHALL exit 1 saying it is offline and to land when `origin` is reachable, land nothing locally and append nothing. `clm reconnect` SHALL push `main` when `origin/main` doesn't exist or is an ancestor of it, then every local ledger branch, and print `reconnected <n> lands`; when `origin/main` has moved it SHALL push only the ledgers, print `land again <branch>` for each local land's branch, and exit 1.

#### Scenario: no remote lands locally
- GIVEN a repository with `host = "none"`, no `origin`, and a passing code branch
- WHEN `clm land` runs
- THEN the local `main` is a merge commit of the branch, the local ledger has `landed`, and it printed `(local)`

#### Scenario: offline with a remote refuses
- GIVEN a clone with `host = "github"` whose `origin` URL points nowhere, and a passing code branch
- WHEN `clm land` runs
- THEN it exits 1 saying it is offline, the local `main` is unchanged, and the ledger has no new record

#### Scenario: first push after adding a remote
- GIVEN the repository of *no remote lands locally*, now with an `origin` that has no `main`
- WHEN `clm reconnect` runs
- THEN `origin/main` is the local `main`, the ledger branch is on `origin`, and it printed `reconnected 1 lands`

#### Scenario: origin moved
- GIVEN the same, but `origin` already has a `main` that isn't an ancestor of the local one
- WHEN `clm reconnect` runs
- THEN `origin/main` is unchanged, the ledger branch is on `origin`, it printed `land again` with the branch, and it exited 1

#### Serves
- capability/git-host
- architecture/no-local-land-with-remote

### Requirement: git-through-layer
`clm-land` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `run` or repo queries (`git-plumbing-layer`), and host operations through `host`.

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-land` reads `crates/clm-land/Cargo.toml` and every file under `crates/clm-land/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Serves
- architecture/reuse-first

### Requirement: scratch-outside-git
`clm land` SHALL merge and check in a worktree no component of whose path is `.git`: the clone's land tree, `.clm/land-tree` in the CLM root of the main worktree, when the attempt holds the lock on `.clm/land.lock` (`land-tree`), else a temporary worktree under the system's temporary directory, which it SHALL remove on every exit path.

#### Scenario: the check runs outside .git
- GIVEN a repository whose `[land] check` prints its working directory
- WHEN `clm land` runs on a branch
- THEN the directory it printed is `.clm/land-tree` in the main worktree (both compared canonicalised) and has no `.git` component

#### Scenario: removed after a failing check
- GIVEN a repository whose `[land] check` exits 1, and an empty `TMPDIR` of the test's own
- WHEN `clm land` runs on a branch
- THEN it exits 1, `TMPDIR` is empty again, and `git worktree list` names no worktree but the main one and the land tree

#### Scenario: a land that compiles nothing uses a temporary worktree
- GIVEN a branch that changes only `specs/x/SPEC.md`, a `checked` record for `origin/main`'s code key with the configured command, and a fake `CLM_CHECK` that records its working directory
- WHEN `clm land` runs on it
- THEN the directory recorded is under `TMPDIR`, not `.clm/land-tree`, and `TMPDIR` is empty again afterwards

### Requirement: shared-land-target
When the caller hasn't set `CARGO_TARGET_DIR`, or set it empty, `clm land` SHALL run `[land] check` with `CARGO_TARGET_DIR` set to `.clm/land` in the CLM root of the repository's main worktree (the first worktree `git worktree list --porcelain` lists), whichever worktree of the repository it runs in, and SHALL hold the clone's lock on `.clm/land.lock` there (`land-target-lock`) while it does. Such a check SHALL also run with `CARGO_INCREMENTAL` set to `0` unless the caller set `CARGO_INCREMENTAL` non-empty, since incremental data in a target shared by every land only grows; a check whose caller set `CARGO_TARGET_DIR` SHALL get `CARGO_INCREMENTAL` as the caller has it.

#### Scenario: land from a linked worktree
- GIVEN a repository whose `[land] check` records `CARGO_TARGET_DIR`, and a linked worktree of it with a branch checked out
- WHEN `clm land` runs in the linked worktree with `CARGO_TARGET_DIR` unset
- THEN the check saw `CARGO_TARGET_DIR` set to the main worktree's `.clm/land`, not the linked worktree's

#### Scenario: CLM root in a subdirectory
- GIVEN a git repository whose CLM root is its subdirectory `tool/`, and a `[land] check` that records `CARGO_TARGET_DIR`
- WHEN `clm land` runs on a branch with `CARGO_TARGET_DIR` unset
- THEN the check saw `CARGO_TARGET_DIR` set to the main worktree's `tool/.clm/land`

#### Scenario: incremental off in the shared target
- GIVEN a `[land] check` that records `CARGO_INCREMENTAL`
- WHEN `clm land` runs with `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL` unset
- THEN the check saw `CARGO_INCREMENTAL` set to `0`

#### Scenario: caller's incremental kept
- GIVEN the same check
- WHEN `clm land` runs with `CARGO_TARGET_DIR` unset and `CARGO_INCREMENTAL=1`, and then with `CARGO_TARGET_DIR` set to a directory of the caller's and `CARGO_INCREMENTAL` unset
- THEN the first check saw `CARGO_INCREMENTAL` set to `1` and the second saw it unset

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: land-target-lock
A `clm land` whose `[land] check` will run with `CARGO_TARGET_DIR` set to the main worktree's `.clm/land` SHALL hold an exclusive advisory lock on `.clm/land.lock` beside it from before it checks the head out into the land tree until the push of the merge commit has gone through and the target's cap has been applied (`land-target-cap`), or the attempt ends sooner, so that the gate on the merge commit, `clm check`, the push and the cap run under it, releasing it on every exit path. A land that waits for the lock SHALL say so on stderr, naming `.clm/land`. A land that runs no `[land] check`, because its code key was already checked or no check is set, or whose caller set `CARGO_TARGET_DIR`, SHALL take no lock.

#### Scenario: overlapping lands
- GIVEN two branches `impl/a` and `impl/b` of one clone, no `checked` records, and a `[land] check` that appends `start <branch>` to a log, sleeps 2 seconds and appends `end <branch>`
- WHEN `clm land impl/a` and `clm land impl/b` start at once, from two linked worktrees
- THEN in the log every `start <branch>` is followed by the `end` of that same branch before any other `start`, so the two checks never interleave, and the land that waited printed `waiting` naming `.clm/land` on stderr

#### Scenario: held through the gate on the merge
- GIVEN the same two branches, a `[land] check` that appends `check <branch>` to a log and sleeps 2 seconds, and a fake `CLM_GATE` that appends `gate` to it and passes
- WHEN `clm land impl/a` and `clm land impl/b` start at once
- THEN the log reads `gate`, `check <one branch>`, `gate` and then `gate`, `check <the other>`, `gate`: both of a land's gate runs fall between its lock's taking and release, never inside the other land's

#### Scenario: checked code takes no lock
- GIVEN a branch that changes only `specs/x/SPEC.md`, a `checked` record for `origin/main`'s code key, and another process holding the lock
- WHEN `clm land` runs
- THEN it lands without waiting and prints no `waiting` line

#### Scenario: lock released after a failing check
- GIVEN a `[land] check` that exits 1
- WHEN `clm land` runs and then a second `clm land` runs
- THEN the first exits 1 and the second doesn't print `waiting`

#### Serves
- capability/work-concurrently
- capability/land-safely

### Requirement: land-binary-copy
Before running its first subcommand, `clm land` SHALL copy its own binary into a fresh directory under the system's temporary directory and run the gate and `clm check` from that copy, unless `CLM_GATE` or `CLM_CHECK` names the command; it SHALL remove the copy on every exit path. When the copy can't be made it SHALL exit 1 naming the path, pushing and recording nothing.

#### Scenario: check deletes the binary
- GIVEN a `clm` binary copied to a scratch path, a repository whose `[land] check` deletes that scratch path, and `CLM_GATE` and `CLM_CHECK` unset
- WHEN the scratch `clm land` runs on a branch
- THEN it lands and prints `landed <sha> code`

#### Scenario: copy removed
- GIVEN an empty `TMPDIR` of the test's own
- WHEN `clm land` runs on a branch and lands
- THEN `TMPDIR` is empty again

### Requirement: land-frees-build-target
After a successful push of an `impl/*` branch, `clm land` SHALL delete the `target/` directory at the top of the worktree it runs in when that worktree is a linked one whose `HEAD` is the landed branch, and print `removed <path>`. It SHALL NOT delete the main worktree's `target/`, delete anything after a `spec/*` or `propose/*` land or a land that pushed nothing, or fail the land when the delete fails.

#### Scenario: build's target freed
- GIVEN a linked worktree with `impl/a` checked out and a `target/` directory holding a file, and a passing check
- WHEN `clm land impl/a` runs in that worktree
- THEN it exits 0, the worktree's `target/` is gone, the worktree itself is still there, and it printed `removed` with the `target/` path

#### Scenario: main worktree kept
- GIVEN the main worktree with `impl/a` checked out and a `target/` directory holding a file
- WHEN `clm land impl/a` runs there
- THEN it exits 0 and `target/` and its file are still there

#### Scenario: spec land kept
- GIVEN a linked worktree with `spec/y` checked out and a `target/` directory
- WHEN `clm land spec/y` lands
- THEN `target/` is still there

#### Scenario: failed land kept
- GIVEN a linked worktree with `impl/a` checked out, a `target/` directory, and a `[land] check` that exits 1
- WHEN `clm land impl/a` runs
- THEN it exits 1 and `target/` is still there

#### Scenario: other branch checked out
- GIVEN a linked worktree with `spec/z` checked out and a `target/` directory, and a pushed branch `impl/a` with a passing check
- WHEN `clm land impl/a` runs in that worktree
- THEN it lands and `target/` is still there

#### Serves
- capability/wrap-up-session

### Requirement: land-push-retry
When the push of the merge commit to `main` is refused, `clm land` SHALL fetch `origin`'s `main` again and judge it: when it is the merge commit, it SHALL treat the push as done and go on to append `landed`; when it is neither the merge commit nor the fetched base, it SHALL start the next attempt (`land`); when it is still the base, it SHALL push the same merge commit again with the same lease, at most twice more in that attempt, waiting 1 second before the first of these and 2 seconds before the second, judging each refusal the same way, without counting these pushes as attempts. When `main` is still at the base after the last push it SHALL exit 2 with git's message for the refused push, including what the remote printed, and a line saying the failure may be transient and that running `clm land <branch>` again is safe, having appended no `landed` record. When the fetch after a refusal fails it SHALL exit 2 with the refused push's git message and no such line. When `append` reports the `landed` record unpushed, with a `host` other than `none`, it SHALL warn naming the writer's ledger branch `clm/ledger/<person>/<clone>` and `git push origin` with that branch, and exit 0.

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- ledger/append-refused-push

#### Scenario: one transient refusal
- GIVEN a passing code branch `impl/a`, and a `pre-receive` hook on `origin` that logs each push naming `refs/heads/main`, declines the first with `declined by test` and accepts the next
- WHEN `clm land impl/a` runs
- THEN the hook logged two pushes to `main`, the check command ran once, `main` on `origin` is the merge commit of the head, the ledger has one `landed` record for it, and it exits 0 printing `landed <sha> code`

#### Scenario: refused every time
- GIVEN a passing code branch `impl/a`, and a `pre-receive` hook on `origin` that logs the time of each push naming `refs/heads/main` and declines it with `declined by test`, accepting every other push
- WHEN `clm land impl/a` runs
- THEN the hook logged three pushes to `main`, the second at least 1 second after the first and the third at least 2 seconds after the second, it exits 2, its message contains `declined by test`, `may be transient` and `clm land impl/a`, and not `keeps moving`, `main` on `origin` is what it was before the land, and the ledger has one `checked` record from this land and no `landed` record for it

#### Scenario: rerun after giving up
- GIVEN the repository of *refused every time*, with the hook then removed
- WHEN `clm land impl/a` runs again
- THEN the check command didn't run again, and it lands, printing `landed <sha> code`

#### Scenario: pushed though git failed
- GIVEN a passing code branch `impl/a`, and a script named `git` in a directory put first on `PATH` that runs the real `git` and, for a `push` naming `refs/heads/main`, then exits 1 whatever the real one did; the test runs the `clm` binary as a child process with that `PATH` (`crates/clm/tests`), since the in-process `clm_land::land` takes the process's own `PATH`
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is that merge commit, whose second parent is the head, and the ledger has one `landed` record for it

#### Scenario: ledger push refused
- GIVEN a passing code branch `impl/a`, and a `pre-receive` hook on `origin` that declines every push naming a `refs/heads/clm/ledger/` ref and accepts every other push
- WHEN `clm land impl/a` runs
- THEN it exits 0 printing `landed <sha> code`, `main` on `origin` is the merge commit, the local ledger branch has the `landed` record, and it warned naming that ledger branch and `git push origin`

### Requirement: selected-tests
When `clm land` runs `[land] check`, it SHALL first run `clm affected --base <sha>`, `<sha>` being the commit of the `origin/main` it fetched (the local `main`'s with no remote), in the worktree it checks, write its stdout unchanged to a file in its temporary directory, and run `[land] check` with `CLM_SELECTION` set to that file's path. When `clm affected` fails, it SHALL print `clm affected failed: <reason>; the check runs every test` on one line of stderr, `<reason>` being the first non-empty line of its stderr, trimmed, or the spawn error, and run `[land] check` with `CLM_SELECTION` unset. A land that runs no check SHALL run no selection.

#### Scenario: selection handed over
- GIVEN a branch that changes one component's code, a `[land] check` that copies the file `CLM_SELECTION` names, and `CLM_AFFECTED` set to a command that prints the component line of `a` and `file crates/a/src/lib.rs`
- WHEN `clm land` runs
- THEN the copied file holds those two lines, and the branch lands

#### Scenario: base is the fetched main
- GIVEN the same branch and a `CLM_AFFECTED` command that records its arguments
- WHEN `clm land` runs
- THEN the command was given `--base` and the commit `origin/main` named when the land fetched

#### Scenario: selection fails
- GIVEN the same branch and `CLM_AFFECTED` set to a command that exits 1
- WHEN `clm land` runs
- THEN it prints `clm affected failed:` on stderr, runs `[land] check` with no `CLM_SELECTION`, and the branch lands

#### Scenario: code already checked
- GIVEN a branch whose merged tree's code key has a `checked` record with the same command
- WHEN `clm land` runs
- THEN it runs neither `clm affected` nor `[land] check`

### Requirement: check-only
`clm land --check-only` SHALL run, in the working tree of the CLM root found from the current directory, `[land] check` when `clm.toml` sets it, the way a land runs it (through `sh -c`, with `CI` unset, and `CARGO_TARGET_DIR` left as the caller's environment has it, so a build's check compiles into its worktree's own `target/` (`build-places`)), and then `clm check`. It SHALL print `checks pass` and exit 0 when both pass; otherwise it SHALL exit 1 naming the command that failed, and SHALL NOT run `clm check` after a failing check command. It SHALL fetch nothing, append no ledger record, make no commit and push nothing. Given a branch as well, it SHALL exit 1 without running either command. When `clm.toml` can't be loaded, it SHALL exit 1 with the reason, running neither command.

#### Scenario: checks pass
- GIVEN a repository whose `clm.toml` sets `[land] check` to a fake that records its working directory and whether `CI` is set, a fake `CLM_CHECK` that passes, and an uncommitted edit in the working tree
- WHEN `clm land --check-only` runs there with `CI=true`
- THEN the fake ran in that working tree with `CI` unset, `clm check` ran after it, it printed `checks pass` and exited 0, and `HEAD`, `origin/main` and the ledger are unchanged

#### Scenario: check command fails
- GIVEN the same repository with a fake check command that fails
- WHEN `clm land --check-only` runs
- THEN it exits 1 naming the check command, and `clm check` never ran

#### Scenario: no check command
- GIVEN a repository whose `clm.toml` has no `[land]` table and a fake `CLM_CHECK` that passes
- WHEN `clm land --check-only` runs
- THEN only `clm check` ran, and it printed `checks pass` and exited 0

#### Scenario: with a branch
- GIVEN the same repository
- WHEN `clm land --check-only impl/a` runs
- THEN it exits 1 and neither the check command nor `clm check` ran

#### Scenario: unreadable settings
- GIVEN a repository whose `clm.toml` isn't valid TOML
- WHEN `clm land --check-only` runs
- THEN it exits 1 naming `clm.toml`, and neither the check command nor `clm check` ran

### Requirement: shared-plumbing
`clm-land` SHALL take the remote's name from `clm-git`'s `REMOTE` wherever it names the remote to git (a fetch, a push, a ref such as `refs/remotes/<remote>/main` or `<remote>/main`, the check that the remote exists), building each such argument from `REMOTE` rather than spelling it: no line of a file under `crates/clm-land/src`, apart from those whose first non-blank characters are `//`, SHALL hold `"origin"`, `"origin/` or `remotes/origin` (`remote-plumbing-scan`). Messages for a person or a session that name `origin`, such as ``origin's `main` has moved: ...``, ``no `origin`: add the remote, then reconnect`` and `offline: origin can't be reached; push and land when it can`, SHALL keep their wording. `clm land`, `clm approve` and `clm reconnect` SHALL keep their output and exit codes.

#### Scenario: no copy of the remote's name
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`, skipping each line whose first non-blank characters are `//`
- THEN no line holds `"origin"`, `"origin/` or `remotes/origin`

#### Scenario: messages keep naming origin
- GIVEN the workspace
- WHEN the same test reads `crates/clm-land/src/lib.rs` and `crates/clm-land/src/reconnect.rs`
- THEN `lib.rs` still holds `origin can't be reached` and `reconnect.rs` still holds ``origin's `main` has moved``

#### Serves
- architecture/reuse-first

### Requirement: derived-specs
When the head adds a change, as `gate.fingerprint/fingerprint` defines it (`clm-gate`'s `adds_change` over the merge base's and the head's listings), or applies one, a change whose `changes/<id>/change.md` has status `proposed` or `approved` at the base and, in the merge commit, has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, `clm land` SHALL, after the merge commit and before any check, regenerate `specs/` with `sync.regenerate`'s `regenerated`, without its stale-delta merge, onto the base (`origin/main`'s, or the local `main`'s when landing offline), from the CLM root in the tree holding the merge commit, for the added changes whose status is `applied`, or `done` for a new archived one, and the applied ones, in ID order, a `change.md` that doesn't parse regenerating nothing, and compare the result with the merge commit's `specs/` byte for byte over the union of both sides' paths in byte order, a path on one side only being a difference. When they differ, or `regenerated` stops, it SHALL push nothing and exit 1 with rule `land/derived-specs`, naming the first differing path or the lines `regenerated` returned, and `clm rebase`. A head that adds and applies no change SHALL land without the comparison, since its `specs/` edits are its own. No file under `crates/clm-land/src` SHALL call `clm_apply::reapply` or `clm_model::load`: the regenerate is `sync.regenerate`'s alone. `clm-land` SHALL call it from crate `clm-regenerate`, never through `clm-sync`, which reaches `clm-health` (`clm/health-isolated`).

#### Scenario: regenerated branch lands
- GIVEN `spec/x` adding applied change `x`, whose delta adds requirement `a/r`, with `specs/` as `clm rebase` regenerates it, and an `origin/main` that has since added requirement `b/s`
- WHEN `clm land spec/x` runs
- THEN it lands, and `main`'s `specs/a/SPEC.md` holds `a/r` and `specs/b/SPEC.md` holds `b/s`

#### Scenario: applied change's deltas lost
- GIVEN an `origin/main` holding change `y` with `status: proposed` and a delta adding requirement `a/x`, and `spec/z` that adds change `z`, which has no delta, and sets `y` to `applied`, with `specs/` as `origin/main`'s, so `a/x` is missing
- WHEN `clm land spec/z` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, and `main` on `origin` is unchanged

#### Scenario: hand edit under specs refused
- GIVEN the same branch with one more commit changing a line of `specs/a/SPEC.md` that no delta of `x` holds, and an `approved` record for its fingerprint
- WHEN `clm land spec/x` runs
- THEN it exits 1 printing `land/derived-specs`, naming `specs/a/SPEC.md` and `clm rebase`, the check command never ran, and `main` on `origin` is unchanged

#### Scenario: branch that adds no change
- GIVEN `impl/a`, which adds no change and sets `status: active` in `specs/a/SPEC.md`
- WHEN `clm land impl/a` runs
- THEN it lands with that edit

#### Scenario: no copy of the regenerate
- GIVEN the workspace
- WHEN a test in `clm-land` reads every file under `crates/clm-land/src`
- THEN no file holds `clm_apply::reapply` or `clm_model::load`

#### Serves
- capability/catch-drift
- architecture/regenerate-dont-merge
- architecture/humans-review-intent

#### Needs
- clm/renamed-commands
- gate.fingerprint/fingerprint
- sync.regenerate/regenerated-tree
- land.approval/approve

### Requirement: land-tree
An attempt of `clm land` that holds the lock on `.clm/land.lock` SHALL check the head and the merge commit out in `.clm/land-tree` in the CLM root of the main worktree, a detached worktree of the repository that it SHALL keep when the attempt ends. Before each checkout it SHALL make that tree exist and clean: when git lists the path as a worktree and its directory exists, `git checkout --force --detach` there and then `git clean -fdx`; otherwise, or when either fails, it SHALL remove whatever is at the path, prune git's worktree list and add the worktree afresh. Everything a land keeps in the main worktree, the land tree, `.clm/land`, `.clm/land.lock` and `.clm/land-logs/`, SHALL be under that CLM root's `.clm/`, so that a repository whose `.gitignore` lists only the entries `clm init` adds (`init/init-ignores-local-settings`) shows none of it in `git status`.

#### Scenario: one tree for every land
- GIVEN two branches `impl/a` and `impl/b`, no `checked` records, and a `[land] check` that records its working directory
- WHEN `clm land impl/a` runs and then `clm land impl/b`
- THEN both checks ran in the main worktree's `.clm/land-tree`, and `git worktree list` names it afterwards, detached

#### Scenario: leftovers cleaned
- GIVEN the same branches and a `[land] check` that exits 1 when a file `left` exists in its working directory and otherwise writes it
- WHEN `clm land impl/a` runs and then `clm land impl/b`
- THEN both land

#### Scenario: tree repaired after .clm is removed
- GIVEN a land of `impl/a` that made the land tree, and the main worktree's `.clm/` then removed
- WHEN `clm land impl/b` runs
- THEN it lands and its check ran in `.clm/land-tree`

#### Scenario: stale directory replaced
- GIVEN a directory at the main worktree's `.clm/land-tree`, holding a file, that git lists no worktree for
- WHEN `clm land impl/a` runs
- THEN it lands, its check ran in `.clm/land-tree`, and the file is gone

#### Scenario: nothing untracked in a project that doesn't ignore target
- GIVEN a repository whose committed `.gitignore` lines are `clm.local.toml`, `.clm/` and `.claude/worktrees/` and nothing else, a branch `impl/a` with a passing `[land] check` that writes a file into `CARGO_TARGET_DIR`, and a branch `impl/b` whose `[land] check` exits 1
- WHEN `clm land impl/a` runs and then `clm land impl/b`
- THEN the first lands, the second exits 1, and `git status --porcelain` in the main worktree prints nothing, with no `target/` directory there

### Requirement: check-log
`clm land` SHALL write `[land] check`'s stdout and stderr, in the order produced, to `.clm/land-logs/<branch>.log` in the CLM root of the main worktree while also printing them to stderr. When the check fails its refusal SHALL hold, under a line naming the check, the last 40 lines of that output, kept in memory, and end with a line naming the log's path, and it SHALL keep the log; when the check passes it SHALL remove the log. When the log can't be created it SHALL warn naming the path and run the check all the same, and a failing check's refusal then names no path.

#### Scenario: failing check's tail and log
- GIVEN a `[land] check` that prints the lines `1` to `100`, the odd ones to stdout and the even ones to stderr, and exits 1
- WHEN `clm land impl/a` runs
- THEN it exits 1, its stderr holds a line naming the check followed by the lines `61` to `100`, the refusal names the main worktree's `.clm/land-logs/impl/a.log`, and that file holds the lines `1` to `100` in order

#### Scenario: short output repeated whole
- GIVEN a `[land] check` that prints three lines and exits 1
- WHEN `clm land impl/a` runs
- THEN its stderr holds the three lines after the line naming the check

#### Scenario: passing check leaves no log
- GIVEN a passing `[land] check`
- WHEN `clm land impl/a` runs
- THEN it lands and the main worktree has no `.clm/land-logs/impl/a.log`

#### Scenario: log can't be created
- GIVEN a file, not a directory, at the main worktree's `.clm/land-logs`
- WHEN `clm land impl/a` runs with a passing check
- THEN it warns naming the log's path, the check ran, and the branch landed

### Requirement: records-effort
After the push to `main`, `clm land` SHALL append, in the commit that appends `landed` and before it, the `effort` records `clm effort record --branch <branch>` would append for the landed branch's sessions on this machine, read once before its first attempt from the transcripts directory its environment gives (`effort/record-command`). When they can't be read it SHALL warn, naming why, and land as before.

#### Scenario: sessions recorded
- GIVEN a transcript of a session on `impl/a` run in the repository
- WHEN `clm land impl/a` lands it
- THEN the fold holds an `effort` record for `impl/a`, written in the commit that writes the `landed` record and before its line, and the last line printed is `landed <sha> code`

#### Scenario: no transcripts
- GIVEN no transcripts directory
- WHEN `clm land impl/a` runs
- THEN it lands, prints `landed <sha> code`, appends no `effort` record and warns `effort: no transcripts at <dir>`

#### Scenario: push retried
- GIVEN a transcript on `impl/a`, and `origin` declining the first push to `main`
- WHEN `clm land impl/a` lands it on a later push
- THEN the ledger holds exactly one `effort` record, in the commit that writes the `landed` record

#### Scenario: push never taken
- GIVEN a transcript on `impl/a`, and `origin` declining every push to `main`
- WHEN `clm land impl/a` gives up
- THEN the ledger holds no `effort` record

#### Scenario: effort unreadable
- GIVEN a checkout `clm effort record` refuses
- WHEN `clm land impl/a` runs
- THEN it lands, appends `landed`, and warns `effort not recorded: <why>`

#### Serves
- capability/review-capability-cost
- architecture/effort-accounted

### Requirement: land-target-cap
After the push of a land whose check ran with `CARGO_TARGET_DIR` set to the main worktree's `.clm/land`, and before it releases the lock on `.clm/land.lock`, `clm land` SHALL sum the lengths of the regular files under `.clm/land`, following no symbolic link, and when the sum is greater than `[land] target_cap_gb` gigabytes (10^9 bytes each) SHALL remove `.clm/land` and print on stderr `cleared .clm/land (<n> GB, cap <cap> GB)`, `<n>` the sum in whole gigabytes rounded down. When any entry can't be read the measure fails and nothing is removed; a failure to measure or remove SHALL print a warning naming the path and leave the land's outcome unchanged. A land that took no lock SHALL neither measure nor remove it.

#### Scenario: over the cap
- GIVEN a `clm.toml` with `[land]`, a `check` that creates a sparse file of 2,000,000,000 bytes in `CARGO_TARGET_DIR`, and `target_cap_gb = 1`, and a branch `impl/a`
- WHEN `clm land impl/a` runs with `CARGO_TARGET_DIR` unset
- THEN it prints `landed`, stderr holds `cleared .clm/land (2 GB, cap 1 GB)`, and the main worktree's `.clm/land` doesn't exist

#### Scenario: under the cap
- GIVEN the same, with a check that creates a file of 10 bytes and the cap left at its default
- WHEN `clm land impl/a` runs with `CARGO_TARGET_DIR` unset
- THEN it lands, prints no `cleared` line, and the file is still under `.clm/land`

#### Scenario: failed check keeps the target
- GIVEN the over-the-cap check, made to exit 1 after creating its file
- WHEN `clm land impl/a` runs
- THEN it exits 1 and `.clm/land` still holds the file

#### Scenario: caller's target untouched
- GIVEN the over-the-cap setup
- WHEN `clm land impl/a` runs with `CARGO_TARGET_DIR` set to a directory of the caller's
- THEN it lands, prints no `cleared` line, and the caller's directory still holds the file

#### Serves
- capability/work-concurrently
- capability/land-safely

#### Needs
- config/land-check-setting

### Requirement: reconnect-names-rebase
When `origin/main` has moved, the message `clm reconnect` prints SHALL tell the person to run `clm rebase` on each branch landed locally and then land it again, keeping its ``origin's `main` has moved`` start (`shared-plumbing`), and no file under `crates/clm-land/src` SHALL name `clm` followed by a space and one of the old subcommands `sync`, `checkpoint` or `page`.

#### Scenario: moved while offline
- GIVEN a repository that landed `impl/a` on its local `main` while offline, and an `origin/main` that has moved since
- WHEN `clm reconnect` runs
- THEN it prints ``origin's `main` has moved`` with `clm rebase`, prints `land again impl/a`, and exits 1

#### Serves
- architecture/plain-terms

### Requirement: spec-branch-holds-spec-paths
`clm land <branch>`, for a branch named `spec/*`, SHALL exit 1 when `gate/files-outside-spec-paths` lists any file for its head against the fetched base, naming each file and saying a spec branch holds only spec paths. It SHALL do so before it takes the land's lock, merges or runs any check, and it SHALL push and append nothing. A branch with any other name SHALL land as before.

#### Scenario: code on a spec branch
- GIVEN a branch `spec/x` that changes only `crates/x/src/lib.rs`, whose gate prints `code`, and a check command that records that it ran
- WHEN `clm land spec/x` runs
- THEN it exits 1 naming `crates/x/src/lib.rs`, the check command never ran, and `main` on `origin` and the ledger are unchanged

#### Scenario: a suggestion on a spec branch
- GIVEN a branch `spec/x` that only adds `backlog/x.md`, whose gate prints `code`
- WHEN `clm land spec/x` runs
- THEN it lands

#### Scenario: code on an impl branch
- GIVEN a branch `impl/x` that changes only `crates/x/src/lib.rs`
- WHEN `clm land impl/x` runs
- THEN this rule refuses nothing and it lands

#### Needs
- gate/files-outside-spec-paths

#### Serves
- capability/contribute-from-outside
- architecture/ai-writes-code
