# CI and deployment

How CLM sits beside a team's pipelines: what it replaces, what it leaves alone, and where slow and integration tests go.

## `clm land` is your CI

Continuous integration means every change joins `main` quickly and `main` always passes its checks. `clm land` does exactly that. It merges the branch with the current `main`, runs the project's check command (`[land] check`, usually `scripts/clm-check.sh`) and `clm check` on the merged result, and pushes `main` only when they pass. Lands go one at a time, so the tree that reaches `main` is the tree that was checked. A pull request pipeline usually tests the branch, not the merge.

The checks run on the machine that runs `clm land`. If your team wants a controlled environment, run `clm land` on a shared machine you trust. CLM generates no pipeline for it.

When the check fails, `clm land` repeats its last 40 lines and keeps its whole output at `.clm/land-logs/<branch>.log`. If your check script runs several steps, print `::group::<name>` as each one starts and `::endgroup::` as it ends, as GitHub Actions does. The refusal then names the step that was running when the check failed and the lines of the log it spans, so a session reads the failure even when it lies more than 40 lines back.

### Running only the tests a branch can break

`clm land` tells your check which tests the branch can break, through the environment variable `CLM_SELECTION`. A check that ignores it runs everything, which is always safe, only slower. To use it, have your check script read the file the variable names.

**When it is set.** Every land that runs `[land] check` sets `CLM_SELECTION` to the path of a file it writes for that run, and so does `clm verify` for `[verify] check` once a run of it has passed. What the file holds:

- on a land that checks code: `clm affected --base <sha>`'s output exactly as it prints it, `<sha>` being the `main` the branch merges into;
- on a land whose code was already checked (see the next section): `clm affected --base <sha> --readers`'s output, `file` lines only;
- for `clm verify`: `clm affected`'s output since the last commit whose slow tests passed.

**When it is unset.** Your check must then run every test. That happens:

- when `clm affected` fails: `clm land` prints `clm affected failed: <reason>; the check runs every test` and runs the check without the variable, so a failed selection costs time, never a skipped test;
- under `clm land --check-only`, a build's own check before its land, which runs the whole suite;
- for `clm verify` before any run of it has passed;
- whenever someone runs the script by hand or another pipeline runs it.

**What the file holds.** One item per line, in one of three forms:

- `all <path>`: the only line in the file. The branch changes a file that no component's `paths` hold, or one that matches `[affected] all`, so every test must run;
- `node <id>`: a component the branch affects, directly or through `depends_on`. The word is `node` rather than component, since programs read these lines. Useful when your suites map to components rather than files;
- `file <path>`: a file whose tests should run: a changed file that still exists, a test that cites a requirement of an affected component, a file of an affected component that cites none, or a test whose `clm:reads` marker matches what changed (next section). Paths are relative to the repository root.

The `node` lines come first, then the `file` lines, each group sorted. An empty file means the branch changes nothing a test depends on, so no test needs to run. Run `clm affected` on a branch to see what a land would hand over.

**An example.** A script for a Rust workspace whose `crates/<name>/` directories hold packages of the same name. It runs everything when the variable is unset, empty or says `all`, and otherwise tests each package a `file` line falls in:

```sh
#!/bin/sh
set -eu

if [ -z "${CLM_SELECTION:-}" ] || grep -q '^all ' "$CLM_SELECTION"; then
  cargo test --workspace
  exit 0
fi

for package in $(sed -n 's|^file crates/\([^/]*\)/.*|\1|p' "$CLM_SELECTION" | sort -u); do
  cargo test -p "$package"
done
```

Map the lines to your own runner the same way: a path to a package, a test file or a suite. Decide what a `file` line your mapping doesn't recognise should do: this script runs nothing for it, which suits files no test exercises, and running everything is the safe choice when you aren't sure. Steps that check the whole tree quickly, such as formatting and lint, can run on every land whatever the selection.

### Tests that read your specs and docs

Your specs and other prose don't count as code for a land. When a branch changes only files under `specs/`, `changes/`, `backlog/` or `docs/` (outside `docs/guide/` and `docs/spec-format.md`), or `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md` or `CLAUDE.md`, and the code it lands on was already checked, `clm land` doesn't run your whole `[land] check`: it runs `clm check` and only the tests that say they read a file the branch changes. A test says so with a `clm:reads <glob>` marker anywhere in its file, usually a comment, whose glob matches each such path it reads, as a component's `paths` do: `clm:reads ARCHITECTURE.md`, `clm:reads specs/**`. A file may hold several.

So when a test reads one of those files of your project's own tree, such as a test that checks your specs for a retired term, give it the marker. Without one, a branch that changes only that file lands without running the test, and can leave `main` failing it until a later land happens to run it. `clm affected --readers` prints the test files whose markers match what the current branch changes, and `clm land` runs `[land] check` with `CLM_SELECTION` naming a file that lists them, as it names the selected tests of a code land. When it lists none, the land runs only `clm check`. A test that reads only fixtures it writes itself, or only code, needs no marker. Markdown files are never readers.

### A build's own check counts for its land

Before it lands, a build runs `clm land --check-only` in its own worktree: your whole `[land] check`, every test, and `clm check`. When that passes on a committed branch with no uncommitted change that already contains `origin/main`, it is recorded, and a land of the branch made before `main` moves runs no check of its own. So the check must pass from a clean checkout: one that passes only because of a file git ignores and the check doesn't make itself, such as a build output left from earlier work, can pass in the build and fail at a later land.

## Deployment is yours

Every commit on `main` is checked, so every commit is deployable. Whether and when you deploy is your choice:

- keep deploy, release and scheduled workflows or pipelines as they are; `/clm:init` moves only build and test steps into `scripts/clm-check.sh`;
- a pipeline triggered by a push to `main` still runs after each land, as a deploy or as a backstop; it reports, it doesn't decide;
- spec-only lands reach `main` too, so give a deploy triggered by `main` a path filter that skips `specs/`, `changes/` and `backlog/`.

## Integration tests on a test environment

Tests that need a deployed system, such as integration tests on a test cluster, can gate one of two things.

**Promotion after `main` (the default).** `main` deploys to the test environment, the integration tests run there, and only a green build goes on to production. Before the land, the checks run everything that proves the requirements with what lies outside stubbed, as CLM's tests are meant to. Lands stay fast, and a failure on the cluster usually points at a missing requirement or a wrong stub.

**`main` itself.** If `main` must be integration-green, make `[land] check` deploy the merged tree to the test environment, run the tests and fail on red. The environment runs the tests; `clm land` reads the result and decides. Spec-only lands skip it, since a tree whose code was already checked runs only `clm check`. The cost is throughput: lands are serial, so a 30-minute run allows two code lands an hour, a land whose push loses the race to another runs again, and a flaky environment blocks every land. A shared test environment also breaks `tests-run-concurrently` (`clm guide constraints`), since builds run their tests at once: the check lock keeps lands' `[land] check` runs apart, but not a build's own run of it before the land, which starts at once in its build place, nor tests a build runs by hand. Such a project keeps the environment out of the tests its builds run, or replaces that constraint with one of its own that keeps its guarantee.

## Slow tests

Tests too slow for every land have three places to go:

- **before the land, for what changed**: `clm affected` selects the tests for the components a branch touches, so `[land] check` can run only the slow tests a change could break;
- **after the land, with `clm verify`**: set `[verify]` in `clm.toml` and take the tests out of `[land] check`. Lands then wait only for the fast checks;
- **before a release**: a release or deploy step runs `clm verify --commit <sha>`, which passes at once when that commit already passed, and otherwise runs or reads the slow tests for it.

### When a land runs over its target

`[land] check_target_seconds` (default 120) is how long a land's check should take. To see where the time goes, the check marks its steps with `::group::<name>` and `::endgroup::` lines, and inside a step it prints `::clm-selected::<path>` for each selected file that made it run that step. `clm land` times each step and records the times in the ledger. A land over the target prints its three costliest steps, each with the chain that selected it (the same chain `clm affected --why <path>` prints: the file, the component that owns or cites it, the `depends_on` path, the changed file), and how many of the last 20 timed lands also went over. `clm health`'s `land-time` signal shows the trend over the last 50.

Nothing is filed and nothing is decided for you. Read it like this:

- **one land over the target**, from a large shared change or a machine busy with other builds, needs nothing;
- **a step reached through a long chain**, or through a rule of the check script's rather than the graph, such as a suite that always runs after another one, means the selection reaches too far. Fix the selection: a `depends_on` edge that shouldn't be there, a component whose `paths` take in files its dependents' changes can't break, or the script's mapping from files to suites;
- **a step that costs the time however it's reached** belongs after the land: move it to `[verify]`;
- **when lands keep going over** and you judge it needs work, file a backlog item naming the steps and their chains, so the fix gets designed rather than lost.

### When they run

`clm verify --due` runs them only when one of the project's triggers has fired since the last verified commit of `main`, and otherwise exits in seconds:

- `change-done`: a change finished (it was archived). The usual choice: a change is the piece of work you approved, so a failure points at one change;
- `queue-drained`: nothing is left to build;
- `work-done`: a build or triage session ended (`/clm:build` and `/clm:triage` run `clm verify --due --event work-done` before they wrap up). Local mode only;
- `every = { lands = N, days = D }`: a backstop, so long-running changes still get checked.

A failed last run makes any later land due, so a fix is checked at once. A run covers every test `clm affected` selects since the tests last passed.

### Where they run

- `mode = "local"`: on the machine that calls `clm verify`, in its own tree under `.clm/verify-tree`. Its check takes the clone's check lock, as a land's does, so a land on that clone waits while a slow run's check runs. If that wait matters, use `pipeline` mode or run `clm verify` from another clone.
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
