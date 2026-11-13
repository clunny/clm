---
component: view.repo
---

## MODIFIED Requirements

### Requirement: view-at-rev <!-- base: 6e9e02ab9e2c8d11 -->
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

### Requirement: pending-names-its-commit <!-- base: bcee5366e91220b8 -->
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
