---
component: host
base: { contract: "cd050377f21974b8" }
---

## MODIFIED Contract
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

## ADDED Requirements

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
