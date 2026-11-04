---
component: land
base: { design: "dd6fae85bd646481" }
---

## MODIFIED Design

Crate `clm-land`, called from `crates/clm`'s `land`, `approve` (re-exported from `clm-approval`) and `reconnect` subcommands. Every git call goes through `clm-git`, the remote's name taken from `REMOTE` (`git-plumbing-layer`).

**`clm land [<branch>]`**, default the current branch, refused on `main`. Each attempt, up to ten:
1. Fetch `origin` (`main`, the branch, `clm/ledger/*`) and fold the ledger. Take the head, its fingerprint and change IDs from `land.approval`. In a signed repository its key check comes first, then its hold check, then whether the fingerprint is approved under the policy at the fetched base: all before the gate, the lock, the merge and the checks.
2. Run the gate on the head against `origin/main` in the tree this attempt checks in (**Places**, below), adding `--approved` when approved. A violation prints as the gate printed it and exits 1; `gate/approval` tells the caller to ask the human for `clm approve <branch>`.
3. `git merge-tree --write-tree origin/main <head>`. A conflict, or `specs/` changed on both sides, exits 1 telling the caller to run `clm rebase`.
4. The merge commit, `git commit-tree <merged tree> -p origin/main -p <head>`, message `Merge <branch> into main`, with `Change: <id>` on a following line for a spec land that adds or modifies live changes (the first by ID), committed as the repository's git user. A head that already contains `main` is still merged with a merge commit, so a claim's tip reaches `main` only through its land (`merged-claims-released`). The commit is checked out detached into the tree, so the gate and the checks run on a commit. Then **Derived specs** (below).
5. The merged tree's code key (`ledger`). When a `checked` record names it with the same command, it asks **Readers of checked code** (below): when they select no file, only `clm check --no-detectors` runs; when they select some, `[land] check` runs as below with their selection and then `clm check --no-detectors`, and no second `checked` record is appended; otherwise **Selected tests** (below), `[land] check` through `sh -c` with `CI` unset (so a script that installs a browser only in CI doesn't) and `CARGO_TARGET_DIR` set to `<home>/.clm/land` unless the caller set it non-empty, and then `CARGO_INCREMENTAL=0` unless the caller set that non-empty (`shared-land-target`), then `clm check`, and on success a `checked` record. `<main>` is the first worktree `git worktree list --porcelain` lists and `<home>` the CLM root in it, `<main>/<prefix>` with `<prefix>` what `git rev-parse --show-prefix` prints in the CLM root, computed once; a land from a build's worktree under `.claude/worktrees/` so shares one target (`build-places`). The gate then runs again on the merge commit, since a file both sides touched can change whether it needs review, and in a signed repository `land.approval` signs the `landed` record.
6. `git push origin <merge>:refs/heads/main --force-with-lease=refs/heads/main:<the base step 1 fetched>`. A refusal isn't always `main` moving (`land-push-retry`), so it fetches `origin`'s `main` again, directly rather than through step 1's offline refusal, and judges it: the merge commit means the push went through; neither the merge commit nor the base means `main` moved and the next attempt starts, its `checked` record standing, so a retry with the same code key runs only `clm check --no-detectors`; still the base means a refusal for another reason, such as `[remote rejected] main (failure)` while other sessions push, and the same commit is pushed again with the same lease at most twice more, after fixed waits of 1 and then 2 seconds with no override, these pushes not counting as attempts. Still the base after the last is exit 2 with git's message, including the remote's lines (the push drops `--quiet` if it hides them), and a line saying the failure may be transient and that running `clm land <branch>` again is safe. When the refetch fails the push may have gone through, so it exits 2 with the push's message alone. Ten moved attempts exit 1 saying `main` keeps moving.
7. Append, in one commit (`ledger/append-all`), the `effort` records `clm effort record --branch <branch>` would append for the branch's sessions on this machine, read once before the first attempt so neither the reading nor a second ledger push lengthens the time `main` has moved with no `landed` record (`effort-accounted`; the reader's warnings print prefixed `effort: `, and when it can't read at all it warns `effort not recorded: <why>`), then `landed`: commit, branch, `land_kind` and `review` from the gate's first line, fingerprint, the step-5 code key as `code_key`, the change IDs. When `append` reports `landed` unpushed and `host` isn't `none`, it warns `the landed record isn't on origin yet: git push origin clm/ledger/<person>/<clone>` and still exits 0, since `main` has moved. Then `host`'s `branch delete <branch> --lease <head>` for an `impl/*`, `spec/*` or `propose/*` branch (a failure warns), the local branch too when it is checked out nowhere (`clm wrap-up` tidies one that is), and for `impl/*`, **Freeing a build's disk** (below).
8. Print `landed <sha> <kind>`.

**Derived specs** (`derived-specs-fingerprint`). When the head adds a change (`clm-gate`'s `added_changes` over the merge base's and the head's `ls-tree -r -z` listings, the question `gate.fingerprint/fingerprint` asks), or applies one (a change whose `change.md` parses with status `proposed` or `approved` at the base, `origin/main`'s, and in the merge commit has status `applied`, or is gone while `changes/archive/<id>/change.md` has status `done`, as `sync`'s Design defines it: `regenerate-applied-live-changes`), the IDs of the added changes whose `change.md` in the merge commit parses with status `applied` (live) or `done` (new archived), and of the applied ones, sorted, go to `sync.regenerate`'s `regenerated` (crate `clm-regenerate`, which reaches no `clm-health`: `clm/health-isolated`) without its stale-delta merge (`sync.regenerate/regenerated-tree`), run with the tree holding the merge commit as `root` and the fetched base's commit SHA as `onto`, so the scratch `specs/`, the copied `changes/` and `backlog/` and the re-apply are `clm rebase`'s own. A stop exits 1 with rule `land/derived-specs`, the lines it returned and `clm rebase`; otherwise the regenerated `specs/` and the merge commit's are compared byte for byte, a symlink by its target, over the union of their paths in byte order, a path on one side only being a difference, and the first difference exits 1 naming it. It runs before step 5, so before the `--no-detectors` run too. The fingerprint skips `specs/` on a head that adds a change, so this is what keeps an approval from covering a hand edit there. A head that applies a change and adds none keeps `specs/` in its fingerprint, which reads listings alone and can't see a status, so a sync that regenerates its `specs/` onto a `main` that changed them asks for its approval again.

**Places** (`scratch-outside-git`, `land-tree`, `fixed-land-tree`). Cargo names a workspace's outputs by its path and judges them fresh by file times, so two lands that overlap on one target can check each other's code, and a fresh path per land rebuilt everything (43 GB a day on one machine). Before step 2 an attempt computes the merged tree and its code key (`git merge-tree --write-tree` writes only objects); when its check will compile into `<home>/.clm/land`, it takes the exclusive advisory lock on `<home>/.clm/land.lock` (`land-target-lock`, `<home>/.clm/` created when missing, released by the system when a land is killed), printing `waiting for another land's check on <home>/.clm/land` on stderr once when it waits, with no time limit, and holds it until the push has gone through and the cap is applied, or the attempt ends; the append, the deletes and the print run outside it. **The cap** (`land-target-cap`): right after a push that went through, still under the lock, it sums the lengths of the regular files under `.clm/land` (no symlink followed, so a sparse file counts its length) and, above `[land] target_cap_gb` gigabytes, removes the directory and prints `cleared .clm/land (<n> GB, cap <cap> GB)`; an unreadable entry or a failed removal only warns. Incremental data and units for feature sets no land builds again are what grow there, and Cargo prunes neither, so the cap trades one cold land for a bounded directory, and each attempt takes it afresh. A merge that conflicts, a checked code key whose readers select nothing, and a caller's `CARGO_TARGET_DIR` take no lock. A locked attempt checks out in the land tree `<home>/.clm/land-tree` (checks of a root in a subdirectory run in `<home>/.clm/land-tree/<prefix>`): when `git worktree list --porcelain` lists it (both canonicalised) and it exists, `git checkout --quiet --force --detach <commit>` and then `git clean --quiet -fdx`; otherwise, or when either fails, it removes the path, runs `git worktree prune` and `git worktree add --quiet --detach <path> <commit>`, a failed add being exit 2. The tree stays when the attempt ends; `clm land` and `clm wrap-up` never remove it, and removing it by hand starts the next land cold. Any other attempt uses a fresh `land-*` worktree under `TMPDIR` (read through the injected environment lookup) or else `std::env::temp_dir()`, removed on every exit path, never one under `.claude/worktrees/`, and outside the repository, so no tool that refuses paths under `.git/` (Vite's `server.fs.deny`) or walks up to project files trips on it; a killed land's tree stays until the system clears it.

**Its own binary** (`land-binary-copy`). A check that rebuilds `clm` can replace the file `clm land` runs from. So before its first subcommand it copies `std::env::current_exe` into a fresh `land-bin-*` directory beside its temporary worktrees, keeping permissions, and runs the gate, `clm affected` and `clm check` from the copy, removed on every exit path; when the copy can't be made it exits 1 naming the path. The lander's binary is `main`'s rule, which the merged tree lives under. `CLM_GATE`, `CLM_CHECK` and `CLM_AFFECTED` override the three commands, run as given, so tests fake them.

**Selected tests** (`graph-test-selection`). Before `[land] check` it runs `clm affected --base <sha>`, `<sha>` the fetched base's commit (the merge commit's merge base with it, so the selection is what the branch brings), in the tree on the merge commit and under the lock when the check holds it, and writes its stdout unchanged to `selection` in a temporary directory, which `CLM_SELECTION` names. When it fails, `clm affected failed: <the first non-empty line of its stderr, or the spawn error>; the check runs every test`, and the check runs without `CLM_SELECTION`: a failed selection costs time, never a skipped test.

**Readers of checked code** (`doc-readers-marked`). `code_key` leaves the doc-only paths out, so a `checked` record says nothing of the tests that read them: a spec branch's prose can fail such a test (a retired term under `specs/`) on a tree whose code was checked. A land whose code key is checked checks the merge commit out into a temporary worktree, as an attempt with no lock does, and runs `clm affected --base <sha> --readers` there (`check.affected/affected-readers`), `<sha>` as in **Selected tests**. When it prints nothing, the attempt goes on in that worktree as before, with no lock. When it prints a line, the attempt removes that worktree and goes on as one whose check runs: it takes the lock when the check compiles into `.clm/land`, checks the merge commit out into the land tree, so the check reuses the warm target, and runs `[land] check` with `CLM_SELECTION` naming a file holding that output unchanged. When it fails, it prints `clm affected failed: <reason>; the check runs every test` as **Selected tests** does and goes on the same way with `CLM_SELECTION` unset. A failing check exits 1 as any land's does.

**The check's output** (`land-check-log`). One pipe (`std::io::pipe`) carries `[land] check`'s stdout and stderr in the order written, copied to stderr and to `<home>/.clm/land-logs/<branch>.log`, the branch's slashes as directories, created and truncated first; the last 40 lines (split on `\n`) are kept in memory. It reads until the pipe closes, so a check must end what it starts. A failing check's refusal, printed last as `clm land: <why>`, is `last 40 lines of `<check>`:`, those lines and `` `<check>` fails on the merge with main; its output is at <path> ``, and the log stays; a passing check removes it. A log that can't be created warns naming the path, and the refusal then has no `; its output is at` clause. `clm check`'s output isn't logged.

**Freeing a build's disk** (`land-frees-build-target`). After the push and the append of an `impl/*` land it deletes the `target/` at `git rev-parse --show-toplevel` when that is a linked worktree whose `HEAD` is the landed branch, printing `removed <path>`; a missing one is skipped and a failed delete warns. `clm run --finish` removes the worktree itself (`orchestrator.run`).

**`--check-only`** (`land-check-command`). Given a branch as well it exits 1 (by hand: clap's `conflicts_with` would exit 2) and runs nothing. On any branch, in the CLM root (`--root`, else found from the current directory), it runs `[land] check` as step 5 does but with `CARGO_TARGET_DIR` as the caller has it, so a build compiles into its own `target/`, then `clm check` with its detectors (`CLM_CHECK` overrides it), on the working tree as it stands, output on stderr. It prints `checks pass`; otherwise exits 1 with `` `<command>` fails `` or `` `clm check` fails ``, the failure messages being a parameter of the helpers a land shares; a `clm.toml` that doesn't load is exit 1 with `config`'s message. It reads no ledger and a working tree has no code key, so it never lets a land skip its check. It exists for a build's pre-land check (`orchestrator.run`), which Claude Code's worktree isolation refused as a script path.

**Offline** (`no-local-land-with-remote`). With `host = "none"` the base is the local `main`, there is no fetch or push, the records stay local, and it prints `landed <sha> <kind> (local)`. With any other host `main` has one copy, the remote's, so a failed step-1 fetch exits 1 with `offline: origin can't be reached; push and land when it can`. **`clm reconnect`**, for a repository that gains a remote, pushes `main` with a lease when `origin/main` is missing or an ancestor, then every local `clm/ledger/*` branch, and prints `reconnected <n> lands`; when `origin/main` has moved it pushes only the ledgers, prints `land again <branch>` for each local land (the merge commit's second parent) and exits 1.

## MODIFIED Requirements

### Requirement: land <!-- base: cd889d18ee303f3f -->
`clm land [<branch>]` SHALL fetch `origin`, run the gate on the branch's head against `origin/main` with `--approved` when the ledger holds an `approved` record for the head's fingerprint that the `approval` policy accepts, merge the head into `origin/main` in a scratch tree, run `[land] check` and then `clm check` on the merged tree unless a `checked` record names its code key with the same command (then `clm check --no-detectors`, preceded by `[land] check` only when the readers `selected-tests` asks for select a file), run the gate on the merged tree, make a merge commit of the head into `origin/main`, push it to `main` without force and with a lease on the fetched `origin/main`, append `landed` naming the merged tree's code key and whether the land needed review, and print `landed <sha> <kind>`. When the push is refused because `main` moved it SHALL start again from the fetch, up to ten attempts. It SHALL push nothing and exit 1, naming why, when the gate blocks, the merge conflicts, a check fails, or every attempt is refused; and SHALL append `checked` whenever the check command and `clm check` pass on a tree, even when the push is later refused.

#### Scenario: code branch lands
- GIVEN `origin/main`, a branch `impl/a` with a passing code change, no `checked` record, and fake gate and check commands that record the tree they ran on
- WHEN `clm land impl/a` runs
- THEN `main` on `origin` is a merge commit whose parents are the old `main` and the head, the check command ran on that commit's tree, the ledger has `checked` for its code key and `landed` for the commit with `land_kind` `code` and that code key, and it printed `landed <sha> code`

#### Scenario: already checked code
- GIVEN a branch that changes only `specs/x/SPEC.md`, which no `clm:reads` marker names, and a `checked` record for `origin/main`'s code key with the configured command
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

### Requirement: selected-tests <!-- base: 64c312ff885e88b9 -->
When `clm land` runs `[land] check`, it SHALL first run `clm affected --base <sha>`, `<sha>` being the commit of the `origin/main` it fetched (the local `main`'s with no remote), in the worktree it checks, write its stdout unchanged to a file in its temporary directory, and run `[land] check` with `CLM_SELECTION` set to that file's path. When `clm affected` fails, it SHALL print `clm affected failed: <reason>; the check runs every test` on one line of stderr, `<reason>` being the first non-empty line of its stderr, trimmed, or the spawn error, and run `[land] check` with `CLM_SELECTION` unset. When the merged tree's code key has a `checked` record with the same command, it SHALL instead run `clm affected --base <sha> --readers`, and SHALL run `[land] check` only when that prints a line, with `CLM_SELECTION` naming a file holding its output unchanged, or when it fails, with the same message and `CLM_SELECTION` unset; such a check SHALL run in the land tree under the lock as any other check that compiles into `.clm/land` does (`land-target-lock`), and on success SHALL append no second `checked` record. A land that runs no check SHALL run no other selection.

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
- GIVEN a branch whose merged tree's code key has a `checked` record with the same command, and a `CLM_AFFECTED` command that records its arguments and prints nothing
- WHEN `clm land` runs
- THEN it ran that command once, given `--base`, the fetched commit and `--readers`, never ran `[land] check`, and the branch landed

#### Scenario: readers of checked code
- GIVEN a branch that changes only `ARCHITECTURE.md`, a `checked` record for the merged tree's code key with the same command, a `[land] check` that copies the file `CLM_SELECTION` names, and a `CLM_AFFECTED` command that prints `file crates/a/tests/r.rs` when given `--readers`
- WHEN `clm land` runs
- THEN the copied file holds that line, the check ran in the land tree, the branch landed, and the ledger holds one `checked` record for that code key

#### Scenario: a reader fails on checked code
- GIVEN the same branch and `CLM_AFFECTED`, and a `[land] check` that exits 1
- WHEN `clm land` runs
- THEN it exits 1, and `main` on `origin` is unchanged

#### Scenario: readers' selection fails on checked code
- GIVEN the same branch and `checked` record, and `CLM_AFFECTED` set to a command that exits 1
- WHEN `clm land` runs
- THEN it prints `clm affected failed:` on stderr, runs `[land] check` with no `CLM_SELECTION`, and the branch lands

#### Needs
- check.affected/affected-readers
