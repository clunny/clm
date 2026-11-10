# Settings

CLM's settings change how it behaves in your project: where reports go, how many builds run at once, whose approval counts, what runs before a branch lands. This page lists every one, what each choice gives and costs, and where it may be set. `/clm:configure` walks you through them, or straight to one (`/clm:configure set concurrent builds`), and makes the edit for you.

## Where a setting lives

Each setting's value comes from the first of these that sets it:

1. its environment variable, for the few settings that have one;
2. `clm.local.toml`: this clone's, never committed. It's read from the repository's directory in the clone's main worktree (the first one `git worktree list` prints), whichever worktree you run in, so one file serves them all; `clm config list` prints its path;
3. `clm.toml` at the repository's root: the project's, shared by everyone;
4. `~/.config/clm/config.toml` (or under `$XDG_CONFIG_HOME`): yours, for every project;
5. its default.

A table such as `[land]` is one setting: the first file that sets it supplies all of it, and files never merge within a table.

Not every file may set every setting. The project's settings, which decide how its checks and reviews run, can be set only in `clm.toml` or `clm.local.toml`, so nobody weakens a project's checks from their own file. `writer` names a person, so `clm.toml` never sets it. Each section below says where its setting may go.

`clm config list` prints every setting with its value and where it came from; `clm config get <key>` prints one. Both fail, naming the file and the setting, when a value isn't one CLM accepts.

## How a change lands

- An edit to `clm.local.toml` or to your own `config.toml` takes effect at once and is never committed.
- An edit to `clm.toml` is a commit like any other: it goes on a branch of its own and lands with `clm land`, on checks.
- An edit to `clm.toml`'s `approval` needs a human's approval before it lands, since it changes whose approval counts.

## The project's process

### `approval`

Whose approval lets a branch that needs review land. `any` (the default): anyone's, the author's included. `other`: only someone who wrote none of the branch's commits.

- **`any`** suits a person working alone, who reviews their own intent. It's also the only value that works with one person.
- **`other`** gives a team a second pair of eyes on every reviewed change, at the cost of waiting for someone else.

Where: `clm.toml` or `clm.local.toml`. Changing it in `clm.toml` needs review.

### `defects`

Where `clm defect` sends a report when CLM itself gets in a session's way. `ask` (the default): an issue on CLM's own GitHub repository, filed once you confirm it. `backlog`: a `kind: bug` item in this project's backlog. `off`: no report.

- **`ask`** gets CLM's maintainers the report, and asks you each time, since it leaves the project.
- **`backlog`** keeps reports in the project, with no question: suits a project that fixes CLM itself, or one whose work mustn't leave it.
- **`off`** saves the interruption and loses the reports.

Where: any file, or `CLM_DEFECTS`.

### `land`

`[land]` with `check`: the command `clm land` runs on the tree it lands, before `clm check`, such as your build and test script. Optional `target_cap_gb`: the size, in gigabytes, above which a land removes its shared build directory `.clm/land` after its push, and `clm verify` removes its own, `.clm/verify-build`, after each run of `[verify] check` (default 15). Optional `check_target_seconds`: how long a land's check should take at most (default 120).

- **Set `check`** so nothing lands on `main` without your tests passing on this machine. Each land then takes as long as the command does.
- **Leave it unset** and `clm land` runs only `clm check`: fast, but nothing runs your tests before a land.
- **A higher `target_cap_gb`** keeps builds incremental between lands and between verify runs, at the cost of disk; a lower one frees disk and makes the next land or verify run rebuild from scratch.
- **`check_target_seconds`** changes nothing a land does: a land over it prints where its time went and what selected each costly step, so whoever landed it can judge whether the selection reaches too far or a suite belongs after the land. A lower target reports sooner and more often; a higher one stays quiet until lands are slower.

Where: `clm.toml` or `clm.local.toml`.

### `build`

`[build]` with `setup`: the command that makes a fresh worktree of the repository ready to run its tests, such as one installing its dependencies into it. `clm setup` runs it, and builds, drafts and the skills' code fixes run `clm setup` before their first test run, because each build works in a fresh worktree inside your main checkout, where a toolchain that looks in parent directories, or an environment your checkouts share, would otherwise find the main checkout's packages or source and test the wrong code (`tests-run-in-their-own-tree`). For npm packages, `setup = "test ! -f package.json || npm ci"`.

- **Set `setup`** when a fresh worktree's tests would fail for want of dependencies, or find them outside the worktree. Each build then spends the install's time once, and again when its dependencies change.
- **Leave it unset** when your tests need nothing installed in a fresh worktree, as a cargo workspace doesn't. Builds then run their tests straight away.

`[land] check` never runs it: keep your check script installing what it needs, since CI and `[verify]` run it in trees no build prepared.

Where: `clm.toml` or `clm.local.toml`.

### `verify`

`[verify]`: slow tests that run after a land instead of before it, so lands stay fast. `check`: the slow-test command. `mode`: `local` (`clm verify` runs it on this machine), `pipeline` (a host pipeline runs it and records the result) or `import` (`clm verify` reads the runs of the pipeline `workflow` names). Optional `triggers` (`change-done`, `queue-drained`, and `work-done` with `local` only) and `every` (`lands`, `days`): when a run is due.

- **Unset** (the default): slow tests, if any, stay in `[land] check`, and every land waits for them.
- **Set** to keep lands fast: a failing run is reported after the land, `/clm:triage` offers it to you, and once you take it in as a bug, code lands on the components it names wait until its fix lands. You trade catching a break before `main` for lands that don't wait.

Where: `clm.toml` or `clm.local.toml`. `clm verify` reads `clm.toml`'s `[verify]` as committed on `origin/main`, never from the checkout it runs in, so an edit takes effect once it lands.

### `host`

Where the repository's remote lives, which decides how pull requests are opened: `github` (the default, through `gh`), `azure` (Azure DevOps, through `az`), `git` (a plain git server) or `none`.

- **`github`** and **`azure`** give each branch a pull request to read its diff in, and need that host's CLI signed in.
- **`git`** and **`none`** need no account or CLI, and open no pull requests. Branches are still pushed and deleted on `origin`.
- **`none`** is for a repository with no remote. `clm` gives the clone one of its own, a bare repository at `clm/remote.git` in its git directory, added as `origin`, so claims, builds and lands run as on any host. Its path is absolute: after moving the repository, run `git remote set-url origin <new path>`. To move onto a real remote, run `git remote set-url origin <url>`, set `host`, and run `clm reconnect`.

Where: `clm.toml` or `clm.local.toml`.

### `detectors`

`[[detectors]]`: the dead-code and unused-dependency tools `clm check` runs, so code no requirement needs doesn't stay. Each has `name`, `command` and optional `hint` (how to install the tool), `timeout` (seconds, default 600) and `requires` (a path from the repository's root: the detector is skipped while nothing is there, such as `package.json` before a TypeScript project exists).

- **More detectors** catch more dead code, and each adds its run time to every `clm check` and needs its tool installed wherever checks run.
- **None** keeps checks fast and leaves dead code to review.

Where: `clm.toml` or `clm.local.toml`.

### `affected`

`[affected]` with `all`: globs of changed files that make `clm affected` select every test, such as a lockfile or the toolchain pin. A change to a file a component owns otherwise selects only the tests of the components it touches.

- **Add a glob** for a file every component's code depends on but none owns, so a change to it runs every test.
- **Each glob** makes more changes run the whole suite.

Where: `clm.toml` or `clm.local.toml`.

## Sessions on this machine

### `max_builds`

How many builds and drafts may run on this clone at once (default 3).

- **Higher** builds more components in parallel, at the cost of CPU, memory, disk and model usage at once.
- **Lower** suits a small machine or a tight usage budget.

Where: any file, or `CLM_MAX_BUILDS`. `clm.local.toml` or your own file suit it best, since it describes a machine.

### `autosync`

Whether spec work in progress is pushed to `autosync/<branch>` after each turn (default `true`), so it survives a lost session and other sessions see what's being proposed.

- **`true`** costs a push after each turn.
- **`false`** keeps work local until you push it.

Where: any file, or `CLM_AUTOSYNC`.

### `writer`

The name your approvals, checks and lands are recorded under. Defaults to the repository's git `user.name`, as a slug.

- **Set it** when your git name differs between machines, so your records stay under one name.

Where: your own file, `clm.local.toml` or `CLM_WRITER`; never `clm.toml`, which everyone shares.

## Thresholds

### `component_size`

`[component_size]` with `spec_tokens` (default 12000) and `code_tokens` (default 40000): the size above which a component with no `size_budget` of its own is oversized ([Shaping the graph](shaping-the-graph.md)).

- **Higher** flags fewer components, and lets each grow past what one build session holds well.
- **Lower** flags components for splitting sooner.

An oversized component still builds; the flag only says it's worth splitting. Where: `clm.toml` or `clm.local.toml`.

### `health`

`[health]` with `reconsider` (default 3), `scenarios` (6), `exceptions` (4), `stale_days` (30) and `lcov` (a path from the repository's root): the score at which `clm rebase` files a question to reconsider a requirement; the scenario and exception counts above which `clm health` flags one; the days after which an open question or human task is reported; and the coverage file `clm health` reads to find code no test runs.

- **Lower thresholds** surface more to reconsider, and more questions to answer.
- **Higher thresholds** stay quiet longer.
- **Set `lcov`** when your tests write an lcov file, and `clm health` reports the code they don't exercise.

Where: `clm.toml` or `clm.local.toml`.

### `effort`

`[effort]` with `workaround_flag` (default 3): the unsettled workarounds on one capability at which `clm workaround` stops a build to ask whether to redesign it.

- **Lower** asks sooner, before workarounds pile up.
- **Higher** lets builds push on longer before they stop to ask.

Where: `clm.toml` or `clm.local.toml`.

## Upgrades

These record where the project stands with CLM's migrations. `/clm:upgrade` and `clm upgrade` set them; don't edit them by hand.

### `upgraded`

The last migration the project took. `clm init` and `clm upgrade --apply` write it. While a required migration is pending, most commands refuse to run. Where: `clm.toml` only.

### `declined_migrations`

The advised migrations the project chose not to take (`clm upgrade --decline`). Where: `clm.toml` only.

### `skip_migrations`

Required migrations this clone runs past while they are broken; report the migration with `clm defect` first. Where: `clm.local.toml` only, so a skip never reaches another clone.
