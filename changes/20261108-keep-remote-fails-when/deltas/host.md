---
component: host
---

## MODIFIED Requirements

### Requirement: git-through-layer <!-- base: 0d871c32d9d0821a -->
`clm-host` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `git_isolated`, `gh`, `run` or `run_within` (`git-plumbing-layer`). No other crate in the workspace SHALL run the `gh` CLI, whether through `clm_git::gh` or through `clm_git::run` or `clm_git::run_within` with `"gh"` as its program, except:
- `clm-git` itself;
- `clm-guard`, whose only `gh` call SHALL be the release guard's run lookup (`plugin.hooks.guard/release-guard`, `release-guard-runs-gh`): one `run` or `run_within` call with `"gh"` as its program, and no `clm_git::gh`.

The scan reads each file with its whitespace removed, so a call rustfmt splits across lines still matches. A *`run` call with `"gh"` as its program* is the text `run(` or `run_within(`, with or without a path before it, whose second top-level argument (after the first comma outside brackets and quotes) is the literal `"gh"`; other `"gh"` literals in `clm-guard`, such as its tokenizer's `t == "gh"`, aren't calls. Outside the exempt crates any `"gh"` literal counts, whatever surrounds it.

No crate but `clm-git` and `clm-host` SHALL run the Azure CLI: outside those two, no `src/` file SHALL hold the string literal `"az"`, whatever surrounds it, `clm-guard` included (`azure-through-az-cli`).

`clm-web`'s `gh` module is an HTTP client for GitHub's API that runs no process, so it isn't a call to the `gh` CLI and the rule doesn't reach it; `web` reaches GitHub through its GitHub App (`host-adapters`, `web-optional`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-host` reads `crates/clm-host/Cargo.toml` and every file under `crates/clm-host/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")`, `Command::new("gh")` or `Command::new("az")`

#### Scenario: no gh elsewhere
- GIVEN the workspace
- WHEN a test reads every `src/` file of every crate but `clm-git`, `clm-host` and `clm-guard`
- THEN none calls `clm_git::gh`, by its path or by importing it, and none holds the string literal `"gh"`

#### Scenario: no az elsewhere
- GIVEN the workspace
- WHEN a test reads every `src/` file of every crate but `clm-git` and `clm-host`
- THEN none holds the string literal `"az"`

#### Scenario: the release guard's lookup only
- GIVEN the workspace
- WHEN a test reads every `src/` file of `clm-guard`
- THEN none calls `clm_git::gh`, and exactly one `run` or `run_within` call, across all of them, has the literal `"gh"` as its program

#### Scenario: a run of gh is caught
- GIVEN the source texts `clm_git::run(&dir, "gh", &args, options)` and `clm_git::run_within(&dir, "gh", &args, options, limit)`, and each split across lines as rustfmt splits it
- WHEN the matcher for other crates and the `clm-guard` call counter each read them
- THEN both report a `gh` call in each, and the counter reports none in `t == "gh"` or in `prerun_within(&dir, "gh")`

#### Serves
- architecture/host-adapters

#### Needs
- clm/no-watch-crate
- plugin.hooks.guard/git-through-layer

### Requirement: none-keeps-a-remote <!-- base: 85ff1b8b1599f42a -->
`clm-host` SHALL offer `keep_remote(root, host)`. When `host` is `none` and the clone has no remote named `clm-git`'s `REMOTE`, it SHALL make `clm/remote.git` under the clone's common git directory (`clm-git`'s `common_dir`) a bare repository, keeping one already there, add it as `REMOTE` by its absolute path, and return that path; it SHALL push nothing and write nothing in any worktree. On any other host, or when the clone has `REMOTE`, it SHALL change nothing and return nothing. Run from a linked worktree, it SHALL give every worktree of the clone the same `origin`. It SHALL make the bare repository through `clm-git`'s `git_isolated`, so a clone the environment names with `GIT_DIR` and `GIT_WORK_TREE` (a work tree whose git directory is elsewhere) gets its remote too, while reading the remotes, `common_dir` and adding `REMOTE` still run on the clone those variables name.

#### Needs
- vcs/git-isolated

#### Scenario: no remote
- GIVEN `host = "none"`, a clone with a commit on `main` and no remote
- WHEN `keep_remote` runs
- THEN `<common git dir>/clm/remote.git` is a bare repository, `git remote get-url origin` prints its absolute path, it has no branch, and the working tree is unchanged

#### Scenario: from a linked worktree
- GIVEN the same clone with no remote, and a linked worktree of it
- WHEN `keep_remote` runs in the linked worktree
- THEN the bare repository is under the common git directory, not the worktree's, and `git remote get-url origin` prints the same path in both checkouts

#### Scenario: a clone the environment names
- GIVEN `host = "none"`, a work tree `w` with a commit on `main` and no remote, whose git directory `g` is elsewhere, and `GIT_DIR=g` and `GIT_WORK_TREE=w` set in the environment `keep_remote` runs in
- WHEN `keep_remote(w, none)` runs
- THEN it succeeds, `g/clm/remote.git` is a bare repository, and `git remote get-url origin`, run with those variables, prints its absolute path

#### Scenario: already kept
- GIVEN the clone after *no remote*, its `origin` then removed with `git remote remove origin`
- WHEN `keep_remote` runs
- THEN `origin` names the same bare repository again, and what it held is still there

#### Scenario: nothing to do
- GIVEN `host = "none"` with an `origin` naming another bare repository, and in turn `host = "git"` with no remote
- WHEN `keep_remote` runs in each
- THEN the remotes are unchanged, no `clm/remote.git` exists, and it returns nothing
