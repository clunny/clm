---
status: proposed
resolves: [azure-devops-host]
---

# Azure DevOps as a git host

## Intent

Let a team whose repository lives on Azure DevOps use CLM's core (the CLI, the plugin and `clm land`), as a GitHub team does. `host = "azure"` adds an adapter in `host` that opens and updates pull requests through the Azure CLI (`az repos pr`), `clm init` recognises Azure DevOps remotes and says what an Azure repository needs, and `/clm:init` handles Azure Pipelines and branch policies as it handles GitHub workflows. `CAPABILITIES.md`'s `git-host` names Azure DevOps.

The web app's support for Azure (a replica source over Azure's REST API, sign-in through Microsoft Entra ID, and the review flow) is split out to `backlog/azure-devops-web`. Trying `/clm:init` on an Azure repository raised how CLM sits beside a team's CI and CD; the answer is recorded as `deploy-is-downstream` and written up in the guide's new page `ci-and-deployment`, and the slow-test tier it calls for is `backlog/slow-tests-after-land`.

## Problem

`backlog/azure-devops-host` asked for Azure DevOps support. Running `/clm:init` on an Azure DevOps repository showed what stands in the way today:
- `host` takes only `github`, `git` and `none`, so `clm pr open` can't open the pull request a reviewer reads a spec diff in;
- `main` there has blocking build-validation policies, so a direct push to `main`, which `clm land` makes, is refused;
- its pipelines are the merge gate and deploy `main` on every merge, with integration tests on a test environment, and slow tests run in the pull request's pipeline before the merge;
- `/clm:init` knows only `.github/workflows/`, and the session read its CI step as deleting every pipeline, deploys included, and offered to adopt CLM with pull requests landing through the host, which `land-locally` (a `method` constraint) rules out.

## Approaches

How the adapter talks to Azure DevOps:
- **A. The Azure CLI, `az repos pr` (chosen).** Mirrors `github` through `gh`: `az` holds the sign-in (`az login`, or `AZURE_DEVOPS_EXT_PAT`), works out the organisation, project and repository from the remote, and CLM handles no token. Costs the user the Azure CLI and its `azure-devops` extension, which `clm init` names.
- **B. The REST API with a personal access token.** No CLI to install, but `clm-host` gains an HTTP client and CLM takes on reading and scoping a secret, for the same two operations.
- **C. No adapter: Azure users set `host = "git"`.** Everything works but opening pull requests, so reviewers lose the place they read a spec diff. Cheapest, and it fits `yagni`, but drops the one thing a host adds.

How landing meets the host's branch policies:
- **Keep `land-locally` (chosen).** `clm land` still checks the merged tree and pushes `main`; the identity that runs it gets Azure's "Bypass policies when pushing" on `main`, as a GitHub bypass does (`land-locally`'s decision already says a protected `main` is fine when the lander may push it). The build-validation policies can stay; they no longer gate. Pipelines triggered by a push to `main` keep running after a land, for deploys and as a backstop.
- **A host-gated landing mode**, where `clm land` completes the host's pull request and its pipelines decide. Rejected: it reverses `land-locally`, a `method` constraint, and makes the host the decision-maker again (`host-adapters`).

## Outcome

**The adapter** (`host`). `host = "azure"` selects it. Through `clm-git`'s `run`, never a process of its own:
- `pr open <branch>`: `az repos pr list --source-branch <branch> --status active --detect true` finds an open pull request and prints its URL; otherwise `az repos pr create --source-branch <branch> --target-branch main --title <title> --description @<file> --detect true` opens one and prints its URL. The URL is the web URL of the pull request (`<repository web URL>/pullrequest/<id>`), built from the JSON `az` returns.
- `pr body <file>`: finds the current branch's active pull request as above and runs `az repos pr update --id <id> --description @<file> --detect true`, printing `updated #<id>`; the same refusals as `github` (missing or blank file, no open pull request).
- `issue`, the contribution operations, `list` and `alerts` behave as on `git`: `issue` prints the report and says there is nowhere to file it, since `clm defect` files on CLM's own GitHub repository, not the project's; `parent` returns nothing; `list` and `alerts` return nothing; the rest report that the host has no such thing. No capability asks for Azure work items, forks or dependency alerts yet (`yagni`).
- `branch push` and `branch delete` are plain git on every host, unchanged.
- No crate but `clm-git` and `clm-host` runs `az`, which `git-through-layer`'s scan checks as it checks `gh`.

**Settings** (`config`). `host` accepts `azure`.

**`clm init`** (`init`). With no `--host`, an `origin` URL whose host is `dev.azure.com`, `ssh.dev.azure.com` or ends in `.visualstudio.com` gives `host = "azure"`; `--host azure` sets it. On `azure`, `Next steps (yours):` names installing the Azure CLI with its `azure-devops` extension and signing in, and the protected-`main` step names Azure's "Bypass policies when pushing" for the identity that runs `clm land`. It still calls no host.

**`/clm:init`** (`plugin`). Its CI step also reads Azure Pipelines (`azure-pipelines.yml`, and pipeline YAML the repository holds elsewhere, such as `.azure-pipelines/`): build and test steps move into `scripts/clm-check.sh`, and those pipelines (or their build-and-test stages) are removed; deploy, release and scheduled pipelines stay, as GitHub's do. It tells the human that build-validation branch policies stop being the gate once `clm land` lands, that the lander needs the bypass permission, and, for a deploy pipeline triggered by `main` with no path filter, that spec-only lands (`specs/`, `changes/`, `backlog/`) would trigger it too, suggesting the filter. It never removes a branch policy or a pipeline on the host: those are the human's.

**Guide.** A new page, `ci-and-deployment`: `clm land` is the project's CI (every tree on `main` was checked merged); deployment is downstream and the project's own (`deploy-is-downstream`); integration tests on a test environment gate promotion after `main`, or, for a team that needs an integration-green `main`, run from `[land] check`, with what that costs in serial lands; slow tests either run from `[land] check` for the components `clm affected` selects, or after the land, pending `backlog/slow-tests-after-land`; and a protected `main` on GitHub or Azure DevOps needs the lander allowed to push it.

**Capabilities.** `git-host` reads "GitHub, Azure DevOps, a bare git server, or no host" and gains a scenario for Azure; `capability`'s test for it drives `clm pr open` with a stubbed `az`. `ARCHITECTURE.md`'s `platform` names Azure DevOps among the optional hosts.

## Constraints

- `host-adapters`: everything Azure-specific is in `clm-host`; a repository on Azure DevOps still works with `host = "git"` or `none`.
- `land-locally`, `guard-blocks-push-to-main`: unchanged; nothing pushes `main` but `clm land`.
- `yagni`: no Azure work items, forks, alerts or web support until a capability asks.
- `web-optional`: the web app's Azure support is a separate idea; nothing here touches `web`, `web.worker` or `replica`.
- No migration: `azure` is a new accepted value, and a project's existing `clm.toml` is unchanged (`upgrades-carry-migrations`).

## Left to the builder

- How `az`'s JSON is parsed for the pull request's ID and URL, and the fake `az` the tests use.
- Where `--host azure` is parsed, alongside the other values.
