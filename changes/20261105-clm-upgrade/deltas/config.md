---
component: config
base: { contract: "4d860914dbe97653" }
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
| `host` | `none`, `git`, `github`: where the repository's remote lives and which host adapter `clm` uses for pull requests, branch deletion and issues (`host-adapters`) | `github` |
| `approval` | `any`: any writer's approval of a diff counts, the author's included; `other`: only an approval by a person other than the branch's commit authors counts (`approval-policy`) | `any` |
| `land` | a table with `check`, a non-empty string: the shell command `clm land` runs in the tree it lands, before `clm check` (`land-locally`), and optional `target_cap_gb`, a positive integer: the size in gigabytes (10^9 bytes) above which a land removes the shared target `.clm/land` after its push (`land/land-target-cap`) | none: `clm check` alone; `target_cap_gb = 15` |
| `affected` | a table with `all`, an array of globs written as in a component's `paths`: changed files that make `clm affected` select every test although a component's `paths` hold them, such as a lockfile or the toolchain pin (`graph-test-selection`) | none: no glob |
| `component_size` | a table with `spec_tokens` and `code_tokens`, each a positive integer: the size in tokens above which a component whose frontmatter sets no `size_budget` is oversized (`node-size-budgets`, `oversized-is-a-state`, `component-size-setting`) | `spec_tokens = 12000`, `code_tokens = 40000` |
| `health` | a table with `reconsider`, `scenarios`, `exceptions` and `stale_days`, each a positive integer, and `lcov`, a path relative to the repository root, at least one ordinary name with no root, prefix or `..` component and neither `,`, `=` nor a control character in it, optional: the score at which `clm rebase` files a reconsider item for a requirement, the scenario and exception-clause counts above which `clm health` flags one, the days after which an open `question` or `human` item is reported, and the lcov file `clm health` reads (`health-advises`, `lcov-coverage-signal`) | `reconsider = 3`, `scenarios = 6`, `exceptions = 4`, `stale_days = 30`, no `lcov` |
| `effort` | a table with `workaround_flag`, a positive integer: the count of unsettled workaround flags on one capability at which `clm workaround` warns and exits 3, telling the session to stop and ask whether to redesign it (`redesign-before-workaround`, `check.health`) | `workaround_flag = 3` |

Three settings belong to `upgrade` (`upgrade-migrations`). Each is read from one layer only, with no variable and no default: unset is unset.
- `upgraded`, a string: the ID of the last migration the repository took.
- `declined_migrations`, an array of strings: the advised migrations the repository chose not to take.
- `skip_migrations`, an array of strings: required migrations one clone runs past while they are broken.

The first two record the repository's state rather than a choice, so only the repository's `clm.toml` may set them. The third lets one clone run past a broken required migration, so only the clone's `clm.local.toml` may set it, and a skip never reaches another clone. Any other layer setting one of them is an error. `config` checks their shape and their layer, and offers `local_settings_path(root)`, the path of the clone's `clm.local.toml` as loading finds it. `upgrade` reads all three itself, as plain TOML, so a project whose other settings `config` rejects can still be told which migration fixes them.

A table (`[component_size]`, `[health]`, `[effort]`, `[land]`, `[affected]`, `[[detectors]]`) is one key: the first layer that sets the table supplies all of it, and layers never merge within a table, so a `clm.local.toml` with `[component_size] code_tokens = 60000` gives `spec_tokens` its default, not `clm.toml`'s value.

Loading the settings for a repository root (or none) gives each key's value and the layer it came from, or an error naming the file or variable and the key; an error in a file names the file that set the value, `clm.local.toml` or `clm.toml`. `clm config get <key>` prints them, naming the file the value came from.

## MODIFIED Requirements

### Requirement: local-settings <!-- base: 482dafad79e65c7a -->
Loading SHALL read `clm.local.toml` from the CLM repository's directory in the clone's main worktree (the first worktree `git worktree list --porcelain` lists), whichever worktree of the clone the root is in, found without running git as the Design says. It SHALL accept every key there, `writer` and the repository-only keys included, except `upgraded` and `declined_migrations` (`upgrade-settings`). A missing `clm.local.toml` SHALL add no layer; one that can't be parsed, has an unknown key or sets a key to a value it doesn't accept SHALL fail naming `clm.local.toml` and the key, as for `clm.toml`.

#### Scenario: read from a linked worktree
- GIVEN a repository whose main worktree holds `clm.local.toml` with `defects = "off"`, and a linked worktree of it added with `git worktree add`
- WHEN `clm config get defects` runs in the linked worktree
- THEN it prints `off` and the path of the main worktree's `clm.local.toml`

#### Scenario: repository-only key set locally
- GIVEN a repository `clm.toml` with `[land] check = "scripts/clm-check.sh"`, and a `clm.local.toml` with `[land] check = "scripts/quick.sh"`
- WHEN `clm config get land` runs
- THEN it prints `check=scripts/quick.sh` and the path of `clm.local.toml`

#### Scenario: invalid local file
- GIVEN a `clm.local.toml` with `defect = "off"`
- WHEN the settings are loaded
- THEN it fails naming `clm.local.toml` and `defect`

#### Scenario: no local file
- GIVEN a repository `clm.toml` with `defects = "backlog"` and no `clm.local.toml`
- WHEN `clm config get defects` runs
- THEN it prints `backlog` and the path of `clm.toml`

## ADDED Requirements

### Requirement: upgrade-settings
Loading the settings SHALL accept `upgraded`, a string, and `declined_migrations`, an array of strings, only from the repository's `clm.toml`, and `skip_migrations`, an array of strings, only from the clone's `clm.local.toml`, with no variable and no default for any. Setting one in another layer, or to a value of another type, SHALL be an error naming the file and the key. `clm config get` SHALL print an unset one as it prints any key with no default.

#### Scenario: where each may be set
- GIVEN `clm.toml` with `upgraded = "20261105-clm-upgrade"` and `declined_migrations = ["n"]`, and `clm.local.toml` with `skip_migrations = ["x"]`
- WHEN `clm config get` runs for each of the three
- THEN it prints each value, naming `clm.toml`, `clm.toml` and `clm.local.toml`

#### Scenario: the wrong layer
- GIVEN `clm.toml` with `skip_migrations = ["x"]`, and separately `clm.local.toml` with `upgraded = "y"`
- WHEN the settings load
- THEN each fails, naming the file and the key

#### Serves
- capability/upgrade-clm

### Requirement: local-settings-path
`clm-config` SHALL offer `local_settings_path(root)`, the path of the `clm.local.toml` that loading the settings for `root` reads, found by the same function, whether or not the file exists.

#### Scenario: a linked worktree
- GIVEN a repository with a linked worktree added with `git worktree add`
- WHEN `local_settings_path` runs for the linked worktree's root
- THEN it returns the main worktree's `clm.local.toml`, the path `clm config get` names for a key set there

#### Serves
- capability/upgrade-clm
