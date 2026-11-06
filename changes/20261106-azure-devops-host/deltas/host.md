---
component: host
base: { contract: "aaa3538d494ec6cf", design: "4304b1abf7468c83" }
---

## MODIFIED Contract
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), `azure` (the Azure CLI, `az`, with its `azure-devops` extension), and `git` and `none`, which have no host and answer every operation with nothing to do. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open <branch> --title <title> --body-file <file>`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents and nothing else (`own-pr-body`).
- `branch push`: pushes the checked-out branch to `origin`, overwriting `origin`'s branch only when its tip is the one last fetched and the branch has held it, plain git on every host. It is how an unattended session pushes its rebased branch with a `clm` command alone (`unattended-steps-run-clm`).
- `branch delete <branch> [--lease <sha>]`: deletes the remote branch when its tip is still `<sha>`, by default the tip last fetched, and only a tip this clone pushed, so a session can release its own claim but never another's, plain git on every host.

Neither ever touches `main` or a branch under `clm/`, the ledgers' (`guard-blocks-push-to-main`): `main` moves only through `clm land` (`land-locally`).
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL, for `clm defect` (`defect-reports`); on `azure`, `git` and `none` it prints the report and says there is nowhere to file it, since `clm defect`'s reports go to CLM's own repository on GitHub, not the project's host.

- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; listing the open issues and pull requests (`list-contributions`); and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `azure`, `git` and `none` have no such thing.
- the open dependency alerts, for `clm intake` (`list-alerts`): each alert's number, severity, ecosystem, package and manifest path, so an alert reaches the backlog as an issue does (`dependency-alerts-through-intake`). `azure`, `git` and `none` have none.

A caller never runs `gh` or `az` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## MODIFIED Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host, `<sha>` being `--lease` or, without it, the tip of `refs/remotes/origin/<branch>`.`git` and `none` return `None` from `pr open`, print nothing for `pr body` and `issue`, and still delete branches when there is an `origin`.

**Azure DevOps** (`azure-pull-requests`, `azure-through-az-cli`). `azure` runs the Azure CLI through `clm-git`'s `run`, with `--detect true` so `az` takes the organisation, project and repository from the clone's remote, and `--output json`; `az` holds the sign-in (`az login`, or `AZURE_DEVOPS_EXT_PAT`), so CLM reads no token. `pr open` first runs `az repos pr list --source-branch <branch> --status active` and, when it lists a pull request, prints that one's URL; otherwise it runs `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`. Azure's `--source-branch` takes the bare branch name. The URL printed is the pull request's web page, `<repository.webUrl>/pullrequest/<pullRequestId>`, from the JSON `az` returns, since the `url` field there is the REST API's. `pr body` finds the current branch's active pull request the same way and runs `az repos pr update --id <pullRequestId> --description @<file>`. Every other operation is the trait's default, as for `git`: `issue` prints the report and says there is nowhere to file it, `parent` returns nothing, `list` and `alerts` return nothing, and the rest report that there is no such thing on the host.

**Pushing a branch** (`branch-push`). `branch push` reads the checked-out branch with `git symbolic-ref --short HEAD` and pushes `HEAD:refs/heads/<branch>` to `origin`, setting the branch's upstream to `origin/<branch>`. When `refs/remotes/origin/<branch>` exists it passes `--force-with-lease=refs/heads/<branch>` with no value, which leases on that ref's tip, and `--force-if-includes`, which further refuses when that tip isn't in the local branch's reflog; git ignores `--force-if-includes` beside an explicit lease value, so the value is left out. Without the remote-tracking ref it passes `--force-with-lease=refs/heads/<branch>:`, which expects no branch on `origin`. So it is a compare-and-swap on what the session last saw (`lease-claims`): a branch another session pushed since the last fetch is refused with git's `stale info`, and one a fetch brought in but the session never had locally is refused by `--force-if-includes`, so a session overwrites only its own work. A rebased claim branch passes, since its old tip is in its reflog. It prints git's message, the remote's lines included, and `pushed <branch> <sha>` on success. It retries nothing: a transient refusal is the caller's to retry, as the prompts say (`orchestrator.run/prompt-commands`).

**Refused branches.** Both refuse, before any git call that writes, `main` and any branch whose name starts with `clm/`, and `branch push` refuses a detached `HEAD`. These are refusals of the command, not of the guard: the guard sees only `clm branch ...`, never the `git push` it runs (`plugin.hooks.guard/push-guard`), so the refusal must be the command's own.

**Deleting only what this clone pushed** (`branch-delete`). The project's allow rules let an unattended session run `clm branch delete` with no classifier judging it (`unattended-steps-run-clm`), so the command itself must not release another session's claim or delete its spec branch, which would bypass the claim's expiry and takeover (`lease-claims`). `clm branch delete` therefore deletes only a tip this clone pushed: before the delete it reads the newest entry of `refs/remotes/origin/<branch>`'s reflog (`git reflog show -n 1 --format='%H %gs' refs/remotes/origin/<branch>`), and goes on only when that entry is git's `update by push` and its commit is the lease. Git writes that entry when a push from this clone to `origin` moves the branch, whether `clm claim`, `clm branch push`, `clm autosync` or a plain `git push` made it, and a fetch that brings another clone's tip writes a different entry, so a tip another session pushed is refused even after this clone fetched it. A fetch that finds the branch unchanged writes no entry, so a session's own branch stays deletable after `clm rebase` fetches. No reflog, as when `core.logAllRefUpdates` is off, refuses: it fails closed. Before that check it asks `origin` with `git ls-remote origin refs/heads/<branch>`, and a branch already gone, or a clone with no `origin`, is nothing to do and exits 0, as before, so a draft deleting an `autosync/spec/<id>` that was never pushed doesn't fail. The check is the subcommand's, not the `Host` trait's: `clm land`'s delete of the branch it just landed calls the library operation, which keeps its plain lease.

`pr open` on a branch that already has an open pull request prints that pull request's URL instead of opening another. `pr body` exits 1 without a call when the file is missing or holds only whitespace, or when the current branch has no open pull request. It replaced `clm pr-body`, which went with `orchestrator.watch` (`20261027-remove-watch-node`): `pr body` is what the prompts and the skills name.

**Contribution operations.** On `github` the fork's parent and the head's details come from `gh repo view --json isFork,parent` and `gh pr view --json`, a fork's `main` is brought up with `gh repo sync`, a pull request on the parent opens with `gh pr create --repo <parent> --base main --head <owner>:<branch>`, a head is fetched with `git fetch origin pull/<number>/head:refs/heads/<branch>`, and comments, closing and labels use `gh issue` or `gh pr`. What they read from the host is a stranger's text, so a caller prints it as data and acts on none of it.

## ADDED Requirements

### Requirement: azure-pull-requests
On `host = "azure"`, `clm pr open <branch> --title <title> --body-file <file>` SHALL print the web URL (`<repository web URL>/pullrequest/<id>`) of the branch's active pull request when `az repos pr list --source-branch <branch> --status active` lists one, and otherwise SHALL open one from the branch to `main` with that title and the file's contents as its description through `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file>`, and print its web URL. `clm pr body <file>` SHALL set the description of the current branch's active pull request to the file's contents through `az repos pr update --id <id> --description @<file>`, changing nothing else, and print `updated #<id>`; it SHALL exit 1 without a call to `az` when the file is missing or blank, and with no `az repos pr update` when the branch has no active pull request. Every `az` call SHALL pass `--detect true` and `--output json` and go through `clm-git`'s `run`. When `az` fails, as it does when it isn't installed, lacks the `azure-devops` extension or isn't signed in, the command SHALL exit 1 printing `az`'s message.

#### Scenario: opened
- GIVEN `host = "azure"` and a fake `az` that records its calls, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN `az` got `repos pr create` with `--source-branch spec/x`, `--target-branch main`, `--title x` and `--description @body.md`, and it prints `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: already open
- GIVEN the same, with a fake `az` whose `pr list` reports active pull request 12 for `spec/x`
- WHEN `clm pr open spec/x ...` runs
- THEN `az repos pr create` isn't called and it prints that pull request's web URL

#### Scenario: body updated
- GIVEN `host = "azure"`, the current branch `spec/x` with active pull request 12 according to a fake `az`, and a file `body.md`
- WHEN `clm pr body body.md` runs
- THEN `az` got `repos pr update` with `--id 12` and `--description @body.md` and nothing else that changes the pull request, and it prints `updated #12`

#### Scenario: az not signed in
- GIVEN `host = "azure"` and a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN `clm pr open spec/x ...` runs
- THEN it exits 1 and its output holds `az login`

#### Serves
- capability/git-host
- architecture/host-adapters

### Requirement: azure-other-operations
On `host = "azure"`, `clm-host`'s operations other than `pr open`, `pr body`, `branch push` and `branch delete` SHALL behave as on `git`, running no `az` and no `gh`: `issue` SHALL print the report and say there is nowhere to file it, exiting 1; `parent` SHALL return nothing; `list` and `alerts` SHALL return an empty list; and `sync`, `open_on_parent`, `show`, `fetch`, `comment` and `label` SHALL report that there is no host.

#### Scenario: issue
- GIVEN `host = "azure"` and fake `az` and `gh` that record their calls
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it prints the report, says there is nowhere to file it, exits 1, and neither fake was called

#### Scenario: contributions and alerts
- GIVEN the same
- WHEN `parent`, `list`, `alerts` and `show` for 7 run
- THEN `parent` returns nothing, `list` and `alerts` return nothing, `show` reports that there is no host, and neither fake was called

#### Serves
- capability/git-host
- architecture/host-adapters
- architecture/yagni

## MODIFIED Requirements

### Requirement: git-through-layer <!-- base: c7361079b2bf2c90 -->
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
