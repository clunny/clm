---
component: host
base: { contract: "412ecf8ea6f7cfb6" }
---

## MODIFIED Contract

The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), `azure` (the Azure CLI, `az`, with its `azure-devops` extension), and `git` and `none`, which have no host and answer every host operation with nothing to do, `none` keeping the clone's own remote (`keep_remote`) besides. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open [<branch>] [--title <title>] [--body-file <file>]`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. The branch defaults to the checked-out one, and the title and body to ones drawn from the branch: its change's `## Intent` on a `spec/<id>` branch, otherwise its commits' subjects (`pr-open-defaults`), so a bare `clm pr open` works wherever a skill or a person writes it. A body that doesn't reach the pull request whole, because the host refused the call or its limit cut the body, is kept outside every worktree and the command names where (`keep-unopened-body`). The body of a spec branch that cuts anything starts with a `## Cuts` section, the lines `clm gate` prints for it, so the human reads what the branch removes where they read the diff (`gate/cuts`, `removal-needs-a-yes`). Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents, after the `## Cuts` section when the branch cuts anything, and changes nothing else (`own-pr-body`), cut to the host's limit as `pr open` cuts it, Azure DevOps keeping 4000 characters (`azure-body-limit`).
- `branch push`: pushes the checked-out branch to `origin`, overwriting `origin`'s branch only when its tip is the one last fetched and the branch has held it, plain git on every host. It is how an unattended session pushes its rebased branch with a `clm` command alone (`unattended-steps-run-clm`).
- `branch delete <branch> [--lease <sha>]`: deletes the remote branch when its tip is still `<sha>`, by default the tip last fetched, and only a tip this clone pushed, so a session can release its own claim but never another's, plain git on every host.
- `keep_remote(root, host)`, a library call: on `none`, a clone with no `origin` gets one of its own, a bare repository in its git directory, so every command, claim and land runs through `origin` as on any other host (`none-keeps-its-own-remote`). On every other host it does nothing.

Neither ever touches `main` or a branch under `clm/`, the ledgers' (`guard-blocks-push-to-main`): `main` moves only through `clm land` (`land-locally`).
- `issue --title <title> --body-file <file> --repo <repo>`: files an issue where the host keeps them and prints its URL; on `azure`, `git` and `none` it prints the report and says there is nowhere to file it. `clm defect` files through the `github` adapter's `issue` whatever the `host` setting, since its reports go to CLM's own repository on GitHub, not the project's host (`defects-file-on-github`).

- the contribution operations, for a project taking contributions from people without write access (`outside-contributions-are-intent`): the fork's parent; bringing a fork's `main` up to its parent's; opening a pull request on the parent; listing the open issues and the open pull requests from forks (`list-contributions`), not those of branches pushed to the repository itself, which `pr open` opens for reading a diff; and, for a pull request or issue by number, reading it, fetching a pull request's head, commenting, closing and labelling. `azure`, `git` and `none` have no such thing.
- the host a remote's URL names, for `clm init` (`host-for-remote`), and the steps a person takes to set a repository on that host up for CLM, which `clm init` prints (`host-setup-steps`), so `clm init` holds no host's names or permissions itself.
- the open dependency alerts, for `clm intake` (`list-alerts`): each alert's number, severity, ecosystem, package and manifest path, so an alert reaches the backlog as an issue does (`dependency-alerts-through-intake`). `azure`, `git` and `none` have none.
- the runs of a named pipeline on `main`, for `clm verify` (`list-runs`): each run's ID, head commit, whether it has completed, its conclusion and its URL, so a project whose slow tests run in its host's pipeline has their results recorded (`verify-after-land`). Only `github` lists them; the others answer that the host lists no pipeline runs.

A caller never runs `gh` or `az` itself; everything host-specific is here, and a new host is a new adapter. One caller is exempt, by name (`git-through-layer`): `clm-hooks`' release guard, which looks up the GitHub Actions run that a session's own `gh run rerun` targets, on the repository and host that command names, whatever the `host` setting says (`release-guard-runs-gh`).

`clm-host` offers each of `clm pr`, `clm issue` and `clm branch` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the repository root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. `crates/clm` routes the command to it and adds nothing to what it writes.

On every host, `clm pr` and `clm issue` exit 2 when the host's tool, `gh` or `az`, fails or gives an answer the adapter can't read, and 1 when the command itself refuses, as its requirements state, such as for a blank body file or a branch with no pull request (`host-tool-failure`), so a caller reads a host's failure the same way on `github` and on `azure`.

## ADDED Requirements

### Requirement: host-tool-failure
When the host's tool, `gh` on `github` or `az` on `azure`, can't be run, exits non-zero, or gives an answer the adapter can't read, `clm pr open`, `clm pr body` and `clm issue` SHALL exit 2, with a message that holds what the tool printed on its error output, or why its answer can't be read, on every host that has the tool. A refusal that a requirement of `host` states as exit 1, such as `pr-body`'s for a blank body file or a branch with no pull request, `pr-open`'s for a gate that can't be evaluated, or `issue`'s on a host with nowhere to file it, SHALL stay exit 1, so a caller tells a host's failure from the command's refusal the same way whichever host the project uses.

#### Scenario: gh fails to open a pull request
- GIVEN `host = "github"` and a fake `gh` whose `pr view` finds no pull request and whose `pr create` exits 1 printing `create failed`
- WHEN `clm pr open spec/x --title x --body-file body.md` runs
- THEN it exits 2 and its error output holds `create failed`

#### Scenario: gh fails to file an issue
- GIVEN `host = "github"` and a fake `gh` whose `issue create` exits 1 printing `create failed`
- WHEN `clm issue --title x --body-file r.md --repo o/r` runs
- THEN it exits 2 and its error output holds `create failed`

#### Scenario: az fails to set a body
- GIVEN `host = "azure"`, the current branch `spec/x`, a file `body.md` holding `## Intent`, and a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN `clm pr body body.md` runs
- THEN it exits 2 and its error output holds `az login`

#### Scenario: an answer that can't be read
- GIVEN `host = "github"`, the current branch `spec/x`, a file `body.md` holding `## Intent`, and a fake `gh` whose `pr view` prints `not json`
- WHEN `clm pr body body.md` runs
- THEN it exits 2 and its error output holds `bad JSON`

## MODIFIED Requirements

### Requirement: azure-selected <!-- base: 288b93b307375762 -->
With `host = "azure"`, `clm-host`'s `adapter` SHALL give the `Azure` adapter (`azure-pull-requests`, `azure-other-operations`), so `clm pr open` and `clm pr body` run `az` as those requirements say, `clm issue` and the contribution operations behave as on `git`, and a failure of `az` makes the command exit 2 printing `az`'s message (`host-tool-failure`). `adapter`'s arm for the `azure` value gives it, and this requirement's tests prove it through the commands. `capability/git-host`'s Azure DevOps scenario, which drives `clm pr open` with a stubbed `az`, sees the adapter once this requirement holds.

#### Scenario: pull request through the setting
- GIVEN a repository `clm.toml` with `host = "azure"` and a fake `az` that records its calls, lists no active pull request for `spec/x`, and answers `pr create` with `pullRequestId` 12 and `repository.webUrl` `https://dev.azure.com/o/p/_git/r`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN `az` got `repos pr create`, and it prints `https://dev.azure.com/o/p/_git/r/pullrequest/12`

#### Scenario: az not signed in
- GIVEN `host = "azure"` and a fake `az` that exits 1 printing `Please run 'az login'`
- WHEN `clm pr open spec/x --title "x" --body-file body.md` runs
- THEN it exits 2 and its output holds `az login`

#### Scenario: issue through the setting
- GIVEN `host = "azure"` and fake `az` and `gh` that record their calls
- WHEN `clm issue --title "x" --body-file r.md --repo clunny/clm` runs
- THEN it prints the report, says there is nowhere to file it, exits 1, and neither fake was called

#### Serves
- capability/git-host
- architecture/host-adapters

#### Needs
- config/host-setting

### Requirement: keep-unopened-body <!-- base: 34bd03bebb9ec7ad -->
When the host's `pr open` fails after `clm pr open`'s own checks have passed, on `github` or `azure`, `clm-host`'s `pr_open` SHALL first write the body it was opening with (the `--body-file`'s contents, or the body drawn from the branch, in either case with the `## Cuts` section `pr-open` puts first for a branch that cuts anything) to `<common git dir>/clm/pr-bodies/<branch>.md`, `<common git dir>` being `clm-git`'s `common_dir` and each `/` of the branch's name written as `-`, creating the directories and replacing an earlier copy, and its error SHALL end `; the body is kept at <path>` with that file's absolute path, so `clm pr open` exits with the status `host-tool-failure` gives a failing host's tool, 2, naming it. The `Azure` adapter SHALL write the whole body to the same path when it cuts one (`azure-body-limit`). When the gate can't be evaluated (`pr-open`) no body SHALL be kept, since none was made. Nothing else SHALL be written there: a pull request that opens with its whole body leaves no copy.

#### Scenario: the host refuses
- GIVEN `host = "azure"`, a fake `az` that lists no active pull request for `impl/a` and exits 1 on `pr create` printing `TF401398`, and `.clm/pr-body.md` holding `## Review`
- WHEN `clm pr open impl/a --title x --body-file .clm/pr-body.md` runs
- THEN it exits 2, its output holds `TF401398` and the path of `<common git dir>/clm/pr-bodies/impl-a.md`, and that file holds `## Review`

#### Scenario: a kept body holds its cuts
- GIVEN `host = "github"`, a fake `gh` whose `pr create` exits 1, and a branch `spec/x` that removes requirement `a/x`
- WHEN `clm pr open spec/x --title x --body-file body.md` runs
- THEN it exits 2, and the kept file starts `## Cuts` and `- specs/a/SPEC.md: gate/cut: requirement a/x is gone`, followed by the contents of `body.md`

#### Scenario: from a linked worktree
- GIVEN the same, run in a linked worktree of the clone
- WHEN `clm pr open impl/a --title x --body-file .clm/pr-body.md` runs and the worktree is then removed
- THEN `<common git dir>/clm/pr-bodies/impl-a.md`, under the main repository's git directory, still holds `## Review`

#### Scenario: opened
- GIVEN `host = "github"` and a fake `gh` that prints a URL
- WHEN `clm pr open impl/a --title x --body-file body.md` runs
- THEN it prints the URL and no `clm/pr-bodies/` file exists
