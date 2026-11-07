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
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), `azure` (the Azure CLI, `az`, with its `azure-devops` extension), and `git` and `none`, which have no host and answer every operation with nothing to do. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open <branch> --title <title> --body-file <file>`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents and nothing else (`own-pr-body`).
- `branch push`: pushes the checked-out branch to `origin`, overwriting `origin`'s branch only when its tip is the one last fetched and the branch has held it, plain git on every host. It is how an unattended session pushes its rebased branch with a `clm` command alone (`unattended-steps-run-clm`).
- `branch delete <branch> [--lease <sha>]`: deletes the remote branch when its tip is still `<sha>`, by default the tip last fetched, and only a tip this clone pushed, so a session can release its own claim but never another's, plain git on every host.

Neither ever touches `main` or a branch under `clm/`, the ledgers' (`guard-blocks-push-to-main`): `main` moves only through `clm land` (`land-locally`).
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL, for `clm defect` (`defect-reports`); on `azure`, `git` and `none` it prints the report and says there is nowhere to file it, since `clm defect`'s reports go to CLM's own repository on GitHub, not the project's host.

- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; listing the open issues and the open pull requests from forks (`list-contributions`), not those of branches pushed to the repository itself, which `pr open` opens for reading a diff; and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `azure`, `git` and `none` have no such thing.
- the host a remote's URL names, for `clm init` (`host-for-remote`), and the steps a person takes to set a repository on that host up for CLM, which `clm init` prints (`host-setup-steps`), so `clm init` holds no host's names or permissions itself.
- the open dependency alerts, for `clm intake` (`list-alerts`): each alert's number, severity, ecosystem, package and manifest path, so an alert reaches the backlog as an issue does (`dependency-alerts-through-intake`). `azure`, `git` and `none` have none.
- the runs of a named pipeline on `main`, for `clm verify` (`list-runs`): each run's ID, head commit, whether it has completed, its conclusion and its URL, so a project whose slow tests run in its host's pipeline has their results recorded (`verify-after-land`). Only `github` lists them; the others answer that the host lists no pipeline runs.

A caller never runs `gh` or `az` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host, `<sha>` being `--lease` or, without it, the tip of `refs/remotes/origin/<branch>`.`git` and `none` return `None` from `pr open`, print nothing for `pr body` and `issue`, and still delete branches when there is an `origin`.

**Azure DevOps** (`azure-pull-requests`, `azure-through-az-cli`). `azure` runs the Azure CLI through `clm-git`'s `run`, with `--detect true` so `az` takes the organisation, project and repository from the clone's remote, and `--output json`; `az` holds the sign-in (`az login`, or `AZURE_DEVOPS_EXT_PAT`), so CLM reads no token. `pr open` first runs `az repos pr list --source-branch <branch> --target-branch main --status active` and, when it lists a pull request, prints the first one's URL; otherwise it runs `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, the file's path made absolute against the caller's directory, since `az` reads `@<file>` relative to the directory it runs in, the repository's root. Azure's `--source-branch` takes the bare branch name. The URL printed is the pull request's web page, `<repository.webUrl>/pullrequest/<pullRequestId>`, from the JSON `az` returns, since the `url` field there is the REST API's. `pr body` finds the current branch's active pull request the same way and runs `az repos pr update --id <pullRequestId> --description @<file>`. Every other operation is the trait's default, as for `git`: `issue` prints the report and says there is nowhere to file it, `parent` returns nothing, `list` and `alerts` return nothing, and the rest report that there is no such thing on the host.

**Pushing a branch** (`branch-push`). `branch push` reads the checked-out branch with `git symbolic-ref --short HEAD` and pushes `HEAD:refs/heads/<branch>` to `origin`, setting the branch's upstream to `origin/<branch>`. When `refs/remotes/origin/<branch>` exists it passes `--force-with-lease=refs/heads/<branch>` with no value, which leases on that ref's tip, and `--force-if-includes`, which further refuses when that tip isn't in the local branch's reflog; git ignores `--force-if-includes` beside an explicit lease value, so the value is left out. Without the remote-tracking ref it passes `--force-with-lease=refs/heads/<branch>:`, which expects no branch on `origin`. So it is a compare-and-swap on what the session last saw (`lease-claims`): a branch another session pushed since the last fetch is refused with git's `stale info`, and one a fetch brought in but the session never had locally is refused by `--force-if-includes`, so a session overwrites only its own work. A rebased claim branch passes, since its old tip is in its reflog. It prints git's message, the remote's lines included, and `pushed <branch> <sha>` on success. It retries nothing: a transient refusal is the caller's to retry, as the prompts say (`orchestrator.run/prompt-commands`).

**Refused branches.** Both refuse, before any git call that writes, `main` and any branch whose name starts with `clm/`, and `branch push` refuses a detached `HEAD`. These are refusals of the command, not of the guard: the guard sees only `clm branch ...`, never the `git push` it runs (`plugin.hooks.guard/push-guard`), so the refusal must be the command's own.

**Deleting only what this clone pushed** (`branch-delete`). The project's allow rules let an unattended session run `clm branch delete` with no classifier judging it (`unattended-steps-run-clm`), so the command itself must not release another session's claim or delete its spec branch, which would bypass the claim's expiry and takeover (`lease-claims`). `clm branch delete` therefore deletes only a tip this clone pushed: before the delete it reads the newest entry of `refs/remotes/origin/<branch>`'s reflog (`git reflog show -n 1 --format='%H %gs' refs/remotes/origin/<branch>`), and goes on only when that entry is git's `update by push` and its commit is the lease. Git writes that entry when a push from this clone to `origin` moves the branch, whether `clm claim`, `clm branch push`, `clm autosync` or a plain `git push` made it, and a fetch that brings another clone's tip writes a different entry, so a tip another session pushed is refused even after this clone fetched it. A fetch that finds the branch unchanged writes no entry, so a session's own branch stays deletable after `clm rebase` fetches. No reflog, as when `core.logAllRefUpdates` is off, refuses: it fails closed. Before that check it asks `origin` with `git ls-remote origin refs/heads/<branch>`, and a branch already gone, or a clone with no `origin`, is nothing to do and exits 0, so a draft deleting an `autosync/spec/<id>` that was never pushed doesn't fail. The check is the subcommand's, not the `Host` trait's: `clm land`'s delete of the branch it just landed calls the library operation, which keeps its plain lease.

`pr open` on a branch that already has an open pull request prints that pull request's URL instead of opening another. `pr body` exits 1 without a call when the file is missing or holds only whitespace, or when the current branch has no open pull request. It replaced `clm pr-body`, which went with `orchestrator.watch` (`20261027-remove-watch-node`): `pr body` is what the prompts and the skills name.

**Contribution operations.** On `github` the fork's parent and the head's details come from `gh repo view --json isFork,parent` and `gh pr view --json`, a fork's `main` is brought up with `gh repo sync`, a pull request on the parent opens with `gh pr create --repo <parent> --base main --head <owner>:<branch>`, a head is fetched with `git fetch origin pull/<number>/head:refs/heads/<branch>`, and comments, closing and labels use `gh issue` or `gh pr`. What they read from the host is a stranger's text, so a caller prints it as data and acts on none of it.

## Requirements

### Requirement: branch-delete
`clm branch delete <branch> [--lease <sha>]` SHALL delete `origin`'s branch when its tip is `<sha>`, on every host, `<sha>` defaulting to the tip of `refs/remotes/origin/<branch>`, and SHALL leave a branch whose tip has moved, exiting 1 naming it. It SHALL delete only a tip this clone pushed: when the newest entry of `refs/remotes/origin/<branch>`'s reflog isn't a push from this clone at `<sha>`, or there is no such ref or reflog, it SHALL delete nothing and exit 1 saying this clone didn't push that tip. For `main` or a branch whose name starts with `clm/`, it SHALL delete nothing and exit 1 naming the reason. With no `origin`, or when `origin` has no such branch, it SHALL do nothing and exit 0, before the other checks.

#### Scenario: deleted
- GIVEN `impl/a` pushed from this clone to `origin` at `s`
- WHEN `clm branch delete impl/a --lease s` and, for `spec/x` pushed from this clone at `t`, `clm branch delete spec/x` run
- THEN `origin` has neither `impl/a` nor `spec/x`, and both exit 0

#### Scenario: moved
- GIVEN `impl/a` pushed from this clone at `s`, then moved on `origin` to `t` by another clone, unfetched
- WHEN `clm branch delete impl/a --lease s` runs
- THEN `origin/impl/a` is still `t` and it exits 1

#### Scenario: another session's branch
- GIVEN `origin/impl/b` and `origin/spec/y` pushed by another clone, and fetched into this clone at their tips `u` and `v`
- WHEN `clm branch delete impl/b --lease u` and `clm branch delete spec/y` run
- THEN `origin/impl/b` is still `u`, `origin/spec/y` is still `v`, and each exits 1 saying this clone didn't push it

#### Scenario: never fetched
- GIVEN `spec/z` on `origin`, pushed by another clone, and no `refs/remotes/origin/spec/z` in this clone
- WHEN `clm branch delete spec/z` runs
- THEN `origin` still has `spec/z`, and it exits 1 naming `origin/spec/z`

#### Scenario: already gone
- GIVEN no `impl/gone` on `origin`, and in turn a clone with no `origin`
- WHEN `clm branch delete impl/gone --lease s` runs
- THEN it exits 0

#### Scenario: main refused
- GIVEN `origin/main` at `m`
- WHEN `clm branch delete main --lease m` runs
- THEN `origin/main` is still `m` and it exits 1

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

### Requirement: contribution-operations
`clm-host` SHALL offer, on `github`, through `gh` and `clm-git`'s layer: `parent`, the fork's parent as `<owner>/<name>`, or nothing when `origin` isn't a fork; `sync`, bringing `origin`'s `main` up to its parent's without force; `open_on_parent(branch, title, body file)`, opening a pull request from `<owner>:<branch>` to the parent's `main`, or returning the URL of the open one; `show(number)`, returning a pull request's author, its author's numeric account ID (from `gh api users/<login>`, which a `Co-authored-by` trailer's `<id>+<login>@users.noreply.github.com` address needs), state, title, body, head repository URL, head branch, whether maintainers may edit and changed files, or an issue's author, its author's numeric account ID, state, title and body; `fetch(number, branch)`, making a local branch at a pull request's head without checking it out, refusing a branch that exists; `comment(number, body file, close)`, adding a comment and closing when `close` is set; and `label(number, label)`. On `git` and `none`, `parent` SHALL return nothing and each other operation SHALL report that there is no host, running no `gh`.

#### Scenario: a fork's parent
- GIVEN `host = "github"` and a fake `gh` whose `repo view` reports a fork of `clunny/clm`
- WHEN `parent` runs
- THEN it returns `clunny/clm`

#### Scenario: opening on the parent
- GIVEN that fake `gh`, with `origin` owned by `dana`
- WHEN `open_on_parent` runs for `spec/x`
- THEN `gh` got `pr create` with `--repo clunny/clm`, `--base main` and `--head dana:spec/x`

#### Scenario: a pull request's files
- GIVEN a fake `gh` reporting pull request 7 with changed files `backlog/x.md` and `crates/a/src/lib.rs`
- WHEN `show` runs for 7
- THEN it returns both files, the head repository's URL, whether maintainers may edit and the author's numeric account ID

#### Scenario: closing with a comment
- GIVEN a fake `gh`
- WHEN `comment` runs for 7 with `close` set
- THEN `gh` got the comment's body and closed 7

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `show` runs for 7
- THEN it reports that there is no host, and no `gh` ran

#### Serves
- capability/contribute-from-outside

### Requirement: list-contributions
`clm-host` SHALL offer `list`, which on `github` returns the repository's open issues and the open pull requests whose head is in another repository, each with its number, whether it is an issue or a pull request, and its author's login, through `gh issue list` and `gh pr list` with `--state open` and `--json`, each with `--limit 1000` rather than `gh`'s default of 30, `gh pr list`'s `--json` also asking for `isCrossRepository`, and SHALL return a pull request only when `gh` reports `isCrossRepository` true, leaving out one whose head is a branch of the repository itself and one reported without the field, since every branch CLM pushes opens one for reading its diff (`pr open`) and none of those is a contribution (`outside-contributions-are-intent`); an author whose account is gone is `ghost`, as `gh` reports it. It SHALL return no title, body or other text the author wrote, so nothing a caller prints from it carries a stranger's words. On `git` and `none` it SHALL return an empty list and run no `gh`.

#### Scenario: issues and pull requests
- GIVEN `host = "github"` and a fake `gh` reporting open issue 73 by `clunny` and open pull request 80 by `dana` from a fork
- WHEN `list` runs
- THEN it returns issue 73 by `clunny` and pull request 80 by `dana`, and `gh` got `--state open` for both

#### Scenario: the repository's own branch
- GIVEN `host = "github"` and a fake `gh` reporting open pull request 80 by `dana` from a fork and open pull request 81 by `clunny` whose head is `impl/a` in the repository itself
- WHEN `list` runs
- THEN it returns pull request 80 by `dana` only

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `list` runs
- THEN it returns nothing, and no `gh` ran

#### Serves
- capability/contribute-from-outside

### Requirement: branch-push
`clm branch push` SHALL push the checked-out branch to `origin`'s branch of the same name and set its upstream there, on every host: when the clone has `refs/remotes/origin/<branch>`, only while `origin`'s branch is still at that ref's tip and the local branch has held that tip, as git's `--force-if-includes` judges it; when it hasn't, only while `origin` has no such branch. It SHALL work on the branch checked out in the directory it runs in, and SHALL print `pushed <branch> <sha>`, `<sha>` the full commit `HEAD` names, and exit 0 when the push goes through or `origin` already holds that commit. With no `origin` remote it SHALL push nothing and exit 1. When git refuses the push it SHALL print git's message, the remote's lines included, and exit 1, leaving `origin`'s branch as it was. On a detached `HEAD`, on `main` or on a branch whose name starts with `clm/`, it SHALL push nothing and exit 1 naming the reason.

#### Scenario: rebased branch pushed
- GIVEN branch `impl/a` checked out, pushed to `origin` at `s` and fetched, then rewritten by a rebase to `t`
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is `t`, the branch's upstream is `origin/impl/a`, it prints `pushed impl/a t` and exits 0

#### Scenario: new branch
- GIVEN branch `spec/x` checked out at `s`, with no `origin/spec/x` and no remote-tracking ref for it
- WHEN `clm branch push` runs
- THEN `origin/spec/x` is `s`, the branch's upstream is `origin/spec/x`, and it exits 0

#### Scenario: moved since the last fetch
- GIVEN branch `impl/a` checked out and rebased, and `origin/impl/a` pushed to `u` by another clone since this clone's last fetch
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is still `u`, the output holds `stale info`, and it exits 1

#### Scenario: fetched but never held
- GIVEN branch `impl/a` checked out at `t`, and `origin/impl/a` pushed to `u` by another clone and fetched into `refs/remotes/origin/impl/a`, `u` never in the local branch's reflog
- WHEN `clm branch push` runs
- THEN `origin/impl/a` is still `u` and it exits 1

#### Scenario: main refused
- GIVEN `main` checked out with a commit `origin/main` lacks, and in turn a detached `HEAD`, and a branch `clm/ledger/p/c` checked out
- WHEN `clm branch push` runs in each
- THEN `origin` is unchanged and it exits 1 each time

#### Serves
- capability/git-host
- capability/build-from-spec
- capability/triage-unattended
- architecture/lease-claims
- architecture/guard-blocks-push-to-main

### Requirement: list-alerts
`clm-host` SHALL offer `alerts`, which on `github` returns the repository's open Dependabot alerts, each with its number, severity (`low`, `medium`, `high` or `critical`), ecosystem, package name and manifest path, through `gh api --paginate` on `repos/{owner}/{repo}/dependabot/alerts` with `state=open`, so every page is read, taking the number from `number`, the severity from `security_advisory.severity`, and the ecosystem, package and manifest path from `dependency.package.ecosystem`, `dependency.package.name` and `dependency.manifest_path`. A field holding whitespace or a control character SHALL come back with each such character replaced by `?`, a severity outside those four as `unknown`, and a missing or empty field as `unknown`, so a caller that prints a field prints one word. It SHALL return no advisory summary, description or other text of the advisory. When `gh` fails, as it does when alerts are off or the login may not read them, it SHALL return an error carrying `gh`'s message. On `git` and `none` it SHALL return an empty list and run no `gh`.

#### Scenario: open alerts
- GIVEN `host = "github"` and a fake `gh` reporting open alert 4, `high`, ecosystem `npm`, package `vite`, manifest `web/package-lock.json`
- WHEN `alerts` runs
- THEN it returns alert 4 with those fields, and `gh` got `api`, `--paginate` and `state=open`

#### Scenario: a field with a space
- GIVEN a fake `gh` reporting open alert 5 whose package is `a b`
- WHEN `alerts` runs
- THEN it returns alert 5 with package `a?b`

#### Scenario: alerts unreadable
- GIVEN a fake `gh` that exits 1 on `api`, printing `HTTP 403`
- WHEN `alerts` runs
- THEN it returns an error whose message holds `HTTP 403`

#### Scenario: no host
- GIVEN `host = "none"`
- WHEN `alerts` runs
- THEN it returns nothing, and no `gh` ran

#### Serves
- capability/keep-a-backlog
- capability/git-host

### Requirement: azure-pull-requests
`clm-host` SHALL offer the `Azure` adapter, a public implementation of `Host` exported as `clm_host::Azure` that a caller can build and call directly, so it is built and tested before the `host` setting can name it (`forced-fixes-cover-additions`). Its `pr open` for a branch, a title and a body file SHALL give the web URL (`<repository web URL>/pullrequest/<id>`) of the branch's active pull request when `az repos pr list --source-branch <branch> --target-branch main --status active` lists one, and otherwise SHALL open one from the branch to `main` with that title and the file's contents as its description through `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, and give its web URL. Its `pr body` for a file SHALL set the description of the current branch's active pull request to the file's contents through `az repos pr update --id <id> --description @<file>`, changing nothing else, and report `updated #<id>`; it SHALL fail without a call to `az` when the file is missing or blank, and with no `az repos pr update` when the branch has no active pull request. Every `az` call SHALL pass `--detect true` and `--output json` and go through `clm-git`'s `run`. When `az` fails, as it does when it isn't installed, lacks the `azure-devops` extension or isn't signed in, the operation SHALL fail with `az`'s message.

#### Scenario: opened
- GIVEN a fake `az` that records its calls, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x` with title `x` and body file `body.md`
- THEN `az` got `repos pr create` with `--source-branch spec/x`, `--target-branch main`, `--title x` and `--description @body.md`, and it gives `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: already open
- GIVEN the same, with a fake `az` whose `pr list` reports active pull request 12 for `spec/x`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`
- THEN `az repos pr create` isn't called and it gives that pull request's web URL

#### Scenario: body updated
- GIVEN the current branch `spec/x` with active pull request 12 according to a fake `az`, and a file `body.md`
- WHEN the `Azure` adapter's `pr body` runs for `body.md`
- THEN `az` got `repos pr update` with `--id 12` and `--description @body.md` and nothing else that changes the pull request, and it reports `updated #12`

#### Scenario: az not signed in
- GIVEN a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN the `Azure` adapter's `pr open` runs for `spec/x`
- THEN it fails with a message holding `az login`

#### Serves
- capability/git-host
- architecture/host-adapters

### Requirement: azure-other-operations
The `Azure` adapter's operations other than `pr open`, `pr body`, `branch push` and `branch delete` SHALL behave as on `git`, running no `az` and no `gh`: `issue` SHALL give the report and say there is nowhere to file it, as a failure; `parent` SHALL return nothing; `list` and `alerts` SHALL return an empty list; and `sync`, `open_on_parent`, `show`, `fetch`, `comment` and `label` SHALL report that there is no host.

#### Scenario: issue
- GIVEN fake `az` and `gh` that record their calls
- WHEN the `Azure` adapter's `issue` runs with title `x` and body file `r.md`
- THEN it fails giving the report and saying there is nowhere to file it, and neither fake was called

#### Scenario: contributions and alerts
- GIVEN the same
- WHEN the `Azure` adapter's `parent`, `list`, `alerts` and `show` for 7 run
- THEN `parent` returns nothing, `list` and `alerts` return nothing, `show` reports that there is no host, and neither fake was called

#### Serves
- capability/git-host
- architecture/host-adapters
- architecture/yagni

### Requirement: host-for-remote
`clm-host` SHALL offer `host_for_remote(url)`, giving the `host` setting a remote's URL names: `github` when the URL's host is `github.com` or `ssh.github.com`, `azure` when it is `dev.azure.com` or `ssh.dev.azure.com` or ends in `.visualstudio.com`, and `git` for any other URL. It SHALL parse the host out of the URL's forms `https://[user@]host[:port]/...`, `http://...`, `ssh://[user@]host[:port]/...` and `user@host:path`, and compare it whole and without regard to case, so a path or a longer host that merely contains one of those names gives `git`. It SHALL run no process.

#### Scenario: hosts named
- GIVEN the URLs `https://github.com/o/r.git`, `git@github.com:o/r.git`, `https://o@dev.azure.com/o/p/_git/r`, `git@ssh.dev.azure.com:v3/o/p/r`, `https://o.visualstudio.com/p/_git/r` and `ssh://git@example.com/r.git`
- WHEN `host_for_remote` reads each
- THEN it gives `github`, `github`, `azure`, `azure`, `azure` and `git`

#### Scenario: a name in the path
- GIVEN the URLs `https://example.com/dev.azure.com/r.git`, `https://o.visualstudio.com.example.org/r` and `https://example.com/github.com/r.git`
- WHEN `host_for_remote` reads each
- THEN it gives `git` for each

#### Serves
- capability/git-host
- capability/adopt-clm
- architecture/host-adapters

#### Needs
- config/host-setting

### Requirement: host-setup-steps
`clm-host` SHALL offer `setup_steps(host)`, the lines a person follows to set a repository on that host up for CLM, which `clm init` prints among its next steps: on `azure`, installing the Azure CLI with its `azure-devops` extension and signing in with `az login`, then, if `main` has branch policies, granting the identity that runs `clm land` Azure DevOps' `Bypass policies when pushing` permission on `main`; on `github`, if `main` is protected, adding the identity that runs `clm land` to the bypass list of the rule or ruleset that protects it; on `git`, if the server protects `main`, letting the identity that runs `clm land` push it; on `none`, nothing. Each line about a protected `main` SHALL name `clm land`. It SHALL run no process.

#### Scenario: azure
- GIVEN `host = "azure"`
- WHEN `setup_steps` runs
- THEN its lines name `azure-devops`, `az login`, `Bypass policies when pushing` and `clm land`

#### Scenario: github and none
- GIVEN `host = "github"`, and in turn `host = "none"`
- WHEN `setup_steps` runs
- THEN the first's lines name `clm land` and none of `az login`, `azure-devops` or `Bypass policies when pushing`, and the second gives no line

#### Serves
- capability/git-host
- capability/adopt-clm
- architecture/host-adapters

#### Needs
- config/host-setting

### Requirement: azure-selected
With `host = "azure"`, `clm-host`'s `adapter` SHALL give the `Azure` adapter (`azure-pull-requests`, `azure-other-operations`), so `clm pr open` and `clm pr body` run `az` as those requirements say, `clm issue` and the contribution operations behave as on `git`, and a failure of `az` makes the command exit 1 printing `az`'s message. The arm that selects it is written by `config/host-setting`'s build, which adds the `azure` variant, as a forced fix (`forced-fixes-cover-additions`); this requirement's build proves it through the commands. `capability/git-host`'s Azure DevOps scenario, which drives `clm pr open` with a stubbed `az`, sees the adapter once this requirement holds.

#### Scenario: pull request through the setting
- GIVEN a repository `clm.toml` with `host = "azure"` and a fake `az` that records its calls, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN `az` got `repos pr create`, and it prints `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: az not signed in
- GIVEN `host = "azure"` and a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN it exits 1 and its output holds `az login`

#### Scenario: issue through the setting
- GIVEN `host = "azure"` and fake `az` and `gh` that record their calls
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it prints the report, says there is nowhere to file it, exits 1, and neither fake was called

#### Serves
- capability/git-host
- architecture/host-adapters

#### Needs
- config/host-setting

### Requirement: list-runs
`clm-host` SHALL offer `runs(workflow)`, which on `github` returns the runs of that workflow on branch `main`, newest first, each with its ID, head commit, whether it has completed, its conclusion and its URL, through `gh run list --workflow <workflow> --branch main --limit 100 --json databaseId,headSha,status,conclusion,url`, taking completed from `status` being `completed` and the conclusion from `conclusion`. A head commit that isn't 40 hex digits SHALL drop the run, and a conclusion holding anything but lower-case letters and `_` SHALL come back as `unknown`, so a caller prints one word. It SHALL return no run's title, log or other text. When `gh` fails it SHALL return an error carrying `gh`'s message. On `azure`, `git` and `none` it SHALL return an error saying the host lists no pipeline runs, and run no `gh` or `az`.

#### Scenario: runs listed
- GIVEN `host = "github"` and a fake `gh` reporting a completed run 11 of `slow` on commit `c` with conclusion `failure`, and a run 12 still `in_progress`
- WHEN `runs("slow")` runs
- THEN it returns both, run 11 completed with conclusion `failure` and its URL, run 12 not completed, and `gh` got `run list`, `--workflow slow` and `--branch main`

#### Scenario: odd conclusion
- GIVEN a fake `gh` reporting a completed run whose conclusion is `Fail ure`
- WHEN `runs("slow")` runs
- THEN the run comes back with conclusion `unknown`

#### Scenario: no runs on other hosts
- GIVEN `host = "azure"`
- WHEN `runs("slow")` runs
- THEN it returns an error saying the host lists no pipeline runs, and neither `gh` nor `az` was called

#### Serves
- capability/slow-tests-after-land
