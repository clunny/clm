---
title: View repository
status: active
owners: ["@clunny"]
depends_on: [view, model, check, orchestrator, vcs]
paths: ["crates/clm-view-repo/**"]
successor: ""
serves: [capability/see-project-state]
---

## Contract
Crate `clm-view-repo`: the `clm status` and `clm view` commands' reading of the repository, for `view`'s models and renderers (`views-rendered-by-clm`). `status(root, budgets)` and `view(root, name, format, width, source)` load the graph and markers from the working tree (`Source::Tree`) or, for `clm view --at <rev>`, from that commit's tree (`Source::At`, `view-at-rev`), read the checkout, the claim and `propose/*` refs and the time, and return what `view` renders, or the spec graph's violations, or why `<rev>` couldn't be read. Its public items are `status`, `view`, `Source`, `Error`, `BudgetsFrom` and a re-export of `clm_check::Budgets`, which its callers pass in. It never fetches and writes no file.

## Design
This crate depends on `clm-view`, on `clm-git`, on `clm-check` with its `checkout` and `detectors` features and on `clm-orchestrator` with `git`, so `clm-view` keeps no feature and no git. `view`'s Design says what the commands print (the checkout line, the size report, the models' arguments); this crate is where they are read. `view` takes a `clm_view::Format`, and `Error` is `Violations` or `At`.

**At a commit** (`view-at-rev`). `--at <rev>` swaps only where the files come from. This crate resolves `<rev>` to a commit (`git rev-parse --verify <rev>^{commit}`) and reads its blobs with `clm_git::tree_blobs` (`vcs/tree-blobs`: one `git ls-tree -r -z` and one `git cat-file --batch`, gitlinks and symbolic links skipped as the size count skips them, blobs as stored, without checkout filters such as `core.autocrlf`), keeping every `.gitattributes` and, when the CLM root lies below the repository's top (`git rev-parse --show-prefix`), only the paths under it, made relative to it; then it hands the paths and texts to the same in-memory calls the `web` Worker uses: `clm_model::load_files` for the graph, `clm_check::markers_in` per file for the markers, and `clm_check::size_of` over each blob's `Length::of(path, text)`, as `clm size` measures the working tree (so test parts count apart and `--at` marks the components `clm view spec` would at the same commit), with that commit's `clm.toml` alone for the budgets (`clm_config::node_size_from`: the view describes the commit, so `clm.local.toml` and the environment don't apply) and its `.gitattributes` for `linguist_excluded`, for the size report. Binary blobs, those holding a NUL in their first 8000 bytes as git judges them, give no markers. Reading blobs rather than extracting the tree as `clm next` does keeps the cost low enough for the pane's 30-second redraw. The claims and the time come from the remote-tracking refs and `now()`, as without `--at`. The hash in the chart's first line is cut to 7 hex digits as the checkout line's is, and `<age>` counts from the newest modification time of the clone's `FETCH_HEAD` files, in whole minutes as `<n>m` under an hour (`0m` under a minute) and whole hours as `<n>h` from then on. `FETCH_HEAD` is per worktree (the main checkout's is `<common dir>/FETCH_HEAD`, a linked worktree's `<common dir>/worktrees/<name>/FETCH_HEAD`), while `origin/*` is shared by every worktree, so a fetch run in any of them moves the ref the line names; this crate takes the common directory from `clm_git::common_dir` (`vcs/common-dir`), and reads the modification times of `<common dir>/FETCH_HEAD` and of every `<common dir>/worktrees/*/FETCH_HEAD` from the file system, skipping those that don't exist or can't be read; a worktree removed without `git worktree prune` counts, since its file recorded a real fetch of the shared refs. Git has no command that says when a remote was last fetched, and the reflog of `refs/remotes/origin/main` says when the ref last moved, not when it was last fetched, and doesn't apply to a `<rev>` that is no ref, so the files are read directly. A fetch of another remote, or a `git pull`, also writes `FETCH_HEAD` and counts as the last fetch. The branch is `clm_git::current_branch`. The JSON's `fetched` is Unix seconds. `clm status` takes no `--at`.

**Tests.** The `--at` tests of the CLI live in `crates/clm/tests/view_at.rs`, so that file counts toward this component and `crates/clm/tests/view.rs` toward `view` alone (`tests-count-where-cited`). `at.rs` takes its tree listing and common directory from `vcs`'s `tree_blobs` and `common_dir`.

## Requirements

### Requirement: view-at-rev
`clm view <name> --at <rev>` SHALL print the view of `<rev>`'s commit rather than of the working tree: its graph, markers, changes and backlog, and for `spec` its size budgets and size report, all from that commit's tree, read from the local objects without fetching and without writing any file, with claims from the remote-tracking refs as without `--at`. A `<rev>` that names no commit SHALL exit 1 naming it. The `text` and `ansi` charts SHALL begin, in place of the checkout line, with `<rev> <hash>`, the commit's first 7 hex digits, then ` · fetched <age> ago` when a `FETCH_HEAD` of the clone exists, the main checkout's or any linked worktree's, `<age>` since the newest of them, as `<n>m` under an hour and `<n>h` from then on, then ` · checkout <branch>` or ` · checkout detached`, and no `⚠`; the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `at` with `rev`, `commit`, and `fetched` and `branch` when known, and no `checkout`. Without `--at` the output SHALL be as `view/view-command` says.

#### Scenario: ahead of the checkout
- GIVEN a repository whose `HEAD` has component `a` with one requirement that no test cites, and whose remote-tracking ref `origin/main` is one commit ahead, adding a test that cites it at its current version
- WHEN `clm view flight` and `clm view flight --at origin/main` run
- THEN the first lists `a` as pending and the second doesn't, and the working tree and the index are unchanged

#### Scenario: header
- GIVEN that repository on branch `impl/a`, with a `FETCH_HEAD` modified 3 minutes ago
- WHEN `clm view spec --at origin/main` runs, and then `clm view spec --at origin/main --format json`
- THEN the chart's first line is `origin/main `, the first 7 hex digits of `origin/main`'s hash, ` · fetched 3m ago · checkout impl/a`, and the JSON has no `checkout` and an `at` with `rev` `origin/main`, that `commit` and `branch` `impl/a`

#### Scenario: detached, never fetched
- GIVEN that repository with `HEAD` detached and no `FETCH_HEAD` in the main checkout or any linked worktree
- WHEN `clm view spec --at origin/main` runs
- THEN its first line ends `· checkout detached` and holds no `fetched`

#### Scenario: fetched from another worktree
- GIVEN that repository with a linked worktree on branch `impl/a` whose own `FETCH_HEAD` was modified 30 hours ago, and the main checkout's `FETCH_HEAD` modified 1 minute ago
- WHEN `clm view spec --at origin/main` runs in the linked worktree
- THEN the chart's first line holds ` · fetched 1m ago`

#### Scenario: no such commit
- WHEN `clm view spec --at nowhere` runs
- THEN it exits 1 and its stderr names `nowhere`

#### Serves
- capability/see-project-state
- architecture/views-name-their-commit

#### Needs
- vcs/tree-blobs
- vcs/common-dir
- view/view-command

### Requirement: shared-plumbing
`clm-view-repo` SHALL take the remote's name from `clm-git`'s `REMOTE` for the ref it compares the checkout against (`refs/remotes/<remote>/main`), and read the time from `clm-git`'s `now()`, defining no clock of its own: no line of a file under `crates/clm-view-repo/src` or `crates/clm-view/src`, apart from those whose first non-blank characters are `//`, SHALL hold `"origin"`, `"origin/`, `remotes/origin`, `SystemTime` or `fn now(` (`remote-plumbing-scan`). Text it renders for a person that names `origin`, such as `⚠ <n> behind origin/main`, SHALL keep its wording.

#### Scenario: no copy of the remote's name or the clock
- GIVEN the workspace
- WHEN a test in `clm-view-repo` reads every file under `crates/clm-view-repo/src` and `crates/clm-view/src`, skipping each line whose first non-blank characters are `//`
- THEN no line holds `"origin"`, `"origin/`, `remotes/origin`, `SystemTime` or `fn now(`

#### Serves
- architecture/reuse-first

#### Needs
- view/view-command
