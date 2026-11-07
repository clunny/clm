---
component: host
base: { contract: "198a2bfc7af092a5" }
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

A caller never runs `gh` or `az` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## MODIFIED Requirements

### Requirement: list-contributions <!-- base: 806621823cacd9d7 -->
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
