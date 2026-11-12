---
title: Hooks
status: active
owners: ["@clunny"]
depends_on: [config, model, sync, view, view.repo, host, vcs, upgrade, agent-harness]
paths: ["crates/clm-hooks/**"]
successor: ""
serves: [capability/guard-sessions, capability/work-from-any-agent-harness, architecture/approval-policy, architecture/guard-blocks-push-to-main, architecture/agent-harness-adapters]
---

## Contract
The rules behind `clm hook session-start`, `clm hook autosync` and `clm defect` (`hook-logic-in-clm`): what the session-start hook prints, when autosync runs, and how a defect report is filed. `plugin`'s `hooks.json` wires Claude Code's events to `clm hook <name>`; this component and its child `plugin.hooks.guard` decide what they do. `clm hook guard`, which asks the human to confirm `clm approve`, `clm capabilities decide` and a `clm defect` whose report leaves the project, and blocks pushes to `main` and release runs, is `plugin.hooks.guard`'s. Crate `clm-hooks`: `session_start(event)`, `autosync(event)` and `defect(...)`, with `settings(cwd)` and `clm_root(cwd)`, which find the CLM repository for a directory and load its settings the one way the hooks, the guard and `clm defect` share.

## Design
Crate `clm-hooks`, called from `crates/clm`'s `clm hook session-start`, `clm hook autosync` and `clm defect` subcommands. The guard is crate `clm-guard`, the child `plugin.hooks.guard`, with the shell reading as its child `plugin.hooks.guard.shell` (`guard-split`); `clm-guard` reads settings through this crate's `settings`, and this crate depends on nothing of it.

This component's tests are `crates/clm-hooks/tests/hooks.rs` and `git_layer.rs`, and, for what runs the built binary, `crates/clm/tests/hooks.rs` and `crates/clm/tests/defect.rs`.

**Defects** (`defect-reports`). `clm defect --title <title> [--component <component>]...` reads the report from stdin and loads the settings for the CLM repository found from the current directory, or none. By `defects`:
- `backlog` (`defect-backlog`): `clm check` validates the components as for any item.
- `ask` (`defect-issue`, `defects-file-on-github`): the issue's body is the report under `## What happened`, then the components, the `clm` version and the OS. It goes through `clm_host::adapter(Host::Github)`, never the adapter of the `host` setting. A report it can't file is kept in `clm/defects/` under the directory `clm_git::common_dir` gives for the current directory, named by the same slug function and `create_new` loop as a backlog report, so two reports with one title at once can't overwrite each other.
- `off`: reporting is off.

## Requirements

### Requirement: session-status
`clm hook session-start` SHALL find the CLM repository for the event's `cwd`: the first of `cwd` and its ancestors that has `specs/SPEC.md`, looking no higher than the first that holds `.git`. When it finds one, it SHALL print `clm status` for it, under a line saying it is the CLM state. Otherwise, or when status fails, it SHALL print nothing. It SHALL always exit 0.

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

#### Serves
- capability/see-project-state

### Requirement: autosync-hook
`clm hook autosync` SHALL find the CLM repository for the event's `cwd` as `session-start` does and run `clm autosync` there, unless `config`'s `autosync` setting for that repository is `false` or can't be loaded. It SHALL print nothing and exit 0 in every case, including when there is no repository, the push fails, or its input can't be parsed.

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

#### Serves
- capability/wrap-up-session
- architecture/autosync-policy

### Requirement: defect-rule
When `clm hook session-start` prints the CLM state and the repository's `defects` setting isn't `off`, it SHALL add a paragraph that names `clm defect`, says what counts as a CLM defect and what doesn't, says a report describes only the process and never the project's files, code, specs or output, and says a session that can't ask the human lists defects under `## CLM defects` in its PR body. When the setting is `backlog` the paragraph SHALL also say that reports go to `backlog/` as `kind: bug` items, and that a small defect needing no spec change is fixed in the moment rather than left filed: folded into the current branch, unreported, when it is an `impl/*` branch and the fix lies in its components (`fold-small-defer-large`), and otherwise reported with `clm defect`, naming with `--component` the component the fix lies in, and, once the current branch has landed and brought the item to `main`, fixed before the session ends on its own branch with `clm claim <component> --fix backlog/<slug>` (`code-fix-resolves-bug`); that only a defect that is a large amount of work or needs a spec change is left in the backlog; and that an unattended session folds what fits and reports the rest. With `ask` it SHALL say none of that, and SHALL say to send the report as the body of a `<<'EOF'` heredoc on the `clm defect` line, the one form the guard shows the human before the report leaves the project (`defect-ask-shows-report`). With `ask` it SHALL also say that when `clm defect` keeps a report it couldn't file (`defect-issue`), the session lists that report under `## CLM defects` in its PR body with the kept file's path, for the human to file by hand (`defects-file-on-github`). When the setting is `off` it SHALL add nothing. When the settings can't be loaded it SHALL print their error instead, and still exit 0.


#### Scenario: asking
- GIVEN a CLM repository with no settings
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect` and `## CLM defects` after the `claims` section, names `<<'EOF'` and `kept`, and doesn't name `--fix`

#### Scenario: backlog
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect`, `backlog/`, `fold-small-defer-large`, `--component`, `clm claim <component> --fix backlog/<slug>`, `landed` and `spec change`

#### Scenario: off
- GIVEN a CLM repository whose `clm.toml` sets `defects = "off"`
- WHEN `clm hook session-start` runs
- THEN its output doesn't mention `clm defect`

#### Scenario: bad settings
- GIVEN a CLM repository whose `clm.toml` sets `defects = "always"`
- WHEN `clm hook session-start` runs
- THEN its output names `clm.toml` and `defects`, and it exits 0

#### Serves
- capability/report-defects

### Requirement: defect-backlog
With `defects` set to `backlog`, `clm defect --title <title> [--component <component>]...` SHALL write `backlog/<slug>.md` with the title, `kind: bug`, the components under the key `components` (`model/component-keys`), the report from stdin as its body, and `origin: <change>` when the current branch is `spec/<change>` and the repository has a live change `<change>`, and print its path. Any other branch, `impl/*` ones included, SHALL give no `origin`. With no CLM repository, or a component a backlog item may not name (`model/backlog-components`: neither in its spec graph nor created by a `create: true` delta of a change outside the archive), it SHALL write nothing and exit 1. The slug SHALL be the title in kebab case, with `-2`, `-3`... added when that file exists.

#### Serves
- capability/report-defects

#### Needs
- model/backlog-components
- model/component-keys
- clm/component-flags

#### Scenario: on a spec branch
- GIVEN a CLM repository with `defects = "backlog"` and a live change `20261022-x`, on branch `spec/20261022-x`
- WHEN `clm defect --title "Gate passes a stale task" --component gate` runs with a report on stdin
- THEN `backlog/gate-passes-a-stale-task.md` has `kind: bug`, `components: [gate]`, `origin: 20261022-x` and the report, and `clm check` accepts it

#### Scenario: on an implementation branch
- GIVEN the same repository, on branch `impl/gate`, then on `impl/archive/20261022-x`, then on `impl/20261022-x/gate`
- WHEN `clm defect --title "x"` runs on each
- THEN each item it writes has no `origin`

#### Scenario: node a live change creates
- GIVEN the same repository, with a live change whose delta for component `fresh` has `create: true`
- WHEN `clm defect --title "Fresh misfires" --component fresh` runs with a report on stdin
- THEN it writes `backlog/fresh-misfires.md` with `components: [fresh]`, exits 0, and `clm check` accepts the item

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

### Requirement: defect-issue
With `defects` set to `ask`, `clm defect` SHALL file the report as an issue on `clunny/clm` through `host`'s `github` adapter, whatever the repository's `host` setting (`defects-file-on-github`), with the title and a body holding the report, the components, the `clm` version and the OS, and print the issue's URL. When it can't file it (`gh` is missing or fails, or prints no URL), it SHALL keep it: write `# <title>`, a blank line and the issue's body to `clm/defects/<slug>.md` in the git common directory of the repository holding the current directory, the slug made from the title as `defect-backlog` makes it, `-2`, `-3`... included, then print the report, the reason, that file's absolute path and that it can be filed by hand on `clunny/clm`, and exit 1. When the current directory is in no git repository, or the file can't be written, it SHALL print the report, the reason and why it wasn't kept, and exit 1. With `defects` set to `off`, it SHALL file nothing, say reporting is off, and exit 0.

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

#### Serves
- capability/report-defects
- architecture/host-adapters

### Requirement: git-through-layer
`clm-hooks` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `gh`, `run`, `run_within` or repo queries (`git-plumbing-layer`), and its own wrapper is gone (`no-dead-code`). It SHALL bound a call's time only through `run_within`'s limit, never by waiting on a channel for a thread that runs the call (`bounded-runs`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-hooks` reads `crates/clm-hooks/Cargo.toml` and every file under `crates/clm-hooks/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")`, `Command::new("gh")` or `recv_timeout`

#### Serves
- architecture/reuse-first

#### Needs
- vcs/bounded-runs
- host/git-through-layer

### Requirement: says-component
The text `clm hook session-start` adds to the CLM state and the reports `clm defect` writes or opens SHALL say component and components, never the old words, as `component-words` defines them: the paragraph names `clm defect --title <title> [--component <component>]...` and `--component`, never the old flag, and the issue body `clm defect` opens with `defects = "ask"` lists the components under `- Components:`. A test reads the string literals in `crates/clm-hooks/src/` and fails naming the file, the line and the word for each old word `component-words` doesn't keep.

#### Scenario: the paragraph
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN `clm hook session-start` runs
- THEN its paragraph names `[--component <component>]` and holds no old word

#### Needs
- clm/component-flags

#### Serves
- architecture/plain-terms

### Requirement: session-upgrade-reminder
When `clm hook session-start` prints the status of a CLM repository with a pending migration, it SHALL print after it one line saying how many `required` and how many `advised` migrations are pending, whether commands are blocked (when a `required` one the clone doesn't skip is pending), and to run `/clm:upgrade`. A skipped `required` migration SHALL be counted as skipped, not as blocking. When `upgrade`'s `pending` returns `TooNew`, the line SHALL say the project needs a newer `clm`. It SHALL print no such line when none is pending, and SHALL still exit 0. It SHALL print the line even when `clm status` fails.

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
- upgrade/pending-migrations
- upgrade/require-current
- init/init-stamps-upgraded

#### Serves
- capability/upgrade-clm

### Requirement: hooks-per-agent-harness
`clm-hooks` SHALL offer `session_start_hook(agent_harness, event)` and `autosync_hook(agent_harness, event)`, which find the event's `cwd` with `agent-harness`'s `parse_event` for that agent harness and do what `session-status` and `autosync-hook` say, returning stdout and an exit code of 0. `session_start_hook` SHALL write what it prints in the form that agent harness adds to the session's context: plain stdout for Claude Code, and Codex's session-start output as its hooks documentation gives it.

#### Scenario: Codex session start
- GIVEN a Codex session-start event whose `cwd` has a valid `specs/`
- WHEN `session_start_hook` runs for `Codex`
- THEN Codex's session-start output carries the `CLM status` text, and the exit code is 0

#### Scenario: Codex end of turn
- GIVEN a Codex end-of-turn event whose `cwd` is a spec branch's checkout with uncommitted spec work
- WHEN `autosync_hook` runs for `Codex`
- THEN it runs `clm autosync` there, its stdout is empty and the exit code is 0

#### Needs
- agent-harness/hook-events

#### Serves
- capability/work-from-any-agent-harness
- architecture/agent-harness-adapters
