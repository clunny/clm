---
component: view.repo
base: { contract: "80b93f16487e6dc5", design: "13e4eb06279d7f06" }
---

## MODIFIED Contract
Crate `clm-view-repo`: the `clm status`, `clm view` and `clm pending` commands' reading of the repository, for `view`'s models and renderers (`views-rendered-by-clm`). `status(root, budgets)` and `view(root, name, format, width, source)` load the graph and markers from the working tree (`Source::Tree`) or, for `clm view --at <rev>`, from that commit's tree (`Source::At`, `view-at-rev`), read the checkout, the claim and `propose/*` refs and the time, and return what `view` renders, or the spec graph's violations, or why `<rev>` couldn't be read. `pending(root, component, at)` returns, for `clm pending [<component>] [--at <rev>]`, the line naming the commit it describes and the pending lines `clm_check::pending` gives for the working tree or for `<rev>`'s tree, or the spec graph's violations, or why `<rev>` couldn't be read (`pending-names-its-commit`). Its public items are `status`, `view`, `pending`, `Source`, `Error`, `BudgetsFrom` and a re-export of `clm_check::Budgets`, which its callers pass in. It never fetches and writes no file.

## MODIFIED Design
This crate depends on `clm-view`, on `clm-git`, on `clm-check` with its `checkout` and `detectors` features and on `clm-orchestrator` with `git`, so `clm-view` keeps no feature and no git. `view`'s Design says what the commands print (the checkout line, the size report, the models' arguments); this crate is where they are read. `view` takes a `clm_view::Format`, and `Error` is `Violations` or `At`.

**At a commit** (`view-at-rev`). `--at <rev>` swaps only where the files come from. This crate resolves `<rev>` to a commit (`git rev-parse --verify <rev>^{commit}`) and reads its blobs with `clm_git::tree_blobs` (`vcs/tree-blobs`: one `git ls-tree -r -z` and one `git cat-file --batch`, gitlinks and symbolic links skipped as the size count skips them, blobs as stored, without checkout filters such as `core.autocrlf`), keeping every `.gitattributes` and, when the CLM root lies below the repository's top (`git rev-parse --show-prefix`), only the paths under it, made relative to it; then it hands the paths and texts to the same in-memory calls the `web` Worker uses: `clm_model::load_files` for the graph, `clm_check::markers_in` per file for the markers, and `clm_check::size_of` over each blob's `Length::of(path, text)`, as `clm size` measures the working tree (so test parts count apart and `--at` marks the components `clm view spec` would at the same commit), with that commit's `clm.toml` alone for the budgets (`clm_config::node_size_from`: the view describes the commit, so `clm.local.toml` and the environment don't apply) and its `.gitattributes` for `linguist_excluded`, for the size report. Binary blobs, those holding a NUL in their first 8000 bytes as git judges them, give no markers. Reading blobs rather than extracting the tree as `clm next` does keeps the cost low enough for the pane's 30-second redraw. The claims and the time come from the remote-tracking refs and `now()`, as without `--at`. The hash in the chart's first line is cut to 7 hex digits as the checkout line's is, and `<age>` counts from the newest modification time of the clone's `FETCH_HEAD` files, in whole minutes as `<n>m` under an hour (`0m` under a minute) and whole hours as `<n>h` from then on. `FETCH_HEAD` is per worktree (the main checkout's is `<common dir>/FETCH_HEAD`, a linked worktree's `<common dir>/worktrees/<name>/FETCH_HEAD`), while `origin/*` is shared by every worktree, so a fetch run in any of them moves the ref the line names; this crate takes the common directory from `clm_git::common_dir` (`vcs/common-dir`), and reads the modification times of `<common dir>/FETCH_HEAD` and of every `<common dir>/worktrees/*/FETCH_HEAD` from the file system, skipping those that don't exist or can't be read; a worktree removed without `git worktree prune` counts, since its file recorded a real fetch of the shared refs. Git has no command that says when a remote was last fetched, and the reflog of `refs/remotes/origin/main` says when the ref last moved, not when it was last fetched, and doesn't apply to a `<rev>` that is no ref, so the files are read directly. A fetch of another remote, or a `git pull`, also writes `FETCH_HEAD` and counts as the last fetch. The branch is `clm_git::current_branch`. The JSON's `fetched` is Unix seconds. `clm status` takes no `--at`.

**Pending** (`pending-names-its-commit`). `clm pending` describes the checkout as `clm status` does, and a session whose checkout is behind `origin/main` read it as the state of `main`: a requirement a build had landed showed as `behind`, and requirements whose blockers had landed showed as blocked, while `clm next` disagreed. So it names its commit as the views do (`views-name-their-commit`), and `--at <rev>` lets a session that may not sync its checkout, such as `build`'s, read `origin/main` after `clm next` has fetched it. Its stdout stays the list `check.coverage/pending-report` defines, read a line at a time and empty when nothing is pending, so the line naming the commit goes to stderr, without colour. `pending` takes the checkout line from the same reading `status` and `view` use, and for `--at` the same blob reading and the same first line as `view-at-rev`, handing the graph and markers to `clm_check::pending`, which gives the same for a graph and markers loaded without a checkout; for the working tree it calls `clm_check::pending_in`, and for `--at` it narrows to `<component>` with `clm_check::pending_for` (`check/pending-narrows-on-a-graph`), the function `pending_in` itself narrows with, so an unknown component gives the same violation either way. The plain checkout line is the `text` form of the line `view/view-command` defines, taken from `clm-view`'s `Checkout` and `At` rather than formatted again here. The `clm pending` command in `crates/clm` calls `pending` and prints what it returns.

**Tests.** The `--at` tests of the CLI, `clm pending`'s commit line among them, live in `crates/clm/tests/view_at.rs`, so that file counts toward this component and `crates/clm/tests/view.rs` toward `view` alone (`tests-count-where-cited`). `at.rs` takes its tree listing and common directory from `vcs`'s `tree_blobs` and `common_dir`.

## ADDED Requirements

### Requirement: pending-names-its-commit
`clm pending [<component>]` SHALL print on stderr, before anything it prints on stdout, and also when it prints the spec graph's violations, the checkout line `view/view-command` defines, `commit <hash>` followed by two spaces and `⚠ <n> behind origin/main` when the checkout lacks `n` of `origin/main`'s commits as of the last fetch, without ANSI escapes, and no such line when `HEAD` can't be read (`views-name-their-commit`). `clm pending [<component>] --at <rev>` SHALL list the pending requirements of `<rev>`'s commit instead of the working tree's, its specs, changes, backlog and markers read from that commit's tree as `view-at-rev` reads them, from the local objects without fetching and without writing any file, and print on stderr, in place of the checkout line, the first line `view-at-rev` gives its `text` chart (`<rev> <hash>`, then ` · fetched <age> ago` when known, then ` · checkout <branch>` or ` · checkout detached`). A `<rev>` that names no commit SHALL exit 1 naming it on stderr. With or without `--at`, stdout SHALL hold exactly what `check.coverage/pending-report` defines, and the exit code SHALL be its.

#### Scenario: behind origin/main
- GIVEN a repository whose `HEAD` has component `a` with one requirement `a/x` that no test cites, and whose remote-tracking ref `origin/main` is one commit ahead, adding a test that cites `a/x` at its current version
- WHEN `clm pending` and `clm pending --at origin/main` run
- THEN the first prints `a/x <version> unbuilt` on stdout and, on stderr, `commit `, the first 7 hex digits of `HEAD`'s hash and `  ⚠ 1 behind origin/main`; the second prints nothing on stdout and a stderr line starting `origin/main ` and the first 7 hex digits of `origin/main`'s hash, holding no `⚠`; both exit 0, and the working tree and the index are unchanged

#### Scenario: up to date
- GIVEN a repository whose `HEAD` is `origin/main`, with nothing pending
- WHEN `clm pending` runs
- THEN it exits 0, prints nothing on stdout, and prints on stderr `commit ` and the first 7 hex digits of `HEAD`'s hash, holding no `⚠`

#### Scenario: no such commit
- WHEN `clm pending --at nowhere` runs
- THEN it exits 1 and its stderr names `nowhere`

#### Serves
- capability/see-project-state
- architecture/views-name-their-commit

#### Needs
- vcs/tree-blobs
- vcs/common-dir
- view/view-command
- check/pending-narrows-on-a-graph
