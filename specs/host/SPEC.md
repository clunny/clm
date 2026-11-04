---
title: Host
status: active
owners: ["@clunny"]
depends_on: [config, vcs]
paths: ["crates/clm-host/**"]
successor: ""
serves: [capability/git-host, architecture/host-adapters]
---

## Contract
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), and `git` and `none`, which have no host and answer every operation with nothing to do. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open <branch> --title <title> --body-file <file>`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents and nothing else (`own-pr-body`).
- `branch delete <branch> --lease <sha>`: deletes the remote branch when its tip is still `<sha>`, plain git on every host.
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL, for `clm defect` (`defect-reports`); on `git` and `none` it prints the report and says there is nowhere to file it.

A caller never runs `gh` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## Design
Crate `clm-host`. One trait, `Host`, with the four operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host. `git` and `none` return `None` from `pr open`, print nothing for `pr body` and `issue`, and still delete branches when there is an `origin`.

`pr open` on a branch that already has an open pull request prints that pull request's URL instead of opening another. `pr body` exits 1 without a call when the file is missing or holds only whitespace, or when the current branch has no open pull request. It replaced `clm pr-body`, which went with `orchestrator.watch` (`20261027-remove-watch-node`): `pr body` is what the prompts and the skills name.

## Requirements

### Requirement: branch-delete
`clm branch delete <branch> --lease <sha>` SHALL delete `origin`'s branch when its tip is `<sha>`, on every host, and SHALL leave a branch whose tip has moved, exiting 1 naming it.

#### Scenario: deleted
- GIVEN `origin/impl/a` at `s`
- WHEN `clm branch delete impl/a --lease s` runs
- THEN `origin` has no `impl/a`

#### Scenario: moved
- GIVEN `origin/impl/a` moved to `t` since
- WHEN `clm branch delete impl/a --lease s` runs
- THEN `origin/impl/a` is still `t` and it exits 1

#### Serves
- capability/git-host
- capability/land-safely
- architecture/host-adapters

### Requirement: issue
`clm issue --title <title> --body-file <file> --repo <repo>` SHALL, on `github`, run `gh issue create --repo <repo>` with the title and body and print the URL; on `git` and `none` it SHALL print the body and say there is nowhere to file it, exiting 1.

#### Scenario: filed
- GIVEN `host = "github"` and a fake `gh` that prints a URL
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN `gh` got `issue create --repo clunny/clm` with the title and the body, and the URL is printed

#### Scenario: nowhere to file
- GIVEN `host = "git"`
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it prints the report, says there is nowhere to file it, and exits 1

#### Serves
- capability/report-defects
- capability/git-host
- architecture/host-adapters

### Requirement: git-through-layer
`clm-host` SHALL list `clm-git` among its dependencies and SHALL run no `git` or `gh` process itself: every call goes through `clm-git`'s `git`, `gh`, `run` or `run_within` (`git-plumbing-layer`). No other crate in the workspace SHALL run the `gh` CLI, whether through `clm_git::gh` or through `clm_git::run` or `clm_git::run_within` with `"gh"` as its program, except:
- `clm-git` itself;
- `clm-guard`, whose only `gh` call SHALL be the release guard's run lookup (`plugin.hooks.guard/release-guard`, `release-guard-runs-gh`): one `run` or `run_within` call with `"gh"` as its program, and no `clm_git::gh`.

The scan reads each file with its whitespace removed, so a call rustfmt splits across lines still matches. A *`run` call with `"gh"` as its program* is the text `run(` or `run_within(`, with or without a path before it, whose second top-level argument (after the first comma outside brackets and quotes) is the literal `"gh"`; other `"gh"` literals in `clm-guard`, such as its tokenizer's `t == "gh"`, aren't calls. Outside the exempt crates any `"gh"` literal counts, whatever surrounds it.

`clm-web`'s `gh` module is an HTTP client for GitHub's API that runs no process, so it isn't a call to the `gh` CLI and the rule doesn't reach it; `web` reaches GitHub through its GitHub App (`host-adapters`, `web-optional`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-host` reads `crates/clm-host/Cargo.toml` and every file under `crates/clm-host/src`
- THEN `clm-git` is a dependency and no file holds `Command::new("git")` or `Command::new("gh")`

#### Scenario: no gh elsewhere
- GIVEN the workspace
- WHEN a test reads every `src/` file of every crate but `clm-git`, `clm-host` and `clm-guard`
- THEN none calls `clm_git::gh`, by its path or by importing it, and none holds the string literal `"gh"`

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

### Requirement: shared-plumbing
`clm-host`'s `branch delete` SHALL take the remote's name from `clm-git`'s `REMOTE` for the check that the remote exists, the leased push that deletes the branch and the `ls-remote` that confirms it, rather than spelling it: no line of a file under `crates/clm-host/src`, apart from those whose first non-blank characters are `//`, SHALL hold `"origin"`, `"origin/` or `remotes/origin` (`remote-plumbing-scan`). Messages for a person that name `origin`, such as `` `<branch>` on origin is no longer at <lease>; left it ``, SHALL keep their wording, and `branch delete` SHALL keep its output and exit codes.

#### Scenario: no copy of the remote's name
- GIVEN the workspace
- WHEN a test in `clm-host` reads every file under `crates/clm-host/src`, skipping each line whose first non-blank characters are `//`
- THEN no line holds `"origin"`, `"origin/` or `remotes/origin`

#### Serves
- architecture/reuse-first

### Requirement: pr-open
`clm pr open <branch> --title <title> --body-file <file>` SHALL, on `host = "github"`, open a pull request from the branch to `main` with that title and body through `gh`, or print the URL of the open one that already exists, and print the pull request's URL; on `git` and `none` it SHALL print `none` and call nothing.

#### Scenario: github
- GIVEN `host = "github"` and a fake `gh` that records its calls and prints a URL
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN `gh` got `pr create` with `--base main`, `--head spec/x` and `--body-file body.md`, and the URL is printed

#### Scenario: already open
- GIVEN the same, with a fake `gh` whose `pr view` reports an open PR for `spec/x`
- WHEN `clm pr open spec/x ...` runs
- THEN `gh pr create` isn't called and that PR's URL is printed

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN it prints `none`, exits 0, and runs no `gh`

### Requirement: pr-body
`clm pr body <file>` SHALL set the body of the current branch's open pull request to the file's contents through the host, changing nothing else, and print `updated #<number>`. It SHALL exit 1 without a host call when the file is missing or blank, or when the branch has no open pull request; on `git` and `none` it SHALL print nothing and exit 0.

#### Scenario: body updated
- GIVEN `host = "github"`, the current branch `spec/x` with an open PR 7 according to a fake `gh`, and a file `body.md`
- WHEN `clm pr body body.md` runs
- THEN `gh` got `api -X PATCH repos/{owner}/{repo}/pulls/7` with `-F body=@body.md` and nothing else, and it printed `updated #7`

#### Scenario: blank file
- GIVEN an empty `body.md`
- WHEN `clm pr body body.md` runs
- THEN it exits 1 and `gh` wasn't called
