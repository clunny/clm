---
title: Hooks
status: active
owners: ["@clunny"]
depends_on: [config, model, sync, view, view.repo, host, vcs]
paths: ["crates/clm-hooks/**"]
successor: ""
serves: [capability/guard-sessions, architecture/approval-policy, architecture/guard-blocks-push-to-main]
---

## Contract
The rules behind `clm hook session-start`, `clm hook autosync` and `clm defect` (`hook-logic-in-clm`): what the session-start hook prints, when autosync runs, and how a defect report is filed. `plugin`'s `hooks.json` wires Claude Code's events to `clm hook <name>`; this component and its child `plugin.hooks.guard` decide what they do. `clm hook guard`, which asks the human to confirm `clm approve`, `clm capabilities decide` and a `clm defect` whose report leaves the project, and blocks pushes to `main` and release runs, is `plugin.hooks.guard`'s. Crate `clm-hooks`: `session_start(event)`, `autosync(event)` and `defect(...)`, with `settings(cwd)` and `clm_root(cwd)`, which find the CLM repository for a directory and load its settings the one way the hooks, the guard and `clm defect` share.

## Design
Crate `clm-hooks`, called from `crates/clm`'s `clm hook session-start`, `clm hook autosync` and `clm defect` subcommands. The guard is crate `clm-guard`, the child `plugin.hooks.guard`, with the shell reading as its child `plugin.hooks.guard.shell` (`guard-split`); `clm-guard` reads settings through this crate's `settings`, and this crate depends on nothing of it.

This component's tests are `crates/clm-hooks/tests/hooks.rs` and `git_layer.rs`, and, for what runs the built binary, `crates/clm/tests/hooks.rs` and `crates/clm/tests/defect.rs`.

**Defects** (`defect-reports`). `clm defect --title <title> [--component <component>]...` reads the report from stdin and loads the settings for the CLM repository found from the current directory, or none. By `defects`:
- `backlog` (`defect-backlog`): `clm check` validates the components as for any item.
- `ask` (`defect-issue`): the issue's body is the report under `## What happened`, then the components, the `clm` version and the OS.
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
When `clm hook session-start` prints the CLM state and the repository's `defects` setting isn't `off`, it SHALL add a paragraph that names `clm defect`, says what counts as a CLM defect and what doesn't, says a report describes only the process and never the project's files, code, specs or output, and says a session that can't ask the human lists defects under `## CLM defects` in its PR body. When the setting is `backlog` the paragraph SHALL also say that reports go to `backlog/` as `kind: bug` items, and that a small defect needing no spec change is fixed in the moment rather than left filed: folded into the current branch, unreported, when it is an `impl/*` branch and the fix lies in its components (`fold-small-defer-large`), and otherwise reported with `clm defect`, naming with `--component` the component the fix lies in, and, once the current branch has landed and brought the item to `main`, fixed before the session ends on its own branch with `clm claim <component> --fix backlog/<slug>` (`code-fix-resolves-bug`); that only a defect that is a large amount of work or needs a spec change is left in the backlog; and that an unattended session folds what fits and reports the rest. With `ask` it SHALL say none of that, and SHALL say to send the report as the body of a `<<'EOF'` heredoc on the `clm defect` line, the one form the guard shows the human before the report leaves the project (`defect-ask-shows-report`). When the setting is `off` it SHALL add nothing. When the settings can't be loaded it SHALL print their error instead, and still exit 0.


#### Scenario: asking
- GIVEN a CLM repository with no settings
- WHEN `clm hook session-start` runs
- THEN its output names `clm defect` and `## CLM defects` after the `claims` section, names `<<'EOF'`, and doesn't name `--fix`

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
With `defects` set to `ask`, `clm defect` SHALL file the report through `host`'s `issue` on `clunny/clm`, with the title and a body holding the report, the components, the `clm` version and the OS, and print the issue's URL. When the host can't file it, or has nowhere to, it SHALL print the report and the reason and exit 1. With `defects` set to `off`, it SHALL file nothing, say reporting is off, and exit 0.

#### Scenario: filed
- GIVEN a repository with no settings, `host = "github"`, and a fake `gh` that records its arguments and prints a URL
- WHEN `clm defect --title "Sync loses a delta"` runs with a report on stdin
- THEN `gh` got `issue create`, `--repo clunny/clm` and the title, its body holds the report and the `clm` version, and the URL is printed

#### Scenario: no access
- GIVEN a fake `gh` that fails with `Could not resolve to a Repository`
- WHEN the same `clm defect` runs
- THEN it prints the report and the error, and exits 1

#### Scenario: no host
- GIVEN a repository whose `clm.toml` sets `host = "git"`
- WHEN the same `clm defect` runs
- THEN it prints the report, says there is nowhere to file it, and exits 1

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
