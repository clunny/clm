# CI and deployment

How CLM sits beside a team's pipelines: what it replaces, what it leaves alone, and where slow and integration tests go.

## `clm land` is your CI

Continuous integration means every change joins `main` quickly and `main` always passes its checks. `clm land` does exactly that. It merges the branch with the current `main`, runs the project's check command (`[land] check`, usually `scripts/clm-check.sh`) and `clm check` on the merged result, and pushes `main` only when they pass. Lands go one at a time, so the tree that reaches `main` is the tree that was checked. A pull request pipeline usually tests the branch, not the merge.

The checks run on the machine that runs `clm land`. If your team wants a controlled environment, run `clm land` on a shared machine you trust. CLM generates no pipeline for it.

## Deployment is yours

Every commit on `main` is checked, so every commit is deployable. Whether and when you deploy is your choice:

- keep deploy, release and scheduled workflows or pipelines as they are; `/clm:init` moves only build and test steps into `scripts/clm-check.sh`;
- a pipeline triggered by a push to `main` still runs after each land, as a deploy or as a backstop; it reports, it doesn't decide;
- spec-only lands reach `main` too, so give a deploy triggered by `main` a path filter that skips `specs/`, `changes/` and `backlog/`.

## Integration tests on a test environment

Tests that need a deployed system, such as integration tests on a test cluster, can gate one of two things.

**Promotion after `main` (the default).** `main` deploys to the test environment, the integration tests run there, and only a green build goes on to production. Before the land, the checks run everything that proves the requirements with what lies outside stubbed, as CLM's tests are meant to. Lands stay fast, and a failure on the cluster usually points at a missing requirement or a wrong stub.

**`main` itself.** If `main` must be integration-green, make `[land] check` deploy the merged tree to the test environment, run the tests and fail on red. The environment runs the tests; `clm land` reads the result and decides. Spec-only lands skip it, since a tree whose code was already checked runs only `clm check`. The cost is throughput: lands are serial, so a 30-minute run allows two code lands an hour, a land whose push loses the race to another runs again, and a flaky environment blocks every land.

## Slow tests

Tests too slow for every land have two places to go:

- **before the land, for what changed**: `clm affected` selects the tests for the components a branch touches, so `[land] check` can run only the slow tests a change could break;
- **after the land**: a pipeline runs them on `main` and reports. When one fails, find the land that broke it with `git bisect --first-parent` (each land is one merge commit on `main`), and file the failure as a `bug` item on that land's component, since CLM doesn't yet bring it into the backlog by itself. Then fix forward with `clm claim <component> --fix backlog/<slug>`, or revert the land on a branch of its own and land that. Nothing rewrites `main`, so clones that already pulled need nothing.

## A protected `main`

`clm land` pushes `main`, so a host that protects `main` must let the identity that runs `clm land` push it:

- on GitHub, add it to the bypass list of the rule or ruleset that protects `main`;
- on Azure DevOps, grant it **Bypass policies when pushing** on `main`. Build-validation policies can stay; they stop being the gate, since branches no longer reach `main` through a pull request's completion.

Pull requests remain where people read a branch's diff: `clm pr open` opens one on GitHub through `gh`, and on Azure DevOps (`host = "azure"`) through the Azure CLI with its `azure-devops` extension, signed in with `az login`.
