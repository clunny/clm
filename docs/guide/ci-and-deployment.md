# CI and deployment

How CLM sits beside a team's pipelines: what it replaces, what it leaves alone, and where slow and integration tests go.

## `clm land` is your CI

Continuous integration means every change joins `main` quickly and `main` always passes its checks. `clm land` does exactly that. It merges the branch with the current `main`, runs the project's check command (`[land] check`, usually `scripts/clm-check.sh`) and `clm check` on the merged result, and pushes `main` only when they pass. Lands go one at a time, so the tree that reaches `main` is the tree that was checked. A pull request pipeline usually tests the branch, not the merge.

The checks run on the machine that runs `clm land`. If your team wants a controlled environment, run `clm land` on a shared machine you trust. CLM generates no pipeline for it.

When the check fails, `clm land` repeats its last 40 lines and keeps its whole output at `.clm/land-logs/<branch>.log`. If your check script runs several steps, print `::group::<name>` as each one starts and `::endgroup::` as it ends, as GitHub Actions does. The refusal then names the step that was running when the check failed and the lines of the log it spans, so a session reads the failure even when it lies more than 40 lines back.

### Tests that read your specs and docs

A land whose code was already checked skips most of `[land] check`. That happens when a branch changes only doc-only paths (`specs/`, `changes/`, `backlog/`, `docs/` outside `docs/guide/` and `docs/spec-format.md`, `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md` and `CLAUDE.md`), or changes `clm.toml` only in its comments or its upgrade notes (`upgraded`, `declined_migrations`): the land then runs `clm check` and only the tests that say they read a file the branch changes. A test says so with a `clm:reads <glob>` marker anywhere in its file, usually a comment, whose glob matches each such path it reads, as a component's `paths` do: `clm:reads ARCHITECTURE.md`, `clm:reads specs/**`, `clm:reads clm.toml`. A file may hold several.

So when a test reads a file of your project's own tree under one of those paths, or reads your `clm.toml`, such as a test that checks your specs for a retired term, give it the marker. Without one, a branch that changes only that file lands without running the test, and can leave `main` failing it until the next land that runs every test. `clm affected --readers` prints the test files whose markers match what the current branch changes, and `clm land` hands that list to `[land] check` in `CLM_SELECTION`, as it hands the selection of a code land. A test that reads only fixtures it writes itself, or files outside those paths, needs no marker. Markdown files are never readers.

## Deployment is yours

Every commit on `main` is checked, so every commit is deployable. Whether and when you deploy is your choice:

- keep deploy, release and scheduled workflows or pipelines as they are; `/clm:init` moves only build and test steps into `scripts/clm-check.sh`;
- a pipeline triggered by a push to `main` still runs after each land, as a deploy or as a backstop; it reports, it doesn't decide;
- spec-only lands reach `main` too, so give a deploy triggered by `main` a path filter that skips `specs/`, `changes/` and `backlog/`.

## Integration tests on a test environment

Tests that need a deployed system, such as integration tests on a test cluster, can gate one of two things.

**Promotion after `main` (the default).** `main` deploys to the test environment, the integration tests run there, and only a green build goes on to production. Before the land, the checks run everything that proves the requirements with what lies outside stubbed, as CLM's tests are meant to. Lands stay fast, and a failure on the cluster usually points at a missing requirement or a wrong stub.

**`main` itself.** If `main` must be integration-green, make `[land] check` deploy the merged tree to the test environment, run the tests and fail on red. The environment runs the tests; `clm land` reads the result and decides. Spec-only lands skip it, since a tree whose code was already checked runs only `clm check`. The cost is throughput: lands are serial, so a 30-minute run allows two code lands an hour, a land whose push loses the race to another runs again, and a flaky environment blocks every land. A shared test environment also breaks `tests-run-concurrently` (`clm guide constraints`), since builds run their tests at once: the check lock keeps `[land] check` runs apart, but not tests a build runs by hand. Such a project keeps the environment out of the tests its builds run, or replaces that constraint with one of its own that keeps its guarantee.

## Slow tests

Tests too slow for every land have three places to go:

- **before the land, for what changed**: `clm affected` selects the tests for the components a branch touches, so `[land] check` can run only the slow tests a change could break;
- **after the land, with `clm verify`**: set `[verify]` in `clm.toml` and take the tests out of `[land] check`. Lands then wait only for the fast checks;
- **before a release**: a release or deploy step runs `clm verify --commit <sha>`, which passes at once when that commit already passed, and otherwise runs or reads the slow tests for it.

### When they run

`clm verify --due` runs them only when one of the project's triggers has fired since the last verified commit of `main`, and otherwise exits in seconds:

- `change-done`: a change finished (it was archived). The usual choice: a change is the piece of work you approved, so a failure points at one change;
- `queue-drained`: nothing is left to build;
- `work-done`: a build or triage session ended (`/clm:build` and `/clm:triage` run `clm verify --due --event work-done` before they wrap up). Local mode only;
- `every = { lands = N, days = D }`: a backstop, so long-running changes still get checked.

A failed last run makes any later land due, so a fix is checked at once. A run covers every test `clm affected` selects since the tests last passed.

### Where they run

- `mode = "local"`: on the machine that calls `clm verify`, in its own tree under `.clm/verify-tree`. Its check takes the clone's check lock, like every check on the clone, so a land on that clone waits while a slow run's check runs. If that wait matters, use `pipeline` mode or run `clm verify` from another clone.
- `mode = "pipeline"`: the host's pipeline runs `clm verify --due` on every push to `main` and records the result itself. Give the workflow the right to push `clm/ledger/*` (on GitHub, `permissions: contents: write`), set `CLM_WRITER` to a name for it, and in a signed repository give it a key of its own in `docs/allowed_signers`.
- `mode = "import"` with `workflow = "<name>"`: the pipeline runs the slow tests and only reports; `clm verify --import`, which `/clm:triage` runs, reads its runs on `main` and records them. The pipeline needs no write access, and can gate its slow job on `clm verify --due --dry-run`, which exits 0 when a run is due and 3 when not. GitHub only for now.

Every clone that reads a `clm.toml` with `[verify]` needs a `clm` that knows it; an older one refuses the unknown table.

### When one fails

A failed run names the land that broke it: one land, or the first red one when `clm verify` bisects the lands since the last pass along `main`'s first parents (`clm verify --bisect <sha>` does it on your machine in import mode). `clm intake` lists it, and `clm land` warns a branch touching those components. `/clm:triage` offers it as a bug on them; taking it in is your judgement that the failure is real, not a flake. From then on `clm land` refuses code lands on those components until a branch whose commit carries `Resolves: backlog/<slug>` for that bug lands. Other components keep landing, and a host's run never holds a land by itself. Fix forward on `clm claim <component> --fix backlog/<slug>`, or revert the land there with `git revert -m 1 <land>`, which leaves its requirements pending again for the queue to rebuild. Nothing rewrites `main`, so clones that already pulled need nothing.

## A protected `main`

`clm land` pushes `main`, so a host that protects `main` must let the identity that runs `clm land` push it:

- on GitHub, add it to the bypass list of the rule or ruleset that protects `main`;
- on Azure DevOps, grant it **Bypass policies when pushing** on `main`. Build-validation policies can stay; they stop being the gate, since branches no longer reach `main` through a pull request's completion.

Pull requests remain where people read a branch's diff: `clm pr open` opens one on GitHub through `gh`, and on Azure DevOps (`host = "azure"`) through the Azure CLI with its `azure-devops` extension, signed in with `az login`.
