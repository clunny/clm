---
component: host
base: { contract: "e4a148cc152ece1d" }
---

## MODIFIED Contract
The one place CLM talks to a git host beyond `git` itself (`host-adapters`). The `host` setting picks the adapter: `github` (`gh`), `azure` (the Azure CLI, `az`, with its `azure-devops` extension), and `git` and `none`, which have no host and answer every host operation with nothing to do, `none` keeping the clone's own remote (`keep_remote`) besides. Crate `clm-host` offers, as library calls and as `clm` subcommands:
- `pr open [<branch>] [--title <title>] [--body-file <file>]`: opens a pull request for the branch against `main` on the host and prints its URL, or `none`. The branch defaults to the checked-out one, and the title and body to ones drawn from the branch: its change's `## Intent` on a `spec/<id>` branch, otherwise its commits' subjects (`pr-open-defaults`), so a bare `clm pr open` works wherever a skill or a person writes it. Nothing in CLM waits on a pull request: it is where a human reads the diff and where the host records the merge.
- `pr body <file>`: sets the body of the current branch's pull request to the file's contents and nothing else (`own-pr-body`).
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

