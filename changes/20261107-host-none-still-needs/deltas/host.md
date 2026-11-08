---
component: host
base: { contract: "e4d71f09f6317efb" }
---

## MODIFIED Contract
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), `azure` (the Azure CLI, `az`, with its `azure-devops` extension), and `git` and `none`, which have no host and answer every host operation with nothing to do, `none` keeping the clone's own remote (`keep_remote`) besides. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open <branch> --title <title> --body-file <file>`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents and nothing else (`own-pr-body`).
- `branch push`: pushes the checked-out branch to `origin`, overwriting `origin`'s branch only when its tip is the one last fetched and the branch has held it, plain git on every host. It is how an unattended session pushes its rebased branch with a `clm` command alone (`unattended-steps-run-clm`).
- `branch delete <branch> [--lease <sha>]`: deletes the remote branch when its tip is still `<sha>`, by default the tip last fetched, and only a tip this clone pushed, so a session can release its own claim but never another's, plain git on every host.
- `keep_remote(root, host)`, a library call: on `none`, a clone with no `origin` gets one of its own, a bare repository in its git directory, so every command, claim and land runs through `origin` as on any other host (`none-keeps-its-own-remote`). On every other host it does nothing.

Neither ever touches `main` or a branch under `clm/`, the ledgers' (`guard-blocks-push-to-main`): `main` moves only through `clm land` (`land-locally`).
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL, for `clm defect` (`defect-reports`); on `azure`, `git` and `none` it prints the report and says there is nowhere to file it, since `clm defect`'s reports go to CLM's own repository on GitHub, not the project's host.

- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; listing the open issues and the open pull requests from forks (`list-contributions`), not those of branches pushed to the repository itself, which `pr open` opens for reading a diff; and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `azure`, `git` and `none` have no such thing.
- the host a remote's URL names, for `clm init` (`host-for-remote`), and the steps a person takes to set a repository on that host up for CLM, which `clm init` prints (`host-setup-steps`), so `clm init` holds no host's names or permissions itself.
- the open dependency alerts, for `clm intake` (`list-alerts`): each alert's number, severity, ecosystem, package and manifest path, so an alert reaches the backlog as an issue does (`dependency-alerts-through-intake`). `azure`, `git` and `none` have none.
- the runs of a named pipeline on `main`, for `clm verify` (`list-runs`): each run's ID, head commit, whether it has completed, its conclusion and its URL, so a project whose slow tests run in its host's pipeline has their results recorded (`verify-after-land`). Only `github` lists them; the others answer that the host lists no pipeline runs.

A caller never runs `gh` or `az` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## ADDED Requirements

### Requirement: none-keeps-a-remote
`clm-host` SHALL offer `keep_remote(root, host)`. When `host` is `none` and the clone has no remote named `clm-git`'s `REMOTE`, it SHALL make `clm/remote.git` under the clone's common git directory (`clm-git`'s `common_dir`) a bare repository, keeping one already there, add it as `REMOTE` by its absolute path, and return that path; it SHALL push nothing and write nothing in any worktree. On any other host, or when the clone has `REMOTE`, it SHALL change nothing and return nothing. Run from a linked worktree, it SHALL give every worktree of the clone the same `origin`.

#### Scenario: no remote
- GIVEN `host = "none"`, a clone with a commit on `main` and no remote
- WHEN `keep_remote` runs
- THEN `<common git dir>/clm/remote.git` is a bare repository, `git remote get-url origin` prints its absolute path, it has no branch, and the working tree is unchanged

#### Scenario: from a linked worktree
- GIVEN the same clone with no remote, and a linked worktree of it
- WHEN `keep_remote` runs in the linked worktree
- THEN the bare repository is under the common git directory, not the worktree's, and `git remote get-url origin` prints the same path in both checkouts

#### Scenario: already kept
- GIVEN the clone after *no remote*, its `origin` then removed with `git remote remove origin`
- WHEN `keep_remote` runs
- THEN `origin` names the same bare repository again, and what it held is still there

#### Scenario: nothing to do
- GIVEN `host = "none"` with an `origin` naming another bare repository, and in turn `host = "git"` with no remote
- WHEN `keep_remote` runs in each
- THEN the remotes are unchanged, no `clm/remote.git` exists, and it returns nothing
