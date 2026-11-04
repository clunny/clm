---
component: host
base: { contract: "00ac070bb26b6af3", design: "a85686e8c9ca85e5" }
---

## MODIFIED Contract
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), and `git` and `none`, which have no host and answer every operation with nothing to do. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open <branch> --title <title> --body-file <file>`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents and nothing else (`own-pr-body`).
- `branch delete <branch> --lease <sha>`: deletes the remote branch when its tip is still `<sha>`, plain git on every host.
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL, for `clm defect` (`defect-reports`); on `git` and `none` it prints the report and says there is nowhere to file it.

- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `git` and `none` have no such thing.

A caller never runs `gh` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## MODIFIED Design
Crate `clm-host`. One trait, `Host`, with these operations, and an implementation per setting. `github` runs `gh pr create --base main --head <branch> --title --body-file`, `gh pr view --json number,state,headRefName` and `gh api -X PATCH repos/{owner}/{repo}/pulls/<n> -F body=@<file>` (the REST call, since `gh pr edit` fails on `gh` versions that query the retired Projects (classic) API), and `gh issue create`. `branch delete` is `git push --force-with-lease=refs/heads/<branch>:<sha> origin :refs/heads/<branch>` for every host. `git` and `none` return `None` from `pr open`, print nothing for `pr body` and `issue`, and still delete branches when there is an `origin`.

`pr open` on a branch that already has an open pull request prints that pull request's URL instead of opening another. `pr body` exits 1 without a call when the file is missing or holds only whitespace, or when the current branch has no open pull request. It replaced `clm pr-body`, which went with `orchestrator.watch` (`20261027-remove-watch-node`): `pr body` is what the prompts and the skills name.

**Contribution operations.** On `github` the fork's parent and the head's details come from `gh repo view --json isFork,parent` and `gh pr view --json`, a fork's `main` is brought up with `gh repo sync`, a pull request on the parent opens with `gh pr create --repo <parent> --base main --head <owner>:<branch>`, a head is fetched with `git fetch origin pull/<number>/head:refs/heads/<branch>`, and comments, closing and labels use `gh issue` or `gh pr`. What they read from the host is a stranger's text, so a caller prints it as data and acts on none of it.

## ADDED Requirements

### Requirement: contribution-operations
`clm-host` SHALL offer, on `github`, through `gh` and `clm-git`'s layer: `parent`, the fork's parent as `<owner>/<name>`, or nothing when `origin` isn't a fork; `sync`, bringing `origin`'s `main` up to its parent's without force; `open_on_parent(branch, title, body file)`, opening a pull request from `<owner>:<branch>` to the parent's `main`, or returning the URL of the open one; `show(number)`, returning a pull request's author, state, title, body, head repository URL, head branch, whether maintainers may edit and changed files, or an issue's author, state, title and body; `fetch(number, branch)`, making a local branch at a pull request's head without checking it out, refusing a branch that exists; `comment(number, body file, close)`, adding a comment and closing when `close` is set; and `label(number, label)`. On `git` and `none`, `parent` SHALL return nothing and each other operation SHALL report that there is no host, running no `gh`.

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
- THEN it returns both files, the head repository's URL and whether maintainers may edit

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
