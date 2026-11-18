---
title: View repository
status: active
owners: ["@clunny"]
depends_on: [view, model, check, check.coverage, orchestrator, orchestrator.loop, vcs]
paths: ["crates/clm-view-repo/**"]
successor: ""
serves: [capability/see-project-state]
---

## Contract
Crate `clm-view-repo`: the `clm status`, `clm view` and `clm pending` commands' reading of the repository, for `view`'s models and renderers (`views-rendered-by-clm`). `status(root, budgets)` and `view(root, name, format, width, source)` load the graph and markers from the working tree (`Source::Tree`) or, for `clm view --at <rev>`, from that commit's tree (`Source::At`, `view-at-rev`), read the checkout, the claim and `propose/*` refs and the time, and return what `view` renders, or the spec graph's violations, or why `<rev>` couldn't be read. `pending(root, component, at)` gives `clm pending [<component>] [--at <rev>]` its commit line and its pending lines (`pending-names-its-commit`). It never fetches and writes no file.

`clm-view-repo` offers each of `clm status`, `clm view` and `clm pending` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## Design
**At a commit** (`view-at-rev`). `--at <rev>` swaps only where the files come from. The commit's blobs are read from the local objects in memory with `clm_git::tree_blobs` (`vcs/tree-blobs`) rather than extracted as `clm next` extracts a tree, which keeps the cost low enough for the pane's 30-second redraw. They go to the same in-memory calls the `web` Worker uses (`clm_model::load_files`, `clm_coverage::markers_in`, `clm_check::size_of`), so `--at` marks what `clm view spec` would at that commit. When the CLM root lies below the repository's top, only the paths under it are kept, made relative to it; `linguist_excluded` comes from the commit's `.gitattributes`, and a binary blob, as git judges it, gives no markers. The budgets come from that commit's `clm.toml` alone (`clm_config::node_size_from`): the view describes the commit, so `clm.local.toml` and the environment don't apply.

**`<age>`.** Git has no command that says when a remote was last fetched, and the reflog of `refs/remotes/origin/main` says when the ref last moved and doesn't apply to a `<rev>` that is no ref, so the `FETCH_HEAD` files are read directly. `FETCH_HEAD` is per worktree while `origin/*` is shared by every worktree, so every worktree's file counts (`vcs/common-dir`), including one removed without `git worktree prune`, since it recorded a real fetch of the shared refs, and a fetch of another remote or a `git pull`, since it writes `FETCH_HEAD` too.

**Pending** (`pending-names-its-commit`). stdout stays the list `check.coverage/pending-report` defines, so the commit line goes to stderr, in `clm-view`'s `text` form. For `--at` the graph and markers are handed to `clm_coverage::pending` and narrowed with `clm_check::pending_for` (`check/pending-narrows-on-a-graph`), the function `pending_in` itself narrows with, so an unknown component gives the same violation either way.

**What the commands decide** (`commands-live-in-their-crates`). `view` states what `clm status` and `clm view` draw for the graph, markers, claim refs, checkout and size report they are handed, and `check.coverage` the lines `clm pending` lists; both crates lie below this one. What only the commands do is this component's: what they read from the repository and never read (`reads-the-repository`), and how they take their arguments and exit (`arguments-and-exit`). `clm-view-repo` is handed the budgets, since it loads no settings, so that `clm status` marks a component by `clm.toml`'s budget is the root's to show (`clm/routes-only`).

**Tests.** Every test of this component sits in `crates/clm-view-repo/tests/` and calls the crate; none runs the `clm` binary.

## Requirements

### Requirement: view-at-rev
`clm view <name> --at <rev>` SHALL print the view of `<rev>`'s commit rather than of the working tree: its graph, markers, changes and backlog, and for `spec` its size budgets and size report, all from that commit's tree, read from the local objects without fetching and without writing any file, with claims from the remote-tracking refs as without `--at`. A `<rev>` that names no commit SHALL exit 1 naming it. In place of a checkout it SHALL hand `view` the commit, which every form names as `view/view-command` says: `<rev>`, the commit's hash cut to its first 7 hex digits, the time of the newest `FETCH_HEAD` of the clone when one exists, the main checkout's or any linked worktree's, and the checked-out branch, or none when `HEAD` is detached. Without `--at` the output SHALL be as `view/view-command` says.

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
`clm pending [<component>]` SHALL print on stderr, before anything it prints on stdout and also when it prints the spec graph's violations, the checkout line as `view/view-command`'s `text` chart begins with it, and no such line when `HEAD` can't be read (`views-name-their-commit`). `clm pending [<component>] --at <rev>` SHALL list the pending requirements of `<rev>`'s commit, read as `view-at-rev` reads that commit, and print on stderr, in place of the checkout line, the first line `view-at-rev` gives its `text` chart; a `<rev>` that names no commit SHALL exit 1 naming it on stderr. With or without `--at`, stdout SHALL hold exactly what `check.coverage/pending-report` defines, and the exit status SHALL be as `arguments-and-exit` says.

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
- check.coverage/pending-report

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

### Requirement: commands-in-crate
`clm-view-repo` SHALL offer each of `clm status`, `clm view` and `clm pending` as a call of its own that takes the command's arguments as the command line gives them, the repository root `crates/clm` found from the current directory, which the command's `--root` overrides, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `view.repo`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `view.repo`'s own files and call `clm-view-repo` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary. What `clm status` and `clm view` print is stated by `view`'s requirements, and what `clm pending` prints by `check.coverage`'s, whose crates lie below this one and can't call it: the call SHALL write the text those crates return for the repository unchanged, so that their tests, which assert that text from their own crates, and this component's, which assert the stream and the exit status, prove the commands between them (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `view.repo`'s own files
- WHEN it calls `clm-view-repo`'s call with the arguments of `clm status` and two buffers
- THEN the buffers hold what `clm status` prints to standard output and to standard error there, and the call returns the status `clm status` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm status` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: text from the crate below
- GIVEN a fixture repository
- WHEN a test calls the call with the arguments of `clm status`
- THEN the output buffer holds exactly the text `clm-view` returns for that repository, and the call returns 0

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: reads-the-repository
`clm status` and `clm view <name>` SHALL describe the working tree and the local refs only. They SHALL never fetch, so a claim on `origin` that the last fetch didn't bring isn't shown; they SHALL print their output when `origin` can't be reached; and they SHALL read no ledger. The checkout they hand `view` for the checkout line (`view/status-report`, `view/view-command`) SHALL be `HEAD`'s hash cut to its first 7 hex digits, whatever length git abbreviates to, with the number of commits `refs/remotes/origin/main` has that `HEAD` doesn't contain when that ref exists, 0 included; they SHALL hand it none when `HEAD` can't be read, as outside a git repository or before the first commit. The size report they hand `view` for `clm status` and `clm view spec` (`view/status-report`, `view/spec-view`, `oversized-is-a-state`) SHALL be `check`'s `size` of the working tree under the budgets the call is passed, so a component is oversized when its spec or its code is over those budgets, or over its own `size_budget` where its frontmatter sets one. When the budgets can't be loaded or the sizes can't be measured, as outside a git repository, `clm status` and `clm view spec` SHALL print `size not checked: <error>` on standard error, hand `view` no size report and exit as they otherwise would. Each test that showed any of this from the top-level test files of `crates/clm/tests/` SHALL sit in `view.repo`'s own files under its own name and call `clm-view-repo`'s calls: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: checkout line
- GIVEN a repository whose `HEAD` is two commits behind its remote-tracking ref `origin/main`, and an `origin` that can't be reached
- WHEN `clm status` runs, then again with `HEAD` at `origin/main`, and then with no remote-tracking ref `origin/main`
- THEN each exits 0; the first's line after `CLM status` starts `  commit ` and ends `⚠ 2 behind origin/main`, the second's is `  commit ` followed by `HEAD`'s hash cut to its first 7 hex digits and nothing more, and the third's starts `  commit ` and holds no `behind`

#### Scenario: checkout behind on the chart
- GIVEN a repository whose `HEAD` is three commits behind its remote-tracking ref `origin/main`, and an `origin` that can't be reached
- WHEN `clm view flight`, `clm view flight --format svg` and `clm view flight --format json` run, and then `clm view spec --format json` with `HEAD` at `origin/main`
- THEN each exits 0; the chart's first line starts `commit ` and ends `⚠ 3 behind origin/main`, the SVG contains the text `3 behind origin/main`, the first JSON's `checkout` has `behind` 3 and `commit` the hash of `HEAD` cut to its first 7 hex digits, and the last JSON's `checkout` has `behind` 0

#### Scenario: seven digits whatever git abbreviates to
- GIVEN a repository whose `core.abbrev` is 12
- WHEN `clm status` runs
- THEN the line after `CLM status` is `  commit ` followed by the first 7 hex digits of `HEAD`'s hash and nothing more

#### Scenario: no commit to name
- GIVEN a directory that is no git repository, and then a git repository with no commit yet
- WHEN `clm status` runs in each, and `clm view spec` and `clm view spec --format json` in the first
- THEN the line after `CLM status` starts `  components `, the chart holds no `commit `, and the JSON has no `checkout`

#### Scenario: never fetches
- GIVEN a repository with pending components `a` and `z`, a remote-tracking ref `origin/impl/a`, and an `origin` holding a branch `impl/z` that no fetch has brought
- WHEN `clm status` and `clm view flight --format json` run
- THEN the status counts `claims 1 live` and lists only `a`, and the view gives `z` as `ready`

#### Scenario: offline
- GIVEN a repository whose `origin` is unreachable, with pending component `x` and a remote-tracking ref `origin/impl/x` committed 1 hour ago
- WHEN `clm view flight` runs
- THEN it exits 0 and shows `x` as claimed

#### Scenario: no ledger read
- GIVEN a repository whose remote-tracking ledger ref holds a `landed` record and a line that is no record
- WHEN `clm status` runs
- THEN it exits 0 with nothing on standard error, and its output holds no `review`

#### Scenario: oversized node
- GIVEN component `a` whose code is over its budget with no `size_budget` set, component `b` within its budgets, and component `c` whose code is over the default budget but within the `size_budget` its frontmatter sets
- WHEN `clm status` runs
- THEN its `components` summary line ends `, 1 oversized`, `a`'s row holds the cell `oversized`, and neither `b`'s row nor `c`'s holds such a cell

#### Scenario: oversized on the chart
- GIVEN component `a` whose spec is over its budget with no `size_budget` set, and component `b` within its budgets
- WHEN `clm view spec --format json` runs, and then `clm view spec`
- THEN `a` has `oversized` true and `b` false, and `a`'s chart line holds `⚠ oversized` while `b`'s holds no `⚠`

#### Scenario: sizes that can't be checked
- GIVEN a repository whose settings don't load
- WHEN `clm status` runs, and then `clm view spec --format json`
- THEN each exits 0 with a standard error starting `size not checked: `, the status holds no `oversized`, and no component of the JSON carries `oversized`

### Requirement: arguments-and-exit
`clm-view-repo`'s calls SHALL decide how `clm status`, `clm view` and `clm pending` take their arguments and how they exit. `clm view` SHALL fit its chart to 100 columns when no `--width` is given. It SHALL exit 2 with a usage error on standard error for a view it doesn't have, naming `spec`, `flight`, `blockers` and `queue`; for a format it doesn't have, `png` and `pdf` among them; for `--width 0`; and for `--format svg` of `queue`, naming `svg`. When the spec graph has violations, each of the three commands SHALL print them on standard output in place of what it otherwise prints there and exit 1; otherwise `clm status` and `clm view` SHALL exit 0. `clm pending [<component>]` SHALL exit 0 when it prints its list, an empty one included, and SHALL print the violation `check`'s `pending_for` gives (`check/pending-narrows-on-a-graph`) and exit 1 for a `<component>` the spec graph doesn't hold, one a live change removed included. `clm view <name> --format json` SHALL print the model `clm-view`'s library gives for the repository's graph, markers, claim refs and time, as `clm-view` renders it with the checkout the command read (`view/view-library`). Each test that showed any of this from the top-level test files of `crates/clm/tests/` SHALL sit in `view.repo`'s own files under its own name and call `clm-view-repo`'s calls: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: unknown view
- WHEN `clm view everything` runs, then `clm view spec --format png`, and then `clm view queue --format svg`
- THEN each exits 2

#### Scenario: default width
- GIVEN a component whose ID is 60 characters long and one whose ID is 120 characters long
- WHEN `clm view spec` runs
- THEN its widest line is 100 columns, holding the first ID whole and the second cut with `…`

#### Scenario: an invalid graph
- GIVEN a repository with a component whose frontmatter lacks its required keys
- WHEN `clm status`, `clm view spec` and `clm pending a` run
- THEN each exits 1 and prints `model/valid-frontmatter`, and the first prints no `CLM status`

#### Scenario: a component the graph doesn't hold
- GIVEN a repository with no component `nope`, and no component `g`, which an `applied` change's delta removed with `remove: true`
- WHEN `clm pending nope` runs, and then `clm pending g`
- THEN each exits 1, the first printing ``unknown component `nope` ``

#### Scenario: command matches library
- GIVEN a repository with a pending component and a claim ref
- WHEN `clm view flight --format json` runs, and a test calls the library with the same graph, markers, refs and time
- THEN both give the same JSON once the command's `checkout` key is removed

#### Needs
- check/pending-narrows-on-a-graph
