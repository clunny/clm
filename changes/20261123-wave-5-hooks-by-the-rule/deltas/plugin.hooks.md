---
component: plugin.hooks
base: { contract: "0691d7a33449eb8b", design: "ca55245187791cd7" }
---

## MODIFIED Contract
The rules behind `clm hook session-start`, `clm hook autosync` and `clm defect` (`hook-logic-in-clm`): what the session-start hook prints, when autosync runs, and how a defect report is filed. `plugin`'s hook files wire each agent harness's events to `clm hook <name>`; this component and its child `plugin.hooks.guard` decide what they do, `clm hook guard` being `plugin.hooks.guard`'s.

Crate `clm-hooks` offers each of its three commands as a call (`commands-in-crate`, `commands-live-in-their-crates`), which takes the command's arguments as the command line gives them, the CLM repository its caller found from the current directory with `clm_root` (or none), the directory it runs in, the text on standard input, the environment as a lookup, through which it reads the settings, and a stream each for standard output and standard error, and returns the exit status:
- `defect_command(args, root, cwd, stdin, env, out, err)`: the settings of `root` say where the report goes; a backlog item is written in `root`, and an issue is filed, or kept, from `cwd`;
- `session_start_command(args, root, cwd, stdin, env, out, err, budgets)`, `budgets` being `view.repo`'s `NodeSize`, the budgets of a component that sets none, as `clm status` is given them;
- `autosync_command(args, root, cwd, stdin, env, out, err)`.

The two hooks read the agent harness's event from `stdin` and find their repository from its `cwd`; they take `root` and `cwd` only because every command's call does. `crates/clm` routes each command to its call and adds nothing to what it writes.

It offers too:
- `DefectArgs` and `HookArgs`, the commands' arguments as a parser the root's command line flattens in;
- `clm_root(cwd)`, the first of `cwd` and its ancestors that has `specs/SPEC.md`, looking no higher than the first that holds `.git` (a directory, or a worktree's file), or none: the one way the root, the hooks, `clm defect` and `clm-guard` find the CLM repository for a directory.

It offers no other public call.

## MODIFIED Design
Crate `clm-hooks`, called from `crates/clm`'s `clm hook session-start`, `clm hook autosync` and `clm defect` subcommands. The guard is crate `clm-guard`, the child `plugin.hooks.guard`, with the shell reading as its child `plugin.hooks.guard.shell` (`guard-split`); `clm-guard` finds the CLM root with this crate's `clm_root` and loads settings there under its call's own environment lookup, and this crate depends on nothing of it. That `clm check` accepts an item `clm defect` wrote is the root's to prove (`clm/check-across-components`).

**Session start** (`session-status`). The status is `view.repo`'s `status_command` with no arguments, its output written to a buffer the hook keeps only when the call returns 0 and its error stream the hook call's own, with the `budgets` the hook call was given, so this crate builds no budgets of its own. The upgrade line is `upgrade`'s `pending_in` over `clm-init`'s `migrations()` and `supersedes()`, with the `skip_migrations` of the settings the hook loaded.

**Autosync** (`autosync-hook`). The snapshot is `sync`'s `autosync_command` with no arguments, the hook call's environment lookup, and streams that discard what it writes, so the hook stays silent whatever happens.

**Defects** (`defect-reports`). With `ask` (`defect-issue`, `defects-file-on-github`) the issue goes through `clm_host::adapter(Host::Github)`, never the adapter of the `host` setting, and a report it can't file is kept under the directory `clm_git::common_dir` gives for the current directory. A backlog item and a kept report are each created with `create_new`, trying `-2`, `-3`... in turn, so two reports with one title at once can't overwrite each other.

## MODIFIED Requirements

### Requirement: session-status <!-- base: 1e9095017ddb4664 -->
`clm hook session-start` SHALL find the CLM repository for the event's `cwd`: the first of `cwd` and its ancestors that has `specs/SPEC.md`, looking no higher than the first that holds `.git`. When it finds one, it SHALL print `clm status` for it, under a line saying it is the CLM state, as `clm status` prints it given the budgets the call is given (`commands-in-crate`), and SHALL write to its error stream what `clm status` writes to standard error there given those budgets. When it finds none, or the event isn't JSON or holds no `cwd`, it SHALL print nothing. When `clm status` fails there, it SHALL print no status and no `defect-rule` paragraph, only the line `session-upgrade-reminder` states when there is one. It SHALL exit 0, save as `commands-in-crate` says for `--help`, arguments it can't parse and a stream it can't write.

#### Scenario: CLM repository
- GIVEN a `SessionStart` event whose `cwd` has a valid `specs/`
- WHEN `clm hook session-start` runs
- THEN its output contains the line `CLM status` and the `changes` and `claims` sections, and it exits 0

#### Scenario: subdirectory
- GIVEN a `SessionStart` event whose `cwd` is `crates/foo` inside a CLM repository
- WHEN `clm hook session-start` runs
- THEN its output contains the repository's `changes` section, and it exits 0

#### Scenario: other repository
- GIVEN a `cwd` inside a git repository with no `specs/SPEC.md` up to its root, nested in a directory that has one
- WHEN `clm hook session-start` runs
- THEN it prints nothing and exits 0

#### Scenario: the status's budgets and error stream
- GIVEN a CLM repository with one component
- WHEN `clm-hooks`' call of `clm hook session-start` is given an event whose `cwd` is that repository, with budgets that give a component that sets none a spec budget of 1 token, and then with budgets that fail with the message `bad`
- THEN with the first its output holds `1 oversized` and its error stream is empty; with the second its output holds the status with no `oversized`, its error stream starts `size not checked: bad`, and it returns 0

#### Scenario: unreadable event
- GIVEN the events `not json` and `{}`
- WHEN `clm hook session-start` runs with each
- THEN it prints nothing and exits 0

#### Scenario: status fails
- GIVEN a CLM repository whose `specs/SPEC.md` doesn't parse, with no migration pending, and then the same repository whose `clm.toml` sets `upgraded` to a migration this `clm` doesn't hold
- WHEN `clm hook session-start` runs in each
- THEN the first prints nothing; the second prints only the line saying the project needs a newer `clm`, with no `CLM status` and no `clm defect`; and each exits 0

#### Serves
- capability/see-project-state

### Requirement: autosync-hook <!-- base: 3a75312d8188cced -->
`clm hook autosync` SHALL find the CLM repository for the event's `cwd` as `session-start` does and run `clm autosync` there (`sync/autosync`), with the environment the call is given, unless `config`'s `autosync` setting for that repository is `false` or can't be loaded. It SHALL print nothing and exit 0 in every case, including when there is no repository, the push fails, or its input can't be parsed, save as `commands-in-crate` says for `--help` and arguments it can't parse.

#### Scenario: spec branch
- GIVEN a `Stop` event whose `cwd` is a CLM repository on branch `spec/x` with an uncommitted delta
- WHEN `clm hook autosync` runs
- THEN `origin/autosync/spec/x` holds the delta, it prints nothing, and it exits 0

#### Scenario: turned off
- GIVEN the same repository and `CLM_AUTOSYNC=0`
- WHEN `clm hook autosync` runs
- THEN it pushes nothing and exits 0

#### Scenario: turned off in the repository
- GIVEN the same repository with a `clm.toml` that sets `autosync = false`
- WHEN `clm hook autosync` runs
- THEN it pushes nothing and exits 0

#### Scenario: push fails
- GIVEN a CLM repository on `spec/x` whose `origin` can't be reached
- WHEN `clm hook autosync` runs
- THEN it prints nothing and exits 0

#### Scenario: the call's environment
- GIVEN a CLM repository on `spec/x` with an uncommitted delta whose `origin` is an `ssh://` URL, and an executable that records that it ran and exits 1
- WHEN `clm-hooks`' call of `clm hook autosync` is given a lookup whose `GIT_SSH_COMMAND` names that executable, while the process's own environment sets none
- THEN the executable ran, the call wrote nothing to either stream, and it returned 0

#### Serves
- capability/wrap-up-session
- capability/work-concurrently

### Requirement: defect-rule <!-- base: 8551aeefbb6a5204 -->
When `clm hook session-start` prints the CLM state and the repository's `defects` setting isn't `off`, it SHALL add a paragraph that names `clm defect`, after the claims. When the setting is `backlog` the paragraph SHALL also name the backlog route: `backlog/` and `clm claim <component> --fix backlog/<slug>`. With `ask` it SHALL name none of that route, and SHALL name the `<<'EOF'` heredoc form of the `clm defect` line, which with `backlog` it SHALL not name. When the setting is `off` it SHALL add nothing. When the settings can't be loaded it SHALL print their error instead, and still exit 0.

#### Scenario: asking
- GIVEN a CLM repository with no settings
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect` after the `claims` section, names `<<'EOF'`, and doesn't name `--fix`

#### Scenario: backlog
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect`, `backlog/` and `clm claim <component> --fix backlog/<slug>`

#### Scenario: off
- GIVEN a CLM repository whose `clm.toml` sets `defects = "off"`
- WHEN `clm hook session-start` runs
- THEN its output doesn't mention `clm defect`

#### Scenario: bad settings
- GIVEN a CLM repository whose `clm.toml` sets `defects = "always"`
- WHEN `clm hook session-start` runs
- THEN its output names `clm.toml` and `defects`, and it exits 0

#### Scenario: the paragraph
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`, and then the same repository with no settings
- WHEN `clm hook session-start` runs in each
- THEN each output holds one paragraph naming `clm defect` after the `claims` section; the first names `backlog/` and no `<<'EOF'`, and the second names `<<'EOF'` and no `backlog/`

#### Serves
- capability/report-defects

### Requirement: defect-backlog <!-- base: 06439020b3404d6e -->
With `defects` set to `backlog`, `clm defect --title <title> [--component <component>]...` SHALL write `backlog/<slug>.md` with the title, `kind: bug`, the components under the key `components` (`model/component-keys`), the report from stdin as its body, and `origin: <change>` when the current branch is `spec/<change>` and the repository has a live change `<change>`, and print its path. Any other branch, `impl/*` ones included, SHALL give no `origin`. With no CLM repository, or a component a backlog item may not name (`model/backlog-components`: neither in its spec graph nor created by a `create: true` delta of a change outside the archive), it SHALL write nothing and exit 1. The slug SHALL be the title lower-cased and split on every character but an ASCII letter or digit, its words joined with `-` while it stays within 64 bytes (a first word longer than that cut to 64), or `defect` when the title has no ASCII letter or digit, with `-2`, `-3`... added when that file exists. The item's `title` SHALL read back as the title given, whatever characters it holds, and its body SHALL be the report ending in one newline. When it can't write the item, it SHALL write to standard error the path it tried and why, the report, and that the report wasn't filed, and exit 1.

#### Serves
- capability/report-defects

#### Needs
- model/backlog-components
- model/component-keys

#### Scenario: on a spec branch
- GIVEN a CLM repository with `defects = "backlog"` and a live change `20261022-x`, on branch `spec/20261022-x`
- WHEN `clm defect --title "Gate passes a stale task" --component gate` runs with a report on stdin
- THEN `backlog/gate-passes-a-stale-task.md` has `kind: bug`, `components: [gate]`, `origin: 20261022-x` and the report

#### Scenario: on an implementation branch
- GIVEN the same repository, on branch `impl/gate`, then on `impl/archive/20261022-x`, then on `impl/20261022-x/gate`
- WHEN `clm defect --title "x"` runs on each
- THEN each item it writes has no `origin`

#### Scenario: node a live change creates
- GIVEN the same repository, with a live change whose delta for component `fresh` has `create: true`
- WHEN `clm defect --title "Fresh misfires" --component fresh` runs with a report on stdin
- THEN it writes `backlog/fresh-misfires.md` with `components: [fresh]` and exits 0

#### Scenario: unknown node
- GIVEN the same repository
- WHEN `clm defect --title "x" --component nonesuch` runs
- THEN it writes nothing and exits 1 naming `nonesuch`

#### Scenario: not a CLM repository
- GIVEN a directory with no `specs/SPEC.md` and `CLM_DEFECTS=backlog`
- WHEN `clm defect --title "x"` runs
- THEN it writes nothing and exits 1

#### Scenario: taken slug
- GIVEN the same repository, where `backlog/gate-passes-a-stale-task.md` exists
- WHEN the same `clm defect` runs
- THEN it writes `backlog/gate-passes-a-stale-task-2.md`

#### Scenario: odd titles and reports
- GIVEN the same repository
- WHEN `clm defect` runs with each of the titles `word ` repeated eighty times, `!!!`, `Ça casse` and `Gate: "x" # y`, and once with the report `body` with no final newline
- THEN the first item's file name is at most 64 bytes before `.md` and ends in a whole `word`, and its `title` holds the whole title; the second is `backlog/defect.md`; the third is `backlog/a-casse.md`; the fourth's `title`, parsed as YAML, equals `Gate: "x" # y`; and the item whose report was `body` ends in `\n\nbody\n`

#### Scenario: backlog can't be written
- GIVEN the same repository, whose `backlog/` directory can't be written
- WHEN `clm defect --title "x"` runs with a report on stdin
- THEN it writes no file, its standard error holds `backlog/x.md`, the report and `wasn't filed`, and it exits 1

### Requirement: defect-issue <!-- base: 698b188d2d29ebd6 -->
With `defects` set to `ask`, `clm defect` SHALL file the report as an issue on `clunny/clm` through `host`'s `github` adapter, whatever the repository's `host` setting (`defects-file-on-github`), with the title and a body holding the report, the components, the `clm` version and the OS, and print the issue's URL. The body SHALL be `## What happened`, a blank line and the report, then `- Components: <ids>` when any are named, unchecked against the repository's specs, then `- clm: <version>` and `- OS: <os> <arch>`. When it can't file it (`gh` is missing or fails, or prints no URL), it SHALL keep it: write `# <title>`, a blank line and the issue's body to `clm/defects/<slug>.md` in the git common directory of the repository holding the current directory, the slug made from the title as `defect-backlog` makes it, `-2`, `-3`... included, then print the report, the reason, that file's absolute path and that it can be filed by hand on `clunny/clm`, and exit 1. When the current directory is in no git repository, or the file can't be written, it SHALL print the report, the reason and why it wasn't kept, and exit 1. With `defects` set to `off`, it SHALL file nothing, say reporting is off, and exit 0. When the settings can't be loaded, it SHALL file nothing, write `clm defect: settings: <why>` to standard error, and exit 1.

#### Scenario: filed
- GIVEN a repository with no settings, `host = "github"`, and a fake `gh` that records its arguments and prints a URL
- WHEN `clm defect --title "Sync loses a delta"` runs with a report on stdin
- THEN `gh` got `issue create`, `--repo clunny/clm` and the title, its body holds the report and the `clm` version, and the URL is printed

#### Scenario: filed from a project on another host
- GIVEN a repository whose `clm.toml` sets `host = "azure"`, then one that sets `host = "git"`, then one that sets `host = "none"`, each with the same fake `gh`
- WHEN the same `clm defect` runs in each
- THEN `gh` got `issue create` and `--repo clunny/clm` each time, and the URL is printed

#### Scenario: no access
- GIVEN a fake `gh` that fails with `Could not resolve to a Repository`
- WHEN the same `clm defect` runs
- THEN it prints the report and the error, `clm/defects/sync-loses-a-delta.md` in the git common directory holds `# Sync loses a delta` and the report, its absolute path is printed, and it exits 1

#### Scenario: no gh
- GIVEN a repository whose `clm.toml` sets `host = "git"` and no `gh` on `PATH`
- WHEN the same `clm defect` runs twice
- THEN each prints the report and says it wasn't filed, the reports are kept as `sync-loses-a-delta.md` and `sync-loses-a-delta-2.md` under `clm/defects/` in the git common directory, each run prints its file's path, and each exits 1

#### Scenario: from a linked worktree
- GIVEN a linked worktree of a repository, added with `git worktree add`, and a fake `gh` that fails
- WHEN the same `clm defect` runs in the worktree, and the worktree is then removed with `git worktree remove`
- THEN the kept report is under `clm/defects/` in the main repository's `.git` directory and is still there

#### Scenario: outside git
- GIVEN a directory in no git repository and a fake `gh` that fails
- WHEN the same `clm defect` runs
- THEN it prints the report and the error, writes no file, and exits 1

#### Scenario: off
- GIVEN `CLM_DEFECTS=off` and a fake `gh`
- WHEN `clm defect --title "x"` runs
- THEN `gh` isn't called, it says reporting is off, and it exits 0

#### Scenario: the issue's body
- GIVEN a repository with no settings and a fake `gh` that records its body and prints a URL
- WHEN `clm defect --title "x" --component nonesuch` runs, and then `clm defect --title "x"` with no `--component`
- THEN the first files the issue, its body starting `## What happened` followed by a blank line and the report, and holding `- Components: nonesuch`, `- clm: ` and `- OS: `; and the second's body holds no `- Components:` line

#### Scenario: bad settings
- GIVEN a repository whose `clm.toml` sets `defects = "always"`, and a fake `gh` that records its arguments
- WHEN `clm defect --title "x"` runs
- THEN `gh` isn't called, no file is written, its standard error starts `clm defect: settings: ` and names `clm.toml`, and it exits 1

#### Serves
- capability/report-defects
- architecture/host-adapters

### Requirement: session-upgrade-reminder <!-- base: c217417ca942e49a -->
When `clm hook session-start` prints the status of a CLM repository with a pending migration, it SHALL print after it one line saying how many `required` and how many `advised` migrations are pending, whether commands are blocked (when a `required` one the clone doesn't skip is pending), and to run `/clm:upgrade`. A skipped `required` migration SHALL be counted as skipped, not as blocking. It SHALL count, of every migration `init` holds (`init/all-migrations`), those `upgrade/pending-migrations` finds pending in the repository, so a migration `init` holds is counted. When the repository's `clm.toml` sets `upgraded` to a migration this `clm` doesn't hold, the line SHALL say the project needs a newer `clm`. It SHALL print no such line when none is pending, and SHALL still exit 0. It SHALL print the line even when `clm status` fails.

#### Scenario: advised drift
- GIVEN a CLM repository with one pending `advised` migration
- WHEN `clm hook session-start` runs
- THEN its output holds a line naming `1 advised` and `/clm:upgrade`, not `blocked`, and it exits 0

#### Scenario: required pending
- GIVEN a CLM repository with one pending `required` migration
- WHEN `clm hook session-start` runs
- THEN the line names `1 required`, `blocked` and `/clm:upgrade`, and it exits 0

#### Scenario: required but skipped
- GIVEN the same repository, with the migration in the clone's `skip_migrations`
- WHEN `clm hook session-start` runs
- THEN the line names `skipped` and not `blocked`

#### Scenario: up to date
- GIVEN a repository `clm init` has just set up
- WHEN `clm hook session-start` runs
- THEN no line names `/clm:upgrade`

#### Needs
- init/all-migrations
- upgrade/pending-migrations
- upgrade/require-current
- init/init-stamps-upgraded

#### Serves
- capability/upgrade-clm

### Requirement: hooks-per-agent-harness <!-- base: be580a3cfc9d8dc9 -->
`clm hook session-start` and `clm hook autosync` SHALL read the event on stdin as `agent-harness/hook-events` reads the hook events of the agent harness their `--agent-harness` names, `claude-code`, `codex` or `none`, take the event's `cwd` from it, and do what `session-status` and `autosync-hook` say. `clm hook session-start` SHALL print its text in the form that agent harness adds to the session's context: as plain text for `claude-code` and `none`, and for `codex` as Codex's session-start output, as its hooks documentation gives it, a JSON object whose `hookSpecificOutput` holds `hookEventName` `SessionStart` and the text as `additionalContext`, followed by a newline, printing nothing when it has no text. Given an `--agent-harness` that names none of the three, each SHALL print nothing and exit 0.

#### Scenario: Codex session start
- GIVEN a Codex session-start event whose `cwd` has a valid `specs/`
- WHEN `clm hook session-start --agent-harness codex` runs
- THEN Codex's session-start output carries the `CLM status` text, and the exit code is 0

#### Scenario: Codex end of turn
- GIVEN a Codex end-of-turn event whose `cwd` is a spec branch's checkout with uncommitted spec work
- WHEN `clm hook autosync --agent-harness codex` runs
- THEN it runs `clm autosync` there, its stdout is empty and the exit code is 0

#### Scenario: other agent harness names
- GIVEN an event whose `cwd` has a valid `specs/`
- WHEN `clm hook session-start` and `clm hook autosync` each run with `--agent-harness nonesuch`, `clm hook session-start` runs with `--agent-harness none`, and `clm hook session-start --agent-harness codex` runs on the event `not json`
- THEN with `nonesuch` each prints nothing on either stream and exits 0; with `none` the output starts `CLM state`; and with `codex` on `not json` it prints nothing and exits 0

#### Needs
- agent-harness/hook-events

#### Serves
- capability/work-from-any-agent-harness
- architecture/agent-harness-adapters

### Requirement: commands-in-crate <!-- base: 305d2b911960e947 -->
`clm-hooks` SHALL offer each of `clm defect`, `clm hook session-start` and `clm hook autosync` as a call of its own (the Contract's) that takes the command's arguments as the command line gives them, the CLM repository its caller found from the current directory (or none), the directory it runs in, the text on standard input, the environment as a lookup, through which it SHALL read the settings rather than from the process's environment, and a stream each for standard output and standard error, the call of `clm hook session-start` also taking the budgets `clm status` is given. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. Given `--help` it SHALL write the command's help to the output stream, nothing to the error stream, and return 0: its first line the command's description, ``Report a defect in CLM itself, the report on stdin, as the `defects` setting says: a backlog item, an issue on clunny/clm, or nothing`` for `clm defect`, ``SessionStart: print `clm status` and the rule for reporting CLM defects when the session's cwd is a CLM repository`` for `clm hook session-start` and ``Stop: push a snapshot of the spec work on a `spec/*` branch; silent, never fails. The `autosync` setting (`clm config get autosync`) turns it off`` for `clm hook autosync`; then an empty line; then a line starting `Usage: clm defect ` or `Usage: clm hook <name> `; then its options, `--title <TITLE>` and `--component <COMPONENT>` for `clm defect`, and for each hook `--agent-harness <AGENT_HARNESS>` with `[default: claude-code]`. When a write of what it prints on standard output fails, or of `clm defect`'s failure on standard error, the call SHALL return 2, whatever it has already done.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN `clm-hooks`' call of `clm defect` is given the arguments of `clm defect` and two buffers
- THEN the buffers hold what `clm defect` prints to standard output and to standard error there, and the call returns the status `clm defect` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm defect` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Scenario: help
- GIVEN each of the three calls
- WHEN it is given `--help`
- THEN it returns 0, its error buffer is empty, and its output buffer starts with the command's description, an empty line and `Usage: clm defect ` or `Usage: clm hook <name> `, and lists the command's options

#### Scenario: a stream it can't write
- GIVEN a CLM repository with `defects = "backlog"`, and an output stream whose every write fails
- WHEN the call of `clm defect --title "x"` is given it, and the call of `clm hook session-start` is given it with an event whose `cwd` is the repository
- THEN each returns 2, and `backlog/x.md` exists

#### Scenario: settings through the lookup
- GIVEN a CLM repository with no settings, whose process environment sets no `CLM_DEFECTS`
- WHEN the call of `clm defect --title "x"`, and the call of `clm hook session-start` with an event whose `cwd` is the repository, are each given a lookup that holds `CLM_DEFECTS=off`
- THEN `clm defect` files nothing, says reporting is off and returns 0, and the session start's output holds the `claims` section and no `clm defect`

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

## REMOVED Requirements

### Requirement: git-through-layer <!-- base: bf450838cc640c93 -->

### Requirement: says-component <!-- base: 5c2da288899624ae -->
