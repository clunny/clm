---
title: View repository
status: active
owners: ["@clunny"]
depends_on: [view, model, check, orchestrator, orchestrator.loop, vcs]
paths: ["crates/clm-view-repo/**"]
successor: ""
serves: [capability/see-project-state]
---

## Contract
Crate `clm-view-repo`: the `clm status`, `clm view` and `clm pending` commands' reading of the repository, for `view`'s models and renderers (`views-rendered-by-clm`). `status(root, budgets)` and `view(root, name, format, width, source)` load the graph and markers from the working tree (`Source::Tree`) or, for `clm view --at <rev>`, from that commit's tree (`Source::At`, `view-at-rev`), read the checkout, the claim and `propose/*` refs and the time, and return what `view` renders, or the spec graph's violations, or why `<rev>` couldn't be read. `pending(root, component, at)` gives `clm pending [<component>] [--at <rev>]` its commit line and its pending lines (`pending-names-its-commit`). It never fetches and writes no file.

## Design
**At a commit** (`view-at-rev`). `--at <rev>` swaps only where the files come from. The commit's blobs are read from the local objects in memory with `clm_git::tree_blobs` (`vcs/tree-blobs`) rather than extracted as `clm next` extracts a tree, which keeps the cost low enough for the pane's 30-second redraw. They go to the same in-memory calls the `web` Worker uses (`clm_model::load_files`, `clm_check::markers_in`, `clm_check::size_of`), so `--at` marks what `clm view spec` would at that commit. When the CLM root lies below the repository's top, only the paths under it are kept, made relative to it; `linguist_excluded` comes from the commit's `.gitattributes`, and a binary blob, as git judges it, gives no markers. The budgets come from that commit's `clm.toml` alone (`clm_config::node_size_from`): the view describes the commit, so `clm.local.toml` and the environment don't apply.

**`<age>`.** Git has no command that says when a remote was last fetched, and the reflog of `refs/remotes/origin/main` says when the ref last moved and doesn't apply to a `<rev>` that is no ref, so the `FETCH_HEAD` files are read directly. `FETCH_HEAD` is per worktree while `origin/*` is shared by every worktree, so every worktree's file counts (`vcs/common-dir`), including one removed without `git worktree prune`, since it recorded a real fetch of the shared refs, and a fetch of another remote or a `git pull`, since it writes `FETCH_HEAD` too.

**Pending** (`pending-names-its-commit`). stdout stays the list `check.coverage/pending-report` defines, so the commit line goes to stderr, in `clm-view`'s `text` form. For `--at` the graph and markers are handed to `clm_check::pending` and narrowed with `clm_check::pending_for` (`check/pending-narrows-on-a-graph`), the function `pending_in` itself narrows with, so an unknown component gives the same violation either way.

**Tests.** The `--at` tests of the CLI live in `crates/clm/tests/view_at.rs`, so that file counts toward this component and `crates/clm/tests/view.rs` toward `view` alone (`tests-count-where-cited`).

## Requirements

### Requirement: view-at-rev
`clm view <name> --at <rev>` SHALL print the view of `<rev>`'s commit rather than of the working tree: its graph, markers, changes and backlog, and for `spec` its size budgets and size report, all from that commit's tree, read from the local objects without fetching and without writing any file, with claims from the remote-tracking refs as without `--at`. A `<rev>` that names no commit SHALL exit 1 naming it. The `text` and `ansi` charts SHALL begin, in place of the checkout line, with `<rev> <hash>`, the commit's first 7 hex digits, then ` · fetched <age> ago` when a `FETCH_HEAD` of the clone exists, the main checkout's or any linked worktree's, `<age>` since the newest of them, as `<n>m` under an hour and `<n>h` from then on, then ` · checkout <branch>` or ` · checkout detached`, and no `⚠`; the `svg` document SHALL hold the same words, and the `json` model SHALL carry a top-level `at` with `rev`, `commit`, and `fetched` and `branch` when known, and no `checkout`. Without `--at` the output SHALL be as `view/view-command` says.

#### Scenario: ahead of the checkout
- GIVEN a repository whose `HEAD` has component `a` with one requirement that no test cites, and whose remote-tracking ref `origin/main` is one commit ahead, adding a test that cites it at its current version
- WHEN `clm view flight` and `clm view flight --at origin/main` run
- THEN the first lists `a` as pending and the second doesn't, and the working tree and the index are unchanged

#### Scenario: header
- GIVEN that repository on branch `impl/a`, with a `FETCH_HEAD` modified 3 minutes ago
- WHEN `clm view spec --at origin/main` runs, and then `clm view spec --at origin/main --format json`, and then, with `HEAD` detached and no `FETCH_HEAD` in the main checkout or any linked worktree, `clm view spec --at origin/main` again
- THEN the chart's first line is `origin/main `, the first 7 hex digits of `origin/main`'s hash, ` · fetched 3m ago · checkout impl/a`; the JSON has no `checkout` and an `at` with `rev` `origin/main`, that `commit` and `branch` `impl/a`; and the last chart's first line ends `· checkout detached` and holds no `fetched`

#### Scenario: fetched from another worktree
- GIVEN that repository with a linked worktree on branch `impl/a` whose own `FETCH_HEAD` was modified 30 hours ago, and the main checkout's `FETCH_HEAD` modified 1 minute ago
- WHEN `clm view spec --at origin/main` runs in the linked worktree
- THEN the chart's first line holds ` · fetched 1m ago`

#### Scenario: no such commit
- WHEN `clm view spec --at nowhere` runs
- THEN it exits 1 and its stderr names `nowhere`

#### Serves
- capability/see-project-state

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

### Requirement: pending-names-its-commit
`clm pending [<component>]` SHALL print on stderr, before anything it prints on stdout and also when it prints the spec graph's violations, the checkout line as `view/view-command`'s `text` chart begins with it, and no such line when `HEAD` can't be read (`views-name-their-commit`). `clm pending [<component>] --at <rev>` SHALL list the pending requirements of `<rev>`'s commit, read as `view-at-rev` reads that commit, and print on stderr, in place of the checkout line, the first line `view-at-rev` gives its `text` chart; a `<rev>` that names no commit SHALL exit 1 naming it on stderr. With or without `--at`, stdout SHALL hold exactly what `check.coverage/pending-report` defines, and the exit code SHALL be its.

#### Scenario: behind origin/main
- GIVEN a repository whose `HEAD` has component `a` with one requirement `a/x` that no test cites, and whose remote-tracking ref `origin/main` is one commit ahead, adding a test that cites `a/x` at its current version
- WHEN `clm pending` and `clm pending --at origin/main` run, and then `clm pending` again once `HEAD` is moved to `origin/main`
- THEN the first prints `a/x <version> unbuilt` on stdout and, on stderr, `commit `, the first 7 hex digits of `HEAD`'s hash and `  ⚠ 1 behind origin/main`; the second prints nothing on stdout and a stderr line starting `origin/main ` and the first 7 hex digits of `origin/main`'s hash, holding no `⚠`; the third prints nothing on stdout and on stderr `commit ` and the first 7 hex digits of the new `HEAD`'s hash, holding no `⚠`; all three exit 0, and the first two leave the working tree and the index unchanged

#### Scenario: no such commit
- WHEN `clm pending --at nowhere` runs
- THEN it exits 1 and its stderr names `nowhere`

#### Serves
- capability/see-project-state

#### Needs
- vcs/tree-blobs
- vcs/common-dir
- view/view-command
- check/pending-narrows-on-a-graph

### Requirement: view-queue
`clm view queue` SHALL render the plan `orchestrator.loop`'s `stored_plan` reads, never fetching and writing no file, with the checkout line as the other views have it (`views-name-their-commit`) and the time and `origin/main` commit the plan was computed at. With no loop file it SHALL render a loop that isn't held, whose next section lists the ready, unclaimed components of the queue (`queue_from_refs`) read from the working tree, or with `--at <rev>` from that commit's tree, in priority order, and no drafts, saying that no loop has run on this clone; with a loop file, `--at` SHALL change only the checkout line.

#### Scenario: no loop yet
- GIVEN a clone with no `<git common dir>/clm/loop` and component `a` ready
- WHEN `clm view queue` runs
- THEN it exits 0, shows `loop not held`, lists `build:a` under `next up`, and writes no file

#### Scenario: the loop's plan
- GIVEN a loop running `build:a`, whose stored plan holds a fold batch `x` waiting for approval
- WHEN `clm view queue --at origin/main --format json` runs
- THEN its `running` holds `build:a` and its `waiting_on_you` holds `fold:x`

#### Serves
- capability/watch-beside-chat
- capability/build-from-spec

#### Needs
- orchestrator.loop/plan-snapshot
- view/queue-view
