---
component: plugin.hooks
base: { design: "2b0f4d94bfb0ee39" }
---

## MODIFIED Design
Crate `clm-hooks`, called from `crates/clm`'s `clm hook session-start`, `clm hook autosync` and `clm defect` subcommands. The guard is crate `clm-guard`, the child `plugin.hooks.guard`, with the shell reading as its child `plugin.hooks.guard.shell` (`guard-split`); `clm-guard` reads settings through this crate's `settings`, and this crate depends on nothing of it.

This component's tests are `crates/clm-hooks/tests/hooks.rs` and `git_layer.rs`, and, for what runs the built binary, `crates/clm/tests/hooks.rs` and `crates/clm/tests/defect.rs`.

**Defects** (`defect-reports`). `clm defect --title <title> [--component <component>]...` reads the report from stdin and loads the settings for the CLM repository found from the current directory, or none. By `defects`:
- `backlog` (`defect-backlog`): `clm check` validates the components as for any item.
- `ask` (`defect-issue`, `defects-file-on-github`): the issue's body is the report under `## What happened`, then the components, the `clm` version and the OS. It goes through `clm_host::adapter(Host::Github)`, never the adapter of the `host` setting. A report it can't file is kept in `clm/defects/` under the directory `clm_git::common_dir` gives for the current directory, named by the same slug function and `create_new` loop as a backlog report, so two reports with one title at once can't overwrite each other.
- `off`: reporting is off.

## MODIFIED Requirements

### Requirement: defect-rule <!-- base: c40c68631daac352 -->
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

### Requirement: defect-issue <!-- base: 73f880b90b150397 -->
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
