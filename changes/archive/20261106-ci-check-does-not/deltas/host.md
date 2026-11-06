---
component: host
base: { contract: "93b78f24a4e169ee" }
---

## MODIFIED Contract
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

## ADDED Requirements

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
