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
- `branch push`: pushes the checked-out branch to `origin`, overwriting `origin`'s branch only when its tip is the one last fetched and the branch has held it, plain git on every host. It is how an unattended session pushes its rebased branch with a `clm` command alone (`unattended-steps-run-clm`).
- `branch delete <branch> [--lease <sha>]`: deletes the remote branch when its tip is still `<sha>`, by default the tip last fetched, and only a tip this clone pushed, so a session can release its own claim but never another's, plain git on every host.

Neither ever touches `main` or a branch under `clm/`, the ledgers' (`guard-blocks-push-to-main`): `main` moves only through `clm land` (`land-locally`).
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL, for `clm defect` (`defect-reports`); on `git` and `none` it prints the report and says there is nowhere to file it.

- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; listing the open issues and pull requests (`list-contributions`); and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `git` and `none` have no such thing.
- the open dependency alerts, for `clm intake` (`list-alerts`): each alert's number, severity, ecosystem, package and manifest path, so an alert reaches the backlog as an issue does (`dependency-alerts-through-intake`). `git` and `none` have none.

A caller never runs `gh` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host, `<sha>` being `--lease` or, without it, the tip of `refs/remotes/origin/<branch>`. `git` and `none` return `None` from `pr open`, print nothing for `pr body` and `issue`, and still delete branches when there is an `origin`.

**Pushing a branch** (`branch-push`). `branch push` reads the checked-out branch with `git symbolic-ref --short HEAD` and pushes `HEAD:refs/heads/<branch>` to `origin`, setting the branch's upstream to `origin/<branch>`. When `refs/remotes/origin/<branch>` exists it passes `--force-with-lease=refs/heads/<branch>` with no value, which leases on that ref's tip, and `--force-if-includes`, which further refuses when that tip isn't in the local branch's reflog; git ignores `--force-if-includes` beside an explicit lease value, so the value is left out. Without the remote-tracking ref it passes `--force-with-lease=refs/heads/<branch>:`, which expects no branch on `origin`. So it is a compare-and-swap on what the session last saw (`lease-claims`): a branch another session pushed since the last fetch is refused with git's `stale info`, and one a fetch brought in but the session never had locally is refused by `--force-if-includes`, so a session overwrites only its own work. A rebased claim branch passes, since its old tip is in its reflog. It prints git's message, the remote's lines included, and `pushed <branch> <sha>` on success. It retries nothing: a transient refusal is the caller's to retry, as the prompts say (`orchestrator.run/prompt-commands`).

**Refused branches.** Both refuse, before any git call that writes, `main` and any branch whose name starts with `clm/`, and `branch push` refuses a detached `HEAD`. These are refusals of the command, not of the guard: the guard sees only `clm branch ...`, never the `git push` it runs (`plugin.hooks.guard/push-guard`), so the refusal must be the command's own.

**Deleting only what this clone pushed** (`branch-delete`). The project's allow rules let an unattended session run `clm branch delete` with no classifier judging it (`unattended-steps-run-clm`), so the command itself must not release another session's claim or delete its spec branch, which would bypass the claim's expiry and takeover (`lease-claims`). `clm branch delete` therefore deletes only a tip this clone pushed: before the delete it reads the newest entry of `refs/remotes/origin/<branch>`'s reflog (`git reflog show -n 1 --format='%H %gs' refs/remotes/origin/<branch>`), and goes on only when that entry is git's `update by push` and its commit is the lease. Git writes that entry when a push from this clone to `origin` moves the branch, whether `clm claim`, `clm branch push`, `clm autosync` or a plain `git push` made it, and a fetch that brings another clone's tip writes a different entry, so a tip another session pushed is refused even after this clone fetched it. A fetch that finds the branch unchanged writes no entry, so a session's own branch stays deletable after `clm rebase` fetches. No reflog, as when `core.logAllRefUpdates` is off, refuses: it fails closed. Before that check it asks `origin` with `git ls-remote origin refs/heads/<branch>`, and a branch already gone, or a clone with no `origin`, is nothing to do and exits 0, as before, so a draft deleting an `autosync/spec/<id>` that was never pushed doesn't fail. The check is the subcommand's, not the `Host` trait's: `clm land`'s delete of the branch it just landed calls the library operation, which keeps its plain lease.

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
`clm-host` SHALL offer `list`, which on `github` returns the repository's open issues and open pull requests, each with its number, whether it is an issue or a pull request, and its author's login, through `gh issue list` and `gh pr list` with `--state open` and `--json`, each with `--limit 1000` rather than `gh`'s default of 30; an author whose account is gone is `ghost`, as `gh` reports it. It SHALL return no title, body or other text the author wrote, so nothing a caller prints from it carries a stranger's words. On `git` and `none` it SHALL return an empty list and run no `gh`.

#### Scenario: issues and pull requests
- GIVEN `host = "github"` and a fake `gh` reporting open issue 73 by `clunny` and open pull request 80 by `dana`
- WHEN `list` runs
- THEN it returns issue 73 by `clunny` and pull request 80 by `dana`, and `gh` got `--state open` for both

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
