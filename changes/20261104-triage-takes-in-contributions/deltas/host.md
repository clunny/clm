---
component: host
base: { contract: "edbac35aa2eb03cf" }
---

## MODIFIED Contract
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), and `git` and `none`, which have no host and answer every operation with nothing to do. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open <branch> --title <title> --body-file <file>`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents and nothing else (`own-pr-body`).
- `branch delete <branch> --lease <sha>`: deletes the remote branch when its tip is still `<sha>`, plain git on every host.
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL, for `clm defect` (`defect-reports`); on `git` and `none` it prints the report and says there is nowhere to file it.

- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; listing the open issues and pull requests (`list-contributions`); and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `git` and `none` have no such thing.

A caller never runs `gh` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

## MODIFIED Requirements

### Requirement: contribution-operations <!-- base: 6134fd80e9a2c4d1 -->
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

## ADDED Requirements

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
