---
component: view.repo
base: { contract: "e0e8cf8c5b7b39aa", design: "ba6c2af7508e1719" }
---

## MODIFIED Contract
Crate `clm-view-repo`: the `clm status`, `clm view` and `clm pending` commands' reading of the repository, for `view`'s models and renderers (`views-rendered-by-clm`). `status(root, budgets)` and `view(root, name, format, width, source, err)` load the graph and markers from the working tree (`Source::Tree`) or, for `clm view --at <rev>`, from that commit's tree (`Source::At`, `view-at-rev`), read the checkout, the claim and `propose/*` refs and the time, and return what `view` renders, or the spec graph's violations, or why `<rev>` couldn't be read; `queue(root, format, width, source, err)` does the same for `clm view queue` from the build loop's stored plan (`view-queue`). `pending(root, component, at, err)` gives `clm pending [<component>] [--at <rev>]` its commit line and its pending lines (`pending-names-its-commit`). `view`, `queue` and `pending` write to `err` what the command prints on standard error beside its output; `status` writes it to the process's standard error. It never fetches and writes no file.

`clm-view-repo` offers each of `clm status`, `clm view` and `clm pending` as a call, `status_command`, `view_command` and `pending_command` (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, `status_command` and `view_command` also the budgets the repository's settings give a component that sets none (`NodeSize`), and `view_command` the budgets a commit's `clm.toml` text gives (`BudgetsFrom`), and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

## MODIFIED Design
**At a commit** (`view-at-rev`). `--at <rev>` swaps only where the files come from. The commit's blobs are read from the local objects in memory with `clm_git::tree_blobs` (`vcs/tree-blobs`) rather than extracted as `clm next` extracts a tree, which keeps the cost low enough for the pane's 30-second redraw. They go to the same in-memory calls the `web` Worker uses (`clm_model::load_files`, `clm_coverage::markers_in`, `clm_check::size_of`, `clm_check::linguist_excluded`), so `--at` marks what `clm view spec` would at that commit. The budgets come from that commit's `clm.toml` alone (`clm_config::node_size_from`): the view describes the commit, so `clm.local.toml` and the environment don't apply.

**`<age>`.** Git has no command that says when a remote was last fetched, and the reflog of `refs/remotes/origin/main` says when the ref last moved and doesn't apply to a `<rev>` that is no ref, so the `FETCH_HEAD` files are read directly. `FETCH_HEAD` is per worktree while `origin/*` is shared by every worktree, so every worktree's file counts (`vcs/common-dir`): one removed without `git worktree prune` recorded a real fetch of the shared refs, and a fetch of another remote or a `git pull` writes `FETCH_HEAD` too.

**Pending** (`pending-names-its-commit`). stdout stays the list `check.coverage/pending-report` defines, so the commit line goes to stderr, in `clm-view`'s `text` form. For `--at` the graph and markers are handed to `clm_coverage::pending` and narrowed with `clm_check::pending_for` (`check/pending-narrows-on-a-graph`), the function `pending_in` itself narrows with, so an unknown component gives the same violation either way.

**What the commands decide** (`commands-live-in-their-crates`). `view` states what `clm status` and `clm view` draw for the graph, markers, claim refs, checkout and size report they are handed, and `check.coverage` the lines `clm pending` lists; both crates lie below this one. What only the commands do is this component's: what they read from the repository and never read (`reads-the-repository`), and how they take their arguments and exit (`arguments-and-exit`). `clm-view-repo` is handed the budgets, since it loads no settings, so that `clm status` marks a component by `clm.toml`'s budget is the root's to show (`clm/routes-only`).

## MODIFIED Requirements

### Requirement: view-at-rev <!-- base: 3e653d69cad50de1 -->
`clm view <name> --at <rev>` SHALL print the view of `<rev>`'s commit rather than of the working tree: its graph, markers, changes and backlog, and for `spec` its size budgets and size report, all from that commit's tree, read from the local objects without fetching and without writing any file, with claims from the remote-tracking refs as without `--at`. When the CLM root lies below the repository's top, it SHALL read only the commit's files under the root; a file the commit's `.gitattributes` marks generated or vendored SHALL count toward no size, and a binary file, as git judges it, SHALL hold no marker. For `spec` it SHALL size by that commit's `clm.toml` alone, the defaults when the commit holds none; when that file doesn't load it SHALL print `size not checked: <error>` on standard error, mark no component oversized and exit as it otherwise would. For a `<rev>` that names no commit, `clm view --at` and `clm pending --at` SHALL exit 1 naming it, a `<rev>` that begins with `-` being read as a revision, never as an option. When `<rev>`'s commit can't be read, as outside a git repository or when a blob of its tree isn't in the local objects, `clm view --at` and `clm pending --at` SHALL exit 1 with a standard error naming `<rev>` and why, and fetch nothing. In place of the checkout line, every form SHALL name the commit as `view/view-command` names a commit, with: `<rev>`, the commit's hash cut to its first 7 hex digits, the time of the newest `FETCH_HEAD` of the clone when one exists, the main checkout's or any linked worktree's, one removed without `git worktree prune` included, and the checked-out branch, or none when `HEAD` is detached. Without `--at` the output SHALL be as `view/view-command` says.

#### Scenario: ahead of the checkout
- GIVEN a repository whose `HEAD` has component `a` with one requirement that no test cites, and whose remote-tracking ref `origin/main` is one commit ahead, adding a test that cites it at its current version
- WHEN `clm view flight` and `clm view flight --at origin/main` run
- THEN the first lists `a` as pending and the second doesn't, and the working tree and the index are unchanged

#### Scenario: header
- GIVEN that repository on branch `impl/a`, with a `FETCH_HEAD` modified 3 minutes ago
- WHEN `clm view spec --at origin/main` runs, and then `clm view spec --at origin/main --format json`, and then, with `HEAD` detached and no `FETCH_HEAD` in the main checkout or any linked worktree, `clm view spec --at origin/main` again
- THEN the chart's first line is `origin/main `, the first 7 hex digits of `origin/main`'s hash, ` · fetched 3m ago · checkout impl/a`; the JSON has no `checkout` and an `at` with `rev` `origin/main`, that `commit` and `branch` `impl/a`; and the last chart's first line ends `· checkout detached` and holds no `fetched`

#### Scenario: fetched from another worktree
- GIVEN that repository with a linked worktree on branch `impl/a` whose own `FETCH_HEAD` was modified 30 hours ago, and the main checkout's `FETCH_HEAD` modified 1 minute ago; and then, that linked worktree's still 30 hours old, a second linked worktree whose `FETCH_HEAD` was modified 2 minutes ago and whose directory is deleted without `git worktree prune`, with the main checkout's `FETCH_HEAD` modified 30 hours ago
- WHEN `clm view spec --at origin/main` runs in the linked worktree, and then in the main checkout
- THEN the first chart's first line holds ` · fetched 1m ago`, and the second's ` · fetched 2m ago`

#### Scenario: no such commit
- WHEN `clm view spec --at nowhere` runs, and then `clm view spec --at=--all` and `clm pending --at=--all`
- THEN each exits 1, the first's stderr naming `nowhere` and the others' naming `--all`

#### Scenario: a root below the repository's top
- GIVEN a CLM root `clm/` inside a larger repository, its component `a` with requirement `x`, paths `src/**` and a code budget of 10 tokens, whose committed `clm/src/gen.rs` and `clm/src/vendor.rs`, 400 bytes each, the committed top `.gitattributes` marks `linguist-generated` and `linguist-vendored`, a working tree whose top `.gitattributes` has since been deleted, a committed binary file under `clm/` holding a NUL byte and a marker citing `a/x` at its current version, and a committed backlog item `q` naming `a` that the working tree has since deleted, with `origin/main` at that commit
- WHEN `clm view spec --at origin/main --format json` runs in `clm/`, and then again, and `clm view spec --format json`, once `origin/main` also holds a committed 400-byte `clm/src/latin.rs` that isn't UTF-8 and holds no NUL byte
- THEN the first shows `a` not oversized, with 0 current and the backlog item `q`, and the other two each show `a` oversized

#### Scenario: a commit's own settings
- GIVEN a working tree whose `clm.toml` sets a spec budget of 1 token, and an `origin/main` holding no `clm.toml`; then an `origin/main` whose `clm.toml` sets a spec budget of 1 token and a working tree with none; then an `origin/main` whose `clm.toml` doesn't parse
- WHEN `clm view spec --format json` and `clm view spec --at origin/main --format json` run in each
- THEN in the first `a` is oversized without `--at` and not with it; in the second it is oversized with `--at` and not without; in the third the run with `--at` exits 0, its stderr starts `size not checked: `, and no component of its JSON carries `oversized` true

#### Scenario: a commit it can't read
- GIVEN a directory that is no git repository, and then a partial clone, made with `--filter=blob:none` from an origin that allows it, whose `origin/main` commit's `specs/SPEC.md` blob was never fetched
- WHEN `clm view spec --at HEAD` and `clm pending --at HEAD` run in the first, and `clm view spec --at origin/main` in the second
- THEN each exits 1 with a stderr naming the revision it was given, and the second leaves the clone's objects unchanged

#### Serves
- capability/see-project-state

#### Needs
- vcs/tree-blobs
- vcs/common-dir
- view/view-command

### Requirement: view-queue <!-- base: 7aa0fdff4e4ea2df -->
`clm view queue` SHALL render the plan the build loop last stored, with its runs as the loop file holds them now (`orchestrator.loop/plan-snapshot`), never fetching and writing no file, beginning with the checkout line as the other views have it (`views-name-their-commit`), and SHALL print on standard error `plan computed <n>m ago at <commit>`, `<n>` the whole minutes since the plan was computed and `<commit>` the `origin/main` commit the loop stored with the plan, as it stored it; with a stored plan, `--at` SHALL change only the checkout line. With no stored plan it SHALL render, in priority order under next up, the ready, unclaimed components of the queue `orchestrator/queue-from-refs` defines, read from the working tree or, with `--at <rev>`, from that commit's tree, each as `build:<component>`, and nothing under running, waiting on you or waiting on work; the loop line and each entry's reason SHALL be: with no loop file, as outside a git repository, a loop not held and `no loop has run on this clone`; with a loop file that holds no plan yet, `no plan stored yet` and the loop line for the file's state, a stop requested with its mode when the file sets one, a loop held, renewed `<n>` minutes ago, while its lease holds, and a loop not held otherwise; with a loop file it can't read, a loop not held and `loop file not read`, with `loop file not read: <why>` on standard error.

#### Scenario: no loop yet
- GIVEN a clone with no `<git common dir>/clm/loop` and component `a` ready, and then a directory that is no git repository holding the same specs
- WHEN `clm view queue` runs in each
- THEN each exits 0, shows `loop not held`, lists `build:a` under `next up` with the reason `no loop has run on this clone`, and writes no file

#### Scenario: the loop's plan
- GIVEN a loop running `build:a`, whose stored plan, computed now at `origin/main` commit `abc`, holds a fold batch `x` waiting for approval
- WHEN `clm view queue --at origin/main --format json` runs, and then `clm view queue`
- THEN the first's `running` holds `build:a` and its `waiting_on_you` holds `fold:x`, and the second's standard error is `plan computed 0m ago at abc` and its standard output holds no `plan computed`

#### Scenario: a loop with no plan yet
- GIVEN component `a` ready and a loop file whose lease was renewed now and which holds no plan; then the same file with a drain stop set; then a loop file that isn't JSON
- WHEN `clm view queue` runs on each
- THEN each exits 0 and lists `build:a` under `next up`; the first shows `loop held, renewed 0m ago` and the reason `no plan stored yet`, the second `stop requested: drain` and `no plan stored yet`, and the third `loop not held` and `loop file not read`, with a standard error starting `loop file not read: `, and the third's loop file is unchanged

#### Serves
- capability/watch-beside-chat
- capability/build-from-spec

#### Needs
- orchestrator.loop/plan-snapshot
- view/queue-view

### Requirement: commands-in-crate <!-- base: 975b834f32da4103 -->
`clm-view-repo` SHALL offer each of `clm status`, `clm view` and `clm pending` as a call of its own that takes the command's arguments as the command line gives them, the repository root the command line found from the current directory, which the command's `--root` overrides, the environment as a lookup, a stream each for standard output and standard error, and, for `clm status` and `clm view`, the budgets the repository's settings give a component that sets no `size_budget`, or why they don't load, and for `clm view` the budgets a commit's `clm.toml` text gives, for `--at`. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2; when what the command prints on the output stream, `--help`'s usage included, can't be written it SHALL return 1. For `--help` the call SHALL write the command's usage on the output stream, nothing on the error stream, and return 0: `clm status`'s names `--root`; `clm view`'s its `<NAME>` with `spec`, `flight`, `blockers` and `queue`, `--format` with `text` (the default), `ansi`, `svg` and `json`, `--width` with its default 100, `--at` and `--root`; and `clm pending`'s `[COMPONENT]`, `--at` and `--root`. What `clm status` and `clm view` print is stated by `view`'s requirements (`view/status-report`, `view/view-command`), and what `clm pending` prints by `check.coverage/pending-report`: the call SHALL write that text for the repository unchanged (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN `clm status`'s call is given its arguments and two buffers, and then again once a component's `SPEC.md` no longer parses
- THEN the first returns 0 with an output buffer starting `CLM status` and an empty error buffer, and the second returns 1 with the output buffer holding the spec graph's violations, one per line

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is given an option `clm status` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the caller's process goes on

#### Scenario: text from the crate below
- GIVEN a fixture repository
- WHEN the call is given the arguments of `clm status`
- THEN the output buffer holds exactly the status `view/status-report` defines for that repository's graph, markers, claim refs, checkout and size report, and the call returns 0

#### Scenario: help
- WHEN the calls of `clm status`, `clm view` and `clm pending` are each given `--help`
- THEN each returns 0 with an empty error buffer, and the output buffer names each option and value listed for its command

#### Scenario: an output it can't write
- GIVEN a fixture repository
- WHEN `clm status`'s call is given an output stream whose every write fails, and then `clm view`'s call is given that stream and `--help`
- THEN each returns 1 and the caller's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: reads-the-repository <!-- base: 8ad0cdee212a617c -->
`clm status` and `clm view <name>` SHALL describe the working tree and the local refs only. They SHALL never fetch, so a claim on `origin` that the last fetch didn't bring isn't shown; they SHALL print their output when `origin` can't be reached; and they SHALL read no ledger. The checkout line they print (`view/status-report`, `view/view-command`) SHALL name `HEAD`'s hash cut to its first 7 hex digits, whatever length git abbreviates to, and, when `refs/remotes/origin/main` exists, the number of commits it has that `HEAD` doesn't contain, 0 included; they SHALL print no checkout line, and the JSON no `checkout`, when `HEAD` can't be read, as outside a git repository or before the first commit. When `refs/remotes/origin/main` exists but can't be compared with `HEAD`, `clm status`, and `clm view` and `clm pending` without `--at`, SHALL print `checkout not compared: <error>` on standard error, give the checkout line no count, and exit as they otherwise would. The components `clm status` and `clm view spec` mark oversized (`view/status-report`, `view/spec-view`, `oversized-is-a-state`) SHALL be those `clm size` reports as oversized for the working tree (`check/node-size`) under the budgets the call is passed, so a component is oversized when its spec or its code is over those budgets, or over its own `size_budget` where its frontmatter sets one. When the budgets can't be loaded or the sizes can't be measured, as outside a git repository, `clm status` and `clm view spec` SHALL print `size not checked: <error>` on standard error, mark no component oversized and exit as they otherwise would.

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

#### Scenario: a main it can't compare
- GIVEN a repository whose `refs/remotes/origin/main` names a commit its objects don't hold
- WHEN `clm status`, `clm view flight --format json` and `clm pending` run
- THEN each exits 0 with a standard error starting `checkout not compared: `; the status's line after `CLM status` is `  commit ` followed by `HEAD`'s hash cut to its first 7 hex digits and nothing more, and the JSON's `checkout` has no `behind`

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

### Requirement: arguments-and-exit <!-- base: b12cc5dec0c52943 -->
`clm-view-repo`'s calls SHALL decide how `clm status`, `clm view` and `clm pending` take their arguments and how they exit. `clm view` SHALL fit its chart to 100 columns when no `--width` is given, and SHALL take a `--width` from 1 to 65535. It SHALL exit 2 with a usage error on standard error for a view it doesn't have, naming `spec`, `flight`, `blockers` and `queue`; for a format it doesn't have, `png` and `pdf` among them; for a `--width` outside 1 to 65535, `0` among them; and for `--format svg` of `queue`, naming `svg`. When the spec graph it reads, the commit's under `--at`, has violations, each of the three commands SHALL print them on standard output in place of what it otherwise prints there and exit 1; otherwise `clm status` and `clm view` SHALL exit 0. `clm pending [<component>]` SHALL exit 0 when it prints its list, an empty one included, and SHALL print the violation `check/pending-narrows-on-a-graph` states and exit 1 for a `<component>` the spec graph it reads doesn't hold, one a live change removed included. `clm view <name> --format json` SHALL print the model `view/view-library` defines for the repository's graph, markers, claim refs and the current time, with the `checkout` `view/view-command` adds for the checkout the command read.

#### Scenario: unknown view
- WHEN `clm view everything` runs, then `clm view spec --format png`, `clm view spec --width 0`, `clm view spec --width 70000`, and then `clm view queue --format svg`
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
- WHEN `clm view flight --format json` runs, and the `view/view-library` model is computed for the same graph, markers, refs and time
- THEN both give the same JSON once the command's `checkout` key is removed

#### Needs
- check/pending-narrows-on-a-graph

## REMOVED Requirements

### Requirement: shared-plumbing <!-- base: c3db052e844b6f8b -->
