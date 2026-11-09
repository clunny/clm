---
component: config
base: { contract: "46051478ee5becce" }
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
| `land` | a table with `check`, a non-empty string: the shell command `clm land` runs in the tree it lands, before `clm check` (`land-locally`), and optional `target_cap_gb`, a positive integer: the size in gigabytes (10^9 bytes) above which a land removes the shared target `.clm/land` after its push (`land/land-target-cap`), and optional `check_target_seconds`, a positive integer: the time in seconds a land's check aims to stay within, past which the land reports where its time went and what selected it (`land/check-time-report`) | none: `clm check` alone; `target_cap_gb = 15`; `check_target_seconds = 120` |
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

Loading the settings for a repository root (or none) gives each key's value and the layer it came from, or an error naming the file or variable and the key; an error in a file names the file that set the value, `clm.local.toml` or `clm.toml`. `load_committed(root, text, source, env)` loads them the same way with the repository's layer taken from the text of a `clm.toml` committed somewhere, or none, instead of from the root's file, each value and error from it naming `source` (such as `origin/main:clm.toml`), for a command that judges a commit by the settings committed there (`committed-settings`). `clm config get <key>` prints one, naming the file the value came from, and `clm config list` prints every key the same way, one line each (`config-list`). `docs/guide/settings.md`, the guide's page `settings`, says what each key does, what each choice trades off and where it may be set, and has a section for every key `clm config list` prints (`settings-guide-covers-keys`, `settings-guide-page`).

## ADDED Requirements

### Requirement: committed-settings
`clm_config::load_committed(root, text, source, env)` SHALL return the settings `load(Some(root), env)` returns, except that the repository's layer SHALL be read from `text`, the contents of a `clm.toml` committed somewhere, or be absent when `text` is `None`, and never from `<root>/clm.toml`; the clone's `clm.local.toml`, the variables and the user's file SHALL be read as `load` reads them. Each value the text sets SHALL have `source`, such as `origin/main:clm.toml`, as its layer, and wherever `load` fails for a `clm.toml` holding that text (an unknown key, an invalid value, a repository-only rule broken, text that isn't TOML) it SHALL fail naming `source` and the key. It exists for a command that judges a commit by the settings committed at it rather than by the checkout it runs in (`verify/settings-from-main`).

#### Scenario: the committed text, not the working file
- GIVEN a root whose `clm.toml` sets no `[verify]`, and a text setting `[verify]` with `check = "x"` and `mode = "local"`
- WHEN `load_committed` reads them with `source` `origin/main:clm.toml`
- THEN `verify` is set with `check = "x"`, and its layer names `origin/main:clm.toml`

#### Scenario: no committed file
- GIVEN a root whose `clm.toml` sets `[verify]`, and no text
- WHEN `load_committed` reads them
- THEN `verify` is unset, with layer `default`

#### Scenario: the clone's file still wins
- GIVEN a text setting `[verify]` with `check = "x"`, and a `clm.local.toml` setting `[verify]` with `check = "y"` and `mode = "local"`
- WHEN `load_committed` reads them
- THEN `verify` has `check = "y"`, and its layer is the `clm.local.toml`

#### Scenario: an error names the commit
- GIVEN a text setting `max_builds = 0`
- WHEN `load_committed` reads it with `source` `origin/main:clm.toml`
- THEN it fails naming `origin/main:clm.toml` and `max_builds`

#### Serves
- capability/slow-tests-after-land
