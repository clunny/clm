---
component: config
base: { contract: "4db91b5ffb95c32d", design: "b04f815cbaa5061a" }
---

## MODIFIED Contract

CLM's settings (`layered-settings`, `local-settings`). Each key's value comes from the first layer that sets it:
1. its environment variable;
2. the clone's `clm.local.toml`: the file of that name in the CLM repository's directory in the clone's main worktree, whichever worktree of the clone `clm` runs in;
3. the repository's `clm.toml`, at its root;
4. the user's `${XDG_CONFIG_HOME:-$HOME/.config}/clm/config.toml`;
5. its default.

`clm.local.toml` belongs to one clone on one machine and is never committed (`clm init` lists it in `.gitignore`), so it may set every key below, `writer` and the repository's keys included, except `upgraded` and `declined_migrations`, which record the repository's state. The *main worktree* is the first worktree `git worktree list --porcelain` lists; a linked worktree never holds an untracked file of the main one, so one `clm.local.toml` serves every worktree of the clone. The *CLM repository's directory* in it is the repository root's path relative to the top of its own worktree, taken in the main worktree, so a CLM repository in a subdirectory such as `tool/` reads `<main>/tool/clm.local.toml`.

| Key | Values | Default | Variable |
|---|---|---|---|
| `autosync` | `true`, `false` | `true` | `CLM_AUTOSYNC` |
| `defects` | `ask`, `backlog`, `off` | `ask` | `CLM_DEFECTS` |
| `max_builds` | a whole number, at least `1`: how many builds and drafts may hold a place on the clone at once (`build-places`) | `3` | `CLM_MAX_BUILDS` |

One setting names the person rather than describing the repository, and only its variable, `clm.local.toml` or the user's `config.toml` may set it. The repository's `clm.toml` setting it is an error, since every clone shares that file and one committed value would make everyone write as one person:

| Key | Values | Default | Variable |
|---|---|---|---|
| `writer` | a slug: the person part of the ledger writer identity (`per-writer-ledgers`) | the repository's git `user.name`, as a slug | `CLM_WRITER` |

Some settings describe the repository rather than the user, and only the repository's `clm.toml` or the clone's `clm.local.toml` may set them. They have no variable, and the user's file setting one is an error, so nobody can weaken a repository's checks from outside its clone:

| Key | Values | Default |
|---|---|---|
| `detectors` | an array of tables `[[detectors]]`, each with `name` (string), `command` (non-empty array of strings), optional `hint` (string), optional `requires` (a path relative to the repository root: `clm check` skips the detector while nothing exists there, `detector-requires-manifest`) and optional `timeout` (a whole number of seconds, at least `1`, default `600`: how long `clm check` lets the detector run before it stops it, `detector-timeouts`) | none |
| `host` | `none`, `git`, `github`, `azure`: where the repository's remote lives and which host adapter `clm` uses for pull requests, branch deletion and issues (`host-adapters`) | `github` |
| `approval` | `any`: any writer's approval of a diff counts, the author's included; `other`: only an approval by a person other than the branch's commit authors counts (`approval-policy`) | `any` |
| `land` | a table with `check`, a non-empty string: the shell command `clm land` runs in the tree it lands, before `clm check` (`land-locally`), and optional `target_cap_gb`, a positive integer: the size in gigabytes (10^9 bytes) above which a land removes the shared target `.clm/land` after its push (`land/land-target-cap`) | none: `clm check` alone; `target_cap_gb = 15` |
| `affected` | a table with `all`, an array of globs written as in a component's `paths`: changed files that make `clm affected` select every test although a component's `paths` hold them, such as a lockfile or the toolchain pin (`graph-test-selection`) | none: no glob |
| `verify` | a table: `check`, a non-empty string, the slow-test command `clm verify` runs after the land; `mode`, `local`, `pipeline` or `import`: where the slow tests run and who records the result; optional `triggers`, an array of `change-done`, `queue-drained` and `work-done` (the last only with `mode = "local"`); optional `every`, a table with `lands` and `days`, each a positive integer; and `workflow`, a non-empty string, required with `mode = "import"` and refused otherwise: the host pipeline whose runs `clm verify` reads (`verify-after-land`) | none: slow tests stay in `[land] check` |
| `component_size` | a table with `spec_tokens` and `code_tokens`, each a positive integer: the size in tokens above which a component whose frontmatter sets no `size_budget` is oversized (`node-size-budgets`, `oversized-is-a-state`, `component-size-setting`) | `spec_tokens = 12000`, `code_tokens = 40000` |
| `health` | a table with `reconsider`, `scenarios`, `exceptions` and `stale_days`, each a positive integer, and `lcov`, a path relative to the repository root, at least one ordinary name with no root, prefix or `..` component and neither `,`, `=` nor a control character in it, optional: the score at which `clm rebase` files a reconsider item for a requirement, the scenario and exception-clause counts above which `clm health` flags one, the days after which an open `question` or `human` item is reported, and the lcov file `clm health` reads (`health-advises`, `lcov-coverage-signal`) | `reconsider = 3`, `scenarios = 6`, `exceptions = 4`, `stale_days = 30`, no `lcov` |
| `effort` | a table with `workaround_flag`, a positive integer: the count of unsettled workaround flags on one capability at which `clm workaround` warns and exits 3, telling the session to stop and ask whether to redesign it (`redesign-before-workaround`, `check.health`) | `workaround_flag = 3` |

Three settings belong to `upgrade` (`upgrade-migrations`). Each is read from one layer only, with no variable and no default: unset is unset.
- `upgraded`, a string: the ID of the last migration the repository took.
- `declined_migrations`, an array of strings: the advised migrations the repository chose not to take.
- `skip_migrations`, an array of strings: required migrations one clone runs past while they are broken.

The first two record the repository's state rather than a choice, so only the repository's `clm.toml` may set them. The third lets one clone run past a broken required migration, so only the clone's `clm.local.toml` may set it, and a skip never reaches another clone. Any other layer setting one of them is an error. `config` checks their shape and their layer, and offers `local_settings_path(root)`, the path of the clone's `clm.local.toml` as loading finds it. `upgrade` reads all three itself, as plain TOML, so a project whose other settings `config` rejects can still be told which migration fixes them.

A table (`[component_size]`, `[health]`, `[effort]`, `[land]`, `[affected]`, `[[detectors]]`) is one key: the first layer that sets the table supplies all of it, and layers never merge within a table, so a `clm.local.toml` with `[component_size] code_tokens = 60000` gives `spec_tokens` its default, not `clm.toml`'s value.

Loading the settings for a repository root (or none) gives each key's value and the layer it came from, or an error naming the file or variable and the key; an error in a file names the file that set the value, `clm.local.toml` or `clm.toml`. `clm config get <key>` prints one, naming the file the value came from, and `clm config list` prints every key the same way, one line each (`config-list`). `docs/guide/settings.md`, the guide's page `settings`, says what each key does, what each choice trades off and where it may be set, and has a section for every key `clm config list` prints (`settings-guide-covers-keys`, `settings-guide-page`).

## MODIFIED Design

Crate `clm-config`. Files are TOML, read with the `toml` crate into a struct that denies unknown fields. The user's file is read into a struct without the repository-only keys, the repository's into one without `writer`, and `clm.local.toml` into one with every key, so setting a key in the wrong file is an unknown-key error naming that file and the key. A missing file is skipped, a file that can't be parsed is an error naming it. A variable is read only when set: `CLM_AUTOSYNC` accepts `0`, `false`, `1` and `true` (any other value is an error, which leaves autosync off for that turn), `CLM_DEFECTS` the key's values, `CLM_WRITER` a slug (lowercase letters, digits and `-`), and `CLM_MAX_BUILDS` a whole number of at least `1` written in decimal digits. The environment and `XDG_CONFIG_HOME`/`HOME` are passed in, not read globally, so tests don't touch the process environment. Tests of commands and hooks that load settings set `XDG_CONFIG_HOME` to an empty directory, so the developer's own file never leaks in.

**Finding `clm.local.toml`** (`local-settings`). The crate runs no git, so it finds the main worktree from the files git keeps. From the repository root it walks up to the first directory holding `.git`, the top of the root's worktree. When `.git` is a directory, that top is the main worktree. When it is a file, the root is in a linked worktree: the file's `gitdir: <dir>` line names the worktree's directory under the main repository's git directory, whose `commondir` file names that git directory (relative to `<dir>`, usually `../..`), and the main worktree is the git directory's parent. The local file is `<main>/<root relative to its worktree's top>/clm.local.toml`. A root with no `.git` above it, or whose `gitdir` or `commondir` can't be read, reads `<root>/clm.local.toml`. A common git directory that isn't named `.git`, as a bare repository's is, has no main worktree, and then no local layer is read, from whichever worktree. `clm config get` names the file by that path, so a linked worktree's output names the main worktree's file.

`max_builds` is a TOML integer of at least `1`; a float or a string fails like `0` (`max-builds-setting`). It is read by the commands that count places (`orchestrator`'s `clm next`, `orchestrator.run`'s `clm run`, `orchestrator.triage`'s `clm draft`), each from the settings `crates/clm` or its own `load` gives it.

`writer`'s default is computed by the caller that needs it (`ledger`), from `git config user.name` of the repository, lowercased with runs of anything but letters and digits replaced by `-`; `clm-config` reports the key as unset with layer `default` and leaves the lookup to `ledger`, so the crate runs no git. `clm config get writer` prints the value when a layer sets it, or `(git user.name)` and `default`.

The repository root is whoever calls it decides: `clm config` uses the CLM repository found from the current directory as `plugin`'s `session-start` does, and none outside one.

`clm config get <key>` prints `<value>` and the layer, tab-separated: `env CLM_DEFECTS`, the file's path, or `default`. It exits 1 with the error on an unknown key or when the settings can't be loaded. For `detectors` it prints the detectors' names, comma-separated, or nothing. For `land` it prints `check=<command>` or nothing. For `affected` it prints `all=<glob>,<glob>`, or nothing. For `health` it prints `reconsider=<n>,scenarios=<n>,exceptions=<n>,stale_days=<n>`, followed by `,lcov=<path>` when it is set. For `effort` it prints `workaround_flag=<n>`.

`clm config list` loads the settings once and prints, for each key in the crate's one ordered list of keys, `KEYS` (`autosync`, `defects`, `max_builds`, `writer`, `host`, `approval`, `land`, `verify`, `affected`, `detectors`, `component_size`, `health`, `effort`, `upgraded`, `declined_migrations`, `skip_migrations`), `<key>`, then what `clm config get <key>` prints, tab-separated. `get` and `list` both read that list, so a key `get` accepts is one `list` prints. When loading fails it prints the error to stderr, nothing to stdout, and exits 1, so a session validates an edit by running it. A test of this crate reads `docs/guide/settings.md` (with a `clm:reads` marker, `doc-readers-marked`) and fails naming each key of `KEYS` that no heading `` ### `<key>` `` holds. It doesn't fail on a heading for a key `KEYS` lacks: a change that adds a key writes its section on the spec branch, which lands before the build that parses the key, as the glossary's entries do (`propose-adds-settings-sections`).

This repository's `clm.toml` sets `defects = "backlog"`, `host = "github"`, `[land] check = ".github/scripts/ci-check.sh"`, `[affected] all = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates/clm/src/**", ".github/scripts/ci-check.sh"]` (the workspace's manifest and lockfile and the toolchain pin, which every crate builds with; the CLI's dispatch, which every test in `crates/clm/tests/` runs through, though each cites only the components it tests; and the check script, which does the selecting), `[health] lcov = "target/lcov.info"` (the tracefile `.github/scripts/coverage.sh` writes, `ci/coverage-script`, which `clm health`'s `unexercised` signal reads, `lcov-coverage-signal`; `health-lcov-here`) and two detectors: `unused-dependencies`, whose command is `["cargo-machete"]` (run directly, not as `cargo machete`: under `cargo run` or `cargo test` it inherits `CARGO_PKG_NAME` and reads `machete` as a path) with the hint `cargo install --locked --version <x.y.z> cargo-machete`, and `unreferenced-public-items`, whose command is `clm unreferenced` with this repository's allow-list (`public-item-detector`, `unreferenced-allow-then-narrow`) and whose hint is `scripts/dev-install.sh` (`dogfood-from-main`). Its `.gitignore` lists `clm.local.toml`, `.clm/` and `.claude/worktrees/`, as `clm init` writes them in other repositories (`init-ignores-session-files`), each as an exact line with no leading `/` (`local-settings-ignored`).

**The allow-list** (`unreferenced-detector`, `unreferenced-allow-then-narrow`). `unreferenced-public-items`'s command is a multi-line TOML array: the line `"clm", "unreferenced",`, then one `"--allow", "<entry>",` line per exception, in groups, each group under one or more `#` comment lines that give its reason, and nothing else. An entry names one item, `<crate>::<name>` or `<crate>::<Type>::<name>`, never a whole library, so a new unused `pub` item in any crate fails `clm check`. There are two reasons. *Kept*: the item is live by a route the name match can't see, and the comment says which. A type named in the signature of a public item other crates use needs no entry, since `check.unreferenced` counts that use (`unreferenced-signature-carriers`); nor does an item an exported macro's expansion names, since `check.unreferenced` counts that use (`unreferenced-carriers`), and a framework entry point the detector doesn't know, such as `#[event(fetch)]`, gets no `--allow` entry either: the attribute is named once with an `"--entry", "<attribute>",` line, under a comment saying which framework calls it, between the two command words and the first `--allow` pair (`unreferenced-entry-attributes`). *Pending*: the item waits for its component's clean-up, `backlog/narrow-public-items-<component>` (the component's ID with `.` replaced by `-`), and the comment names that item. Each crate's entries form their own groups, a pending group per component and a kept group per reason within it, separated by their comment lines, so builds of different components edit lines far enough apart for git to merge them. A stale entry is itself a finding (`check.unreferenced/allow-list`), so the list only shrinks: the build that narrows an item deletes its entry in the same diff, editing this file from that component's `impl/*` branch, as builds edit the root's dispatch files (each item names `config` among its `components` for that reason), and a build that finds a pending entry is in fact kept moves it under a kept comment.

A `check.unreferenced` change that stops reporting a kept item can't simply delete its entry in the same branch: `clm land` runs the tests with the branch's `clm`, which finds the entry stale, and then `clm check` with the installed `clm`, which needs it. Such a change orders the two steps as `settings-before-use` does for a key: the detector change lands first, a `human-task` item asks for `clm` to be reinstalled from that `main`, and then a branch deletes the entries.

The detector runs whichever `clm` is first on `PATH`. In `clm land` and in a session that is the clone's installed `clm`, built from `main` (`dogfood-from-main`), which has `clm unreferenced`; in `cargo test`'s `this_repository_is_clean` it is the binary under test (`clm/clean-check-runs-own-clm`), so the release workflow's `prepare` job, which installs no `clm`, runs it too. `.github/scripts/detector-tools.sh` installs nothing for it, and `ci/detector-tools` exempts a detector whose command is `clm`.

**A key this repository sets lands before its value** (`settings-before-use`). Every `clm` denies the keys it doesn't know, and the `clm` that runs `clm rebase`, `clm check` and `clm land` on a branch is the installed one, built from an earlier `main`, not the branch's. So a branch that both teaches this crate a key and sets it in this repository's `clm.toml` can't sync or land, and once on `main` it breaks the `clm` of any clone not yet rebuilt. The same holds for a new field of a table key and a new value of a key, when this repository sets them. A change that adds one of these and has this repository's `clm.toml` set it therefore gives two requirements of this component: one that parses it, with no scenario reading this repository's `clm.toml`, and one that sets it here. The build of this component that makes the parsing requirement current leaves the second out of its work and files, in its own branch, a new `human-task` backlog item `backlog/install-clm-for-<key>.md` (`kind: human-task`, `components: [config]`, `blocks: [config/<second-requirement>]`, `origin: <change-id>`), whose body says to install `clm` from a `main` that holds the parser (`scripts/dev-install.sh`, or a merging pull with its hook, `dogfood-from-main`) in every clone the human uses, and then delete the item. The build files it, since adding an item never makes a code branch mixed (`gate/backlog-additions`). Once the human has installed, a session lands the item's deletion on a backlog-only branch, which needs no review, and the next build writes the value. Both requirements are this component's, so the wait is the item, not a `#### Needs`, which can't name a requirement of its own component (`requirement-needs`). Removing a key this repository sets goes the other way: a requirement that drops the value lands first, and the parser's removal waits on the same kind of item.

## ADDED Requirements

### Requirement: config-list
`clm config list` SHALL print one line per key, in the order `autosync`, `defects`, `max_builds`, `writer`, `host`, `approval`, `land`, `verify`, `affected`, `detectors`, `component_size`, `health`, `effort`, `upgraded`, `declined_migrations`, `skip_migrations`, each `<key>`, the value `clm config get <key>` prints and its layer, tab-separated, and exit 0. When the settings can't be loaded it SHALL print nothing to stdout and exit 1 with the error, naming the file or variable and the key, on stderr. Every key `clm config get` accepts SHALL be one it prints.

#### Scenario: every key
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`, a `clm.local.toml` beside it with `max_builds = 5`, and no user file or variables
- WHEN `clm config list` runs in it
- THEN it prints sixteen lines in the stated order, one is `defects`, `backlog` and the path of `clm.toml`, one is `max_builds`, `5` and the path of `clm.local.toml`, one is `autosync`, `true` and `default`, and it exits 0

#### Scenario: an invalid edit
- GIVEN a CLM repository whose `clm.local.toml` sets `max_builds = 0`
- WHEN `clm config list` runs in it
- THEN stdout is empty, stderr names `clm.local.toml` and `max_builds`, and it exits 1

#### Scenario: a key get knows
- GIVEN every key `clm config get` accepts
- WHEN a test compares them with the keys `clm config list` prints
- THEN they are the same

### Requirement: settings-guide-covers-keys
The guide's page `docs/guide/settings.md` SHALL hold, for every key `clm config list` prints, a heading `` ### `<key>` ``. A test of this component SHALL read the page and fail naming each key that has no such heading, and SHALL NOT fail on a heading naming a key `clm config list` doesn't print.

#### Scenario: this repository
- GIVEN this repository's `docs/guide/settings.md`
- WHEN the test runs
- THEN it passes

#### Scenario: a key left out
- GIVEN a page holding a heading for every key but `max_builds`
- WHEN the check runs on it
- THEN it reports `max_builds`

#### Scenario: a key not yet parsed
- GIVEN a page holding a heading for every key and one for `colour`
- WHEN the check runs on it
- THEN it reports nothing
