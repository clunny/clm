---
component: config
base: { contract: "f3ba3d830eee67a6", design: "b5c51f3335b5fba1" }
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
| `autosync` | `true`, `false` | `true` | `CLM_AUTOSYNC`, which also takes `1` for `true` and `0` for `false` |
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
| `land` | a table with `check`, a non-empty string: the shell command `clm land` runs in the tree it lands, before `clm check` (`land-locally`), and optional `target_cap_gb`, a positive integer: the size in gigabytes (10^9 bytes) above which a land removes the shared target `.clm/land` after its push (`land/land-target-cap`) and `clm verify` removes its build place `.clm/verify-build` after each run of `[verify] check` (`verify/verify-build-cap`), and optional `check_target_seconds`, a positive integer: the time in seconds a land's check aims to stay within, past which the land reports where its time went and what selected it (`land/check-time-report`) | none: `clm check` alone; `target_cap_gb = 15`; `check_target_seconds = 120` |
| `build` | a table with `setup`, a non-empty string: the shell command that makes a fresh worktree of the repository ready to run its tests, such as one installing its dependencies into it, which `clm setup` runs before a build's first test run (`tests-run-in-their-own-tree`, `build-setup-step`) | none: a build's worktree gets nothing before its tests run |
| `env` | a table whose keys are environment variable names (an ASCII letter or `_`, then ASCII letters, digits and `_`), none beginning `CLM_` and none `CI`, which CLM sets or removes itself, and whose values are strings holding no NUL, carriage return or newline: variables CLM sets in the environment of `[build] setup`, `[land] check` and `[verify] check`, each replacing the value the caller's environment holds, before the variables CLM sets for the run itself (`env-table`) | none: the commands get the caller's environment and CLM's own variables |
| `affected` | a table with `all`, an array of globs written as in a component's `paths`: changed files that make `clm affected` select every test although a component's `paths` hold them, such as a lockfile or the toolchain pin (`graph-test-selection`) | none: no glob |
| `verify` | a table: `check`, a non-empty string, the slow-test command `clm verify` runs after the land; `mode`, `local`, `pipeline` or `import`: where the slow tests run and who records the result; optional `triggers`, an array of `change-done`, `queue-drained` and `work-done` (the last only with `mode = "local"`); optional `every`, a table with `lands` and `days`, each a positive integer; `workflow`, a non-empty string, required with `mode = "import"` and refused otherwise: the host pipeline whose runs `clm verify` reads (`verify-after-land`); and optional `mutate`, the mutation pass's command, with `mutate_jobs`, the most requirements it judges at once, and `mutate_minutes`, the longest a pass runs, each a positive integer set only beside `mutate` (`mutation-after-land`) | none: slow tests stay in `[land] check` |
| `component_size` | a table with `spec_tokens` and `code_tokens`, each a positive integer: the size in tokens above which a component whose frontmatter sets no `size_budget` is oversized (`node-size-budgets`, `oversized-is-a-state`, `component-size-setting`) | `spec_tokens = 12000`, `code_tokens = 40000` |
| `health` | a table with `reconsider`, `scenarios`, `exceptions` and `stale_days`, each a positive integer, and `lcov`, a path relative to the repository root, at least one ordinary name with no root, prefix or `..` component and neither `,`, `=` nor a control character in it, optional: the score at which `clm rebase` files a reconsider item for a requirement, the scenario and exception-clause counts above which `clm health` flags one, the days after which an open `question` or `human` item is reported, and the lcov file `clm health` reads (`health-advises`, `lcov-coverage-signal`) | `reconsider = 3`, `scenarios = 6`, `exceptions = 4`, `stale_days = 30`, no `lcov` |
| `effort` | a table with `workaround_flag`, a positive integer: the count of unsettled workaround flags on one capability at which `clm workaround` tells the session to stop and ask whether to redesign it (`check.health/workaround-command`, `redesign-before-workaround`) | `workaround_flag = 3` |

Three settings belong to `upgrade` (`upgrade-migrations`). Each is read from one layer only, with no variable and no default: unset is unset.
- `upgraded`, a string: the ID of the last migration the repository took.
- `declined_migrations`, an array of strings: the advised migrations the repository chose not to take.
- `skip_migrations`, an array of strings: required migrations one clone runs past while they are broken.

The first two record the repository's state rather than a choice, so only the repository's `clm.toml` may set them. The third lets one clone run past a broken required migration, so only the clone's `clm.local.toml` may set it, and a skip never reaches another clone. Any other layer setting one of them is an error. `config` checks their shape and their layer, and offers `local_settings_path(root)`, the path of the clone's `clm.local.toml` as loading finds it, whether or not the file exists, or none for a clone that has no local layer (`local-settings-path`). `upgrade` reads all three itself, as plain TOML, so a project whose other settings `config` rejects can still be told which migration fixes them.

A table (`[component_size]`, `[health]`, `[effort]`, `[land]`, `[build]`, `[verify]`, `[affected]`, `[[detectors]]`) is one key: the first layer that sets the table supplies all of it, and layers never merge within a table, so a `clm.local.toml` with `[component_size] code_tokens = 60000` gives `spec_tokens` its default, not `clm.toml`'s value. `[env]` is the one table whose layers merge (`env-table`): its variables are the union of those `clm.toml` and `clm.local.toml` set, `clm.local.toml`'s value replacing `clm.toml`'s for a name both set, and each variable keeps the layer it came from, so one clone can change one variable without copying the rest.

`load(root, env)` loads the settings for a repository root, or for none, in which case it reads only the user's file and the variables, `env` being the lookup it reads every variable through, `XDG_CONFIG_HOME` and `HOME` included; `process_env(key)` is that lookup over the process's own environment, giving a variable whose value isn't UTF-8 as set, to a value no key accepts. Loading gives each key's value and the layer it came from, or an error naming the file or variable and the key; an error in a file names the file that set the value, `clm.local.toml` or `clm.toml`. The error holds its source (the file's path, or `<VARIABLE>=<value>`) apart from its message, which names the key and never the source, and reads `<source>: <message>` (`invalid-settings`), so a caller that knows the file by another name words the source itself. `load_committed(root, text, source, env)` loads them the same way with the repository's layer taken from the text of a `clm.toml` committed somewhere, or none, instead of from the root's file, each value and error from it naming `source` (such as `origin/main:clm.toml`), for a command that judges a commit by the settings committed there (`committed-settings`). `committed_env(root, text, source)` gives the `[env]` variables alone the same way, reading no other key of the text (`env-setting`), and `node_size_from(text)` the `component_size` a repository `clm.toml` holding `text` gives by itself, failing wherever loading that `clm.toml` fails (`component-size-setting`); the crate builds for `wasm32-unknown-unknown` without its command, so the `web` Worker reads sizes through it. `clm config get <key>` prints one, naming the file the value came from, and `clm config list` prints every key the same way, one line each (`config-list`), except that `env` takes one line per variable, each naming the file that variable came from. `docs/guide/settings.md`, the guide's page `settings`, says what each key does, what each choice trades off and where it may be set, and has a section for every key `clm config list` prints (`settings-guide-covers-keys`, `settings-guide-page`).

`clm-config` offers `clm config` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the CLM repository root found from the current directory, or none outside one, since `clm config` has no `--root`, the directory it runs in, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. It takes the directory for the shape every command's call shares and doesn't read it. `crates/clm` routes the command to it; what the root prints itself when it can't read the current directory is the root's (`clm/routes-only`).

## MODIFIED Design
Crate `clm-config`. Files are TOML, read with the `toml` crate into a struct that denies unknown fields. The user's file is read into a struct without the repository-only keys, the repository's into one without `writer`, and `clm.local.toml` into one with every key, so a key set in the wrong file is, to the parser, a key that file doesn't know. A variable is read only when set, and `CLM_DEFECTS` is read by the same spellings as the file's key, so the two layers can't disagree. The environment and `XDG_CONFIG_HOME`/`HOME` are passed in, not read globally, so a caller that loads settings in a test hands it a lookup of its own.

**Finding `clm.local.toml`** (`local-settings`). The crate runs no git, so it finds the main worktree from the files git keeps. From the repository root it walks up to the first directory holding `.git`, the top of the root's worktree. When `.git` is a directory, that top is the main worktree. When it is a file, the root is in a linked worktree: the file's `gitdir: <dir>` line names the worktree's directory under the main repository's git directory, whose `commondir` file names that git directory (relative to `<dir>`, usually `../..`), and the main worktree is the git directory's parent. The local file is `<main>/<root relative to its worktree's top>/clm.local.toml`. What loading reads when those files can't be followed, and for a common git directory that isn't named `.git`, which has no main worktree, is `local-settings`'.

`max_builds` is a TOML integer. It is read by the commands that count places (`orchestrator`'s `clm next`, `orchestrator.run`'s `clm run`, `orchestrator.triage`'s `clm draft`), each from the settings `crates/clm` or its own `load` gives it.

`writer`'s default is computed by the caller that needs it (`ledger`), from `git config user.name` of the repository, lowercased with runs of anything but letters and digits replaced by `-`; `clm-config` reports the key as unset with layer `default` and leaves the lookup to `ledger`, so the crate runs no git.

The repository root is whoever calls it decides: `clm config` is handed the CLM repository found from the current directory, as `plugin`'s `session-start` finds it.

What `clm config get` prints for each key is its requirement's. The loaded settings hold `build` as `build: Setting<Option<Build>>`, `Build { setup: String }`, beside `land`. `[env]` is held as `env: Vec<(String, Setting<String>)>`, sorted by name, each variable's `Setting` naming the file it came from, beside `build`; `land`, `verify` and `orchestrator.run` set each `(name, value)` in the command's environment (`env-table`). The repository's and the local file's tables are parsed as `BTreeMap<String, String>` and merged after both are read and checked, so an error still names the file whose table broke the rule. `committed_env` parses the text as a TOML table and takes its `env` entry alone, through the same check, for the commands that run a check on a commit's tree (`land/check-env`, `verify/verify-env`).

`clm config list` loads the settings once and reads the crate's one ordered list of keys, `KEYS`, which `get` reads too, so a key `get` accepts is one `list` prints (`config-list`). The check of `docs/guide/settings.md` (`settings-guide-covers-keys`) reads `KEYS` as well, and the test that runs it holds a `clm:reads` marker naming the page (`doc-readers-marked`). It passes a heading for a key `KEYS` lacks because a change that adds a key writes its section on the spec branch, which lands before the build that parses the key, as the glossary's entries do.

This repository's `clm.toml` sets `defects = "backlog"`, `host = "github"` and `[land] check = ".github/scripts/ci-check.sh"`, and names the detector `unused-dependencies`, whose command is `["cargo-machete"]` (run directly, not as `cargo machete`: under `cargo run` or `cargo test` it inherits `CARGO_PKG_NAME` and reads `machete` as a path) with the hint `cargo install --locked --version <x.y.z> cargo-machete`. Its other settings are each a requirement of this component: `verify-here`, the `this repository` scenario of `affected-setting`, `health-lcov-here`, `build-setup-here`, `unreferenced-detector` and `retired-terms-detector`, and `local-settings-ignored` for its `.gitignore`. `[verify] check` is the whole suite with no selection, the safety net for lands that check only where the branch and `main` overlap (`land/overlap-recheck`). `[affected] all` names the workspace's manifest and lockfile and the toolchain pin, which every crate builds with; the CLI's dispatch, which the root's tests in `crates/clm/tests/` run the binary through; and the check script, which does the selecting. `[build] setup` names each npm package's directory rather than finding them, as the check script does. A change that adds an npm package to another component therefore modifies `build-setup-here` to list its directory, and the requirement that adds the package names `config/build-setup-here` under `#### Needs`: `config`'s build lists the directory first, which the skip of a directory with no `package.json` lets land while the directory is still empty, and the package's own build adds the `package.json`, each build editing only its own `paths` (`path-ownership`).

**The allow-list** (`unreferenced-detector`, `unreferenced-allow-then-narrow`). `unreferenced-public-items`'s command is a multi-line TOML array: the line `"clm", "unreferenced",`, then one `"--allow", "<entry>",` line per exception, in groups, each group under one or more `#` comment lines that give its reason, and nothing else. An entry names one item, `<crate>::<name>` or `<crate>::<Type>::<name>`, never a whole library, so a new unused `pub` item in any crate fails `clm check`. There are two reasons. *Kept*: the item is live by a route the name match can't see, and the comment says which. A type named in the signature of a public item other crates use needs no entry, since `check.unreferenced` counts that use (`unreferenced-signature-carriers`); nor does an item an exported macro's expansion names, since `check.unreferenced` counts that use (`unreferenced-carriers`), and a framework entry point the detector doesn't know, such as `#[event(fetch)]`, gets no `--allow` entry either: the attribute is named once with an `"--entry", "<attribute>",` line, under a comment saying which framework calls it, between the two command words and the first `--allow` pair (`unreferenced-entry-attributes`). *Pending*: the item waits for its component's clean-up, `backlog/narrow-public-items-<component>` (the component's ID with `.` replaced by `-`), and the comment names that item. Each crate's entries form their own groups, a pending group per component and a kept group per reason within it, separated by their comment lines, so builds of different components edit lines far enough apart for git to merge them. A stale entry is itself a finding (`check.unreferenced/allow-list`), so the list only shrinks: the build that narrows an item deletes its entry in the same diff, editing this file from that component's `impl/*` branch, as builds edit the root's dispatch files (each item names `config` among its `components` for that reason), and a build that finds a pending entry is in fact kept moves it under a kept comment.

A `check.unreferenced` change that stops reporting a kept item can't simply delete its entry in the same branch: `clm land` runs the tests with the branch's `clm`, which finds the entry stale, and then `clm check` with the installed `clm`, which needs it. Such a change orders the two steps as `settings-before-use` does for a key: the detector change lands first, a `human-task` item asks for `clm` to be reinstalled from that `main`, and then a branch deletes the entries.

The detector runs whichever `clm` is first on `PATH`. In `clm land` and in a session that is the clone's installed `clm`, built from `main` (`dogfood-from-main`), which has `clm unreferenced`; in `cargo test`'s `this_repository_is_clean` it is the binary under test (`clm/clean-check-runs-own-clm`), so the release workflow's `prepare` job, which installs no `clm`, runs it too. `.github/scripts/detector-tools.sh` installs nothing for it (`ci/detector-tools`).

**A key this repository sets lands before its value** (`settings-before-use`). Every `clm` denies the keys it doesn't know, and the `clm` that runs `clm rebase`, `clm check` and `clm land` on a branch is the installed one, built from an earlier `main`, not the branch's. So a branch that both teaches this crate a key and sets it in this repository's `clm.toml` can't sync or land, and once on `main` it breaks the `clm` of any clone not yet rebuilt. The same holds for a new field of a table key and a new value of a key, when this repository sets them. A change that adds one of these and has this repository's `clm.toml` set it therefore gives two requirements of this component: one that parses it, with no scenario reading this repository's `clm.toml`, and one that sets it here. The build of this component that makes the parsing requirement current leaves the second out of its work and files, in its own branch, a new `human-task` backlog item `backlog/install-clm-for-<key>.md` (`kind: human-task`, `components: [config]`, `blocks: [config/<second-requirement>]`, `origin: <change-id>`), whose body says to install `clm` from a `main` that holds the parser (`scripts/dev-install.sh`, or a merging pull with its hook, `dogfood-from-main`) in every clone the human uses, and then delete the item. The build files it, since adding an item never makes a code branch mixed (`gate/backlog-additions`). Once the human has installed, a session lands the item's deletion on a backlog-only branch, which needs no review, and the next build writes the value. Both requirements are this component's, so the wait is the item, not a `#### Needs`, which can't name a requirement of its own component (`requirement-needs`). Removing a key this repository sets goes the other way: a requirement that drops the value lands first, and the parser's removal waits on the same kind of item.

## MODIFIED Requirements

### Requirement: layers <!-- base: 36207631d4d0bee8 -->
Loading the settings SHALL give each key the value of its environment variable when set, otherwise of the clone's `clm.local.toml` when it sets the key, otherwise of the repository's `clm.toml` when it sets the key, otherwise of the user's `config.toml` when it sets the key, otherwise its default, together with the layer it came from. A missing file SHALL be skipped. Every table key (`detectors`, `land`, `build`, `verify`, `affected`, `component_size`, `health`, `effort`) SHALL come whole from the first layer that sets it, never merged with another layer's, a key the table leaves out taking its default and never a lower layer's value, except `env`, whose variables merge name by name as `env-setting` says. `CLM_AUTOSYNC` SHALL accept `0` and `false` for `false`, and `1` and `true` for `true`. The user's `config.toml` SHALL be `clm/config.toml` under `XDG_CONFIG_HOME`, or under `$HOME/.config` when `XDG_CONFIG_HOME` is unset or empty, and there SHALL be no user layer when neither variable names a directory. Loading with no repository root SHALL read the user's file and the variables alone.

#### Scenario: repository over user
- GIVEN a user `config.toml` with `defects = "off"` and `autosync = false`, and a repository `clm.toml` with `defects = "backlog"`
- WHEN the settings are loaded with no variables set
- THEN `defects` is `backlog` from the repository file, and `autosync` is `false` from the user file

#### Scenario: local over repository
- GIVEN a repository `clm.toml` with `defects = "backlog"` and `host = "github"`, and a `clm.local.toml` beside it with `defects = "off"`
- WHEN the settings are loaded with no variables set
- THEN `defects` is `off` from `clm.local.toml`, and `host` is `github` from `clm.toml`

#### Scenario: variable over files
- GIVEN the same files as "repository over user", a `clm.local.toml` with `defects = "off"`, and `CLM_DEFECTS=ask`, `CLM_AUTOSYNC=0`
- WHEN the settings are loaded
- THEN `defects` is `ask` and `autosync` is `false`, both from their variables

#### Scenario: no files
- GIVEN no files and no variables
- WHEN the settings are loaded
- THEN `defects` is `ask` and `autosync` is `true`, both from defaults

#### Scenario: tables don't merge
- GIVEN a repository `clm.toml` with `[component_size]` setting `spec_tokens = 20000` and `code_tokens = 50000`, and a `clm.local.toml` with `[component_size]` setting only `code_tokens = 60000`
- WHEN `clm config get component_size` runs
- THEN it prints `spec_tokens=12000,code_tokens=60000` and the path of `clm.local.toml`

#### Scenario: autosync spellings
- GIVEN a user `config.toml` with `autosync = false`
- WHEN the settings are loaded with `CLM_AUTOSYNC` set in turn to `0`, `false`, `1` and `true`
- THEN `autosync` is `false` for the first two and `true` for the last two

#### Scenario: the user's file under HOME
- GIVEN `HOME` naming a directory that holds `.config/clm/config.toml` with `defects = "off"`, and `XDG_CONFIG_HOME` unset or set to the empty string
- WHEN the settings are loaded with no repository root
- THEN `defects` is `off` from that file

#### Scenario: no user directory
- GIVEN neither `XDG_CONFIG_HOME` nor `HOME` set
- WHEN the settings are loaded with no repository root
- THEN every key comes from its default

#### Scenario: no repository root
- GIVEN a user `config.toml` with `defects = "off"`, found through `XDG_CONFIG_HOME`
- WHEN the settings are loaded with no repository root
- THEN `defects` is `off` from the user's file

### Requirement: invalid-settings <!-- base: ec741034ad9ddb97 -->
Loading SHALL fail, naming the file or variable and the key, when a file can't be parsed or has an unknown key, or a file or variable sets a key to a value it doesn't accept. The error SHALL hold its source apart from its message: the source is the file's path as loading read it, or `<VARIABLE>=<value>` for a variable, or none when no layer is at fault, as for an unknown key given to `clm config get`; the message names the key and never the source; and the error reads `<source>: <message>`, or the message alone with no source. A variable whose value isn't UTF-8 SHALL count as set, to a value no key accepts, so the process's own environment, read through `process_env`, fails loading for it and never reads as unset.

#### Scenario: unknown key
- GIVEN a repository `clm.toml` with `defect = "off"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `defect`

#### Scenario: bad value
- GIVEN `CLM_DEFECTS=always`
- WHEN the settings are loaded
- THEN it fails naming `CLM_DEFECTS`

#### Scenario: source apart from the message
- GIVEN a repository `clm.toml` with `defect = "off"`, and separately `CLM_DEFECTS=always`
- WHEN the settings are loaded
- THEN the first error's source is the path of `clm.toml`, its message names `defect` and doesn't hold `clm.toml`, and it reads as the source, `: ` and the message; and the second error's source is `CLM_DEFECTS=always`

#### Scenario: a variable that isn't text
- GIVEN `CLM_DEFECTS` set in the process's environment to the bytes `ask` followed by a byte that isn't UTF-8
- WHEN `clm config get defects` runs
- THEN it exits 1 naming `CLM_DEFECTS`

### Requirement: config-get <!-- base: e2bb5643b2406e1c -->
`clm config get <key>` SHALL print the key's value and its layer (`env <VARIABLE>`, the file's path, or `default`), tab-separated, and exit 0. It SHALL exit 1 with the error when the key is unknown or the settings can't be loaded. Outside a CLM repository it SHALL read no `clm.toml` and no `clm.local.toml`, only the user's file and the variables.

#### Scenario: from the repository
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN `clm config get defects` runs in it
- THEN it prints `backlog` and the path of `clm.toml`, and exits 0

#### Scenario: unknown key
- GIVEN any directory
- WHEN `clm config get colour` runs
- THEN it exits 1 naming `colour`

#### Scenario: outside a repository
- GIVEN a directory that is in no CLM repository and holds a `clm.toml` with `defects = "backlog"`
- WHEN `clm config get defects` runs in it
- THEN it prints `ask` and `default`

### Requirement: repository-only <!-- base: d2cdaefd32a99f32 -->
Loading SHALL take `detectors` only from the repository's `clm.toml` or the clone's `clm.local.toml`, defaulting to none, and SHALL fail naming the user's `config.toml` and the key when that file sets it. A detector without a `name` or with an empty `command` SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `detectors`. `clm config get detectors` SHALL print the detectors' names, joined by `,`, and the path of the file that sets them, or an empty value and `default` when no layer sets any.

#### Scenario: from the repository
- GIVEN a repository `clm.toml` with one detector `machete`, command `["cargo", "machete"]`
- WHEN the settings are loaded
- THEN `detectors` holds that one detector, from `clm.toml`

#### Scenario: from the user's file
- GIVEN a user `config.toml` with a `[[detectors]]` table
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `detectors`

#### Scenario: empty command
- GIVEN a repository `clm.toml` with a detector whose `command` is `[]`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `detectors`

#### Scenario: printed by name
- GIVEN a repository `clm.toml` with the detectors `a` and `b`, in that order
- WHEN `clm config get detectors` runs
- THEN it prints `a,b` and the path of `clm.toml`

### Requirement: host-setting <!-- base: 5ef6c8859787ca0b -->
Loading SHALL take `host` only from the repository's `clm.toml` or the clone's `clm.local.toml`, defaulting to `github`, SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `host` for a value other than `none`, `git`, `github` or `azure`, and SHALL fail naming the user's `config.toml` and the key when that file sets it.

#### Scenario: default
- GIVEN a repository with no `clm.toml`
- WHEN `clm config get host` runs
- THEN it prints `github` and `default`

#### Scenario: from the repository
- GIVEN a repository `clm.toml` with `host = "git"`
- WHEN `clm config get host` runs
- THEN it prints `git` and the file's path

#### Scenario: azure
- GIVEN a repository `clm.toml` with `host = "azure"`
- WHEN `clm config get host` runs
- THEN it prints `azure` and the file's path

#### Scenario: unknown host
- GIVEN a repository `clm.toml` with `host = "gitlab"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `host`

#### Scenario: unknown host set locally
- GIVEN a repository `clm.toml` with `host = "github"` and a `clm.local.toml` with `host = "gitlab"`
- WHEN the settings are loaded
- THEN it fails naming `clm.local.toml` and `host`

#### Scenario: from the user's file
- GIVEN a user `config.toml` setting `host = "none"`
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `host`

### Requirement: local-settings <!-- base: 6ef008e66328c98a -->
Loading SHALL read `clm.local.toml` from the CLM repository's directory in the clone's main worktree (the first worktree `git worktree list --porcelain` lists), whichever worktree of the clone the root is in. It SHALL accept every key there, `writer` and the repository-only keys included, except `upgraded` and `declined_migrations` (`upgrade-settings`). A missing `clm.local.toml` SHALL add no layer; one that can't be parsed, has an unknown key or sets a key to a value it doesn't accept SHALL fail naming `clm.local.toml` and the key, as for `clm.toml`. A clone whose common git directory isn't named `.git`, as a bare repository's isn't, SHALL have no local layer, from whichever worktree. When the root has no `.git` above it, or the `.git` file above it names no git directory whose `commondir` can be read, loading SHALL read the `clm.local.toml` beside the root.

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

#### Scenario: a bare clone
- GIVEN a linked worktree of a repository whose common git directory is named `repo.git`, with a `clm.local.toml` beside the worktree's root and one beside `repo.git`, each setting `defects = "off"`
- WHEN the settings are loaded for the worktree's root
- THEN `defects` comes from `default`

#### Scenario: a worktree file git can't follow
- GIVEN a root holding `clm.local.toml` with `defects = "off"` and a `.git` file that reads `gitdir: /no/such/dir`, or that holds no `gitdir: ` line
- WHEN the settings are loaded for it
- THEN `defects` is `off` from the `clm.local.toml` beside the root

### Requirement: max-builds-setting <!-- base: 11205a64f6e5e68d -->
Loading SHALL give `max_builds` from `CLM_MAX_BUILDS`, `clm.local.toml`, `clm.toml` or the user's `config.toml`, in the layers' order, defaulting to `3`. A value that isn't a whole number of at least `1` SHALL fail naming the variable or file and `max_builds`. `CLM_MAX_BUILDS` SHALL be decimal digits and nothing else: a sign, a space or a decimal point fails as any other value does. `clm config get max_builds` SHALL print the number and its layer.

#### Scenario: default
- GIVEN no files and no variables
- WHEN `clm config get max_builds` runs
- THEN it prints `3` and `default`

#### Scenario: set for the clone
- GIVEN a `clm.local.toml` with `max_builds = 5` and a repository `clm.toml` with `max_builds = 2`
- WHEN `clm config get max_builds` runs
- THEN it prints `5` and the path of `clm.local.toml`

#### Scenario: from the variable
- GIVEN `CLM_MAX_BUILDS=1` and a `clm.local.toml` with `max_builds = 5`
- WHEN `clm config get max_builds` runs
- THEN it prints `1` and `env CLM_MAX_BUILDS`

#### Scenario: zero
- GIVEN a `clm.local.toml` with `max_builds = 0`
- WHEN the settings are loaded
- THEN it fails naming `clm.local.toml` and `max_builds`

#### Scenario: bad variable
- GIVEN `CLM_MAX_BUILDS=many`
- WHEN the settings are loaded
- THEN it fails naming `CLM_MAX_BUILDS`

#### Scenario: a sign or a space
- GIVEN `CLM_MAX_BUILDS` set in turn to `+2`, ` 2` and `1.0`
- WHEN the settings are loaded
- THEN each fails naming `CLM_MAX_BUILDS`

### Requirement: unreferenced-detector <!-- base: d9f516a5f070c5d5 -->
This repository's `clm.toml` SHALL name, after `unused-dependencies`, a detector `unreferenced-public-items` whose command is `"clm"`, `"unreferenced"`, then any `"--entry", "<attribute>"` pairs (`check.unreferenced/entry-attributes`) and then only `"--allow", "<entry>"` pairs, each entry naming one item (`<crate>::<name>` or `<crate>::<Type>::<name>`), never a whole library, and whose hint is `scripts/dev-install.sh`. The array SHALL hold the two command words on its first line and one pair per line after them, and every pair SHALL come under a `#` comment line within the array that gives its reason: kept (live by a route the name match can't see, saying which) or pending (naming `backlog/narrow-public-items-<component>`, `<component>` being the item's component ID with `.` replaced by `-`). Every `backlog/narrow-public-items-<component>` a pending comment names SHALL exist. An entry for an item of `clm-config`, this component's own crate, SHALL be kept, never pending: this component's build narrows or removes the rest of its findings.

#### Scenario: named here
- GIVEN this repository's `clm.toml`
- WHEN the settings are loaded
- THEN `detectors` holds `unused-dependencies` and then `unreferenced-public-items`, whose command starts `clm`, `unreferenced`, whose other arguments are pairs, `--entry` and an attribute name and then `--allow` and an entry, with every `--allow` entry holding `::`, and whose hint is `scripts/dev-install.sh`

#### Scenario: every entry has a reason
- GIVEN this repository's `clm.toml`
- WHEN the `unreferenced-public-items` command's array is read as text
- THEN its first line holds `"clm", "unreferenced",`, every other non-blank line is a `#` comment, one `"--entry", "<attribute>",` pair or one `"--allow", "<entry>",` pair, no `"--entry"` line comes after an `"--allow"` line, the first pair's line comes after a comment line, and every `backlog/narrow-public-items-<component>` a comment names is a file under `backlog/`, and no pending comment names a `config` item

#### Needs
- clm/clean-check-runs-own-clm
- ci/detector-tools
- check.unreferenced/public-items
- check.unreferenced/references
- check.unreferenced/report
- check.unreferenced/entry-attributes

### Requirement: health-lcov-here <!-- base: 62a2a08d029a45c3 -->
This repository's `clm.toml` SHALL set `[health]` with `lcov = ".clm/verify-build/lcov.info"`, the copy of the tracefile `.github/scripts/coverage.sh` writes that the mutation pass leaves in the place `clm verify` keeps from run to run (`ci/mutate-script`), and no other key of the table.

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN `clm config get health` runs against it
- THEN it prints `reconsider=3,scenarios=6,exceptions=4,stale_days=30,lcov=.clm/verify-build/lcov.info` and the file's path

#### Needs
- ci/coverage-script
- ci/mutate-script

### Requirement: component-size-setting <!-- base: 8f8238ab311b3063 -->
Loading SHALL read the table `component_size` from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table whose keys `spec_tokens` and `code_tokens` are each optional and default to `12000` and `40000`, failing, naming the file and `component_size`, for a `component_size` that isn't a table, for any other key, or for a value that isn't a positive integer, and failing, naming the user's `config.toml` and the key, when that file sets it; `clm config get component_size` SHALL print `spec_tokens=<n>,code_tokens=<n>` and the layer: the path of the file that sets the table, `default` otherwise. A file that sets the table's old name, which `20261102-rename-node-to-component` kept beside it, SHALL fail, naming the file, the old name and `component_size`, and `clm config get` given the old name SHALL fail as it fails for any unknown key. `node_size_from(text)` SHALL return the `component_size` a repository `clm.toml` with that text gives when no `clm.local.toml` or environment variable sets it: its `[component_size]` table, each missing key at its default, or both defaults when the text sets no `component_size`. It SHALL read the whole text as loading reads a repository `clm.toml`, and SHALL fail, naming `clm.toml` as the error's source, wherever loading a repository `clm.toml` with that text fails (an unknown key, the table's old name among them, an invalid value of any key, text that isn't TOML). The crate SHALL build for `wasm32-unknown-unknown`.

#### Scenario: the new name
- GIVEN a repository `clm.toml` with `[component_size]` and `code_tokens = 60000`
- WHEN `clm config get component_size` runs
- THEN it prints `spec_tokens=12000,code_tokens=60000` and the file's path

#### Scenario: the old name refused
- GIVEN a repository `clm.toml` that sets the old name's table with `spec_tokens = 20000`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml`, the old name and `component_size`

#### Scenario: partial table
- GIVEN the text `[component_size]` followed by `code_tokens = 60000`
- WHEN `node_size_from` reads it
- THEN it returns `spec_tokens` 12000 and `code_tokens` 60000

#### Scenario: no table
- GIVEN a `clm.toml` text that sets only `[land]`
- WHEN `node_size_from` reads it
- THEN it returns `spec_tokens` 12000 and `code_tokens` 40000

#### Scenario: invalid value
- GIVEN the text `[component_size]` followed by `spec_tokens = 0`
- WHEN `node_size_from` reads it
- THEN it fails naming `clm.toml` and `component_size`

#### Scenario: unknown key
- GIVEN a `clm.toml` text that sets a valid `[component_size]` and a key loading doesn't know
- WHEN `node_size_from` reads it
- THEN it fails naming `clm.toml` and that key, as loading that `clm.toml` does

### Requirement: upgrade-settings <!-- base: ec4b21814ecced7e -->
Loading the settings SHALL accept `upgraded`, a string, and `declined_migrations`, an array of strings, only from the repository's `clm.toml`, and `skip_migrations`, an array of strings, only from the clone's `clm.local.toml`, with no variable and no default for any. Setting one in another layer, or to a value of another type, SHALL be an error naming the file and the key. `clm config get` SHALL print an unset one as an empty value and `default`, and an array as its strings joined by `,`.

#### Scenario: where each may be set
- GIVEN `clm.toml` with `upgraded = "20261105-clm-upgrade"` and `declined_migrations = ["n"]`, and `clm.local.toml` with `skip_migrations = ["x"]`
- WHEN `clm config get` runs for each of the three
- THEN it prints each value, naming `clm.toml`, `clm.toml` and `clm.local.toml`

#### Scenario: the wrong layer
- GIVEN `clm.toml` with `skip_migrations = ["x"]`, and separately `clm.local.toml` with `upgraded = "y"`
- WHEN the settings load
- THEN each fails, naming the file and the key

### Requirement: local-settings-path <!-- base: 9aa7983504f9ef04 -->
`local_settings_path(root)` SHALL return the path of the `clm.local.toml` that loading the settings for `root` reads, whether or not the file exists, or none when loading reads no local file for `root` (`local-settings`).

#### Scenario: a linked worktree
- GIVEN a repository with a linked worktree added with `git worktree add`
- WHEN `local_settings_path` runs for the linked worktree's root
- THEN it returns the main worktree's `clm.local.toml`, the path `clm config get` names for a key set there

#### Scenario: a bare clone
- GIVEN a linked worktree of a repository whose common git directory is named `repo.git`
- WHEN `local_settings_path` runs for the worktree's root
- THEN it returns none

### Requirement: config-list <!-- base: 7bccae0d381647e0 -->
`clm config list` SHALL print one line per key, in the order `autosync`, `defects`, `max_builds`, `writer`, `host`, `approval`, `land`, `build`, `env`, `verify`, `affected`, `detectors`, `component_size`, `health`, `effort`, `upgraded`, `declined_migrations`, `skip_migrations`, each `<key>`, the value `clm config get <key>` prints and its layer, tab-separated, except that `env` SHALL take one line for each line `clm config get env` prints, each `env` followed by that line, and exit 0. When the settings can't be loaded it SHALL print nothing to stdout and exit 1 with the error, naming the file or variable and the key, on stderr. Every key `clm config get` accepts SHALL be one it prints.

#### Scenario: every key
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`, a `clm.local.toml` beside it with `max_builds = 5`, and no user file or variables
- WHEN `clm config list` runs in it
- THEN it prints eighteen lines in the stated order, one is `defects`, `backlog` and the path of `clm.toml`, one is `max_builds`, `5` and the path of `clm.local.toml`, one is `env`, an empty value and `default`, one is `autosync`, `true` and `default`, and it exits 0

#### Scenario: a variable per line
- GIVEN a CLM repository whose `clm.toml` sets `[env]` with `A = "1"` and whose `clm.local.toml` sets `[env]` with `B = "2"`
- WHEN `clm config list` runs in it
- THEN it prints nineteen lines, among them `env`, `A=1` and the path of `clm.toml`, then `env`, `B=2` and the path of `clm.local.toml`, between the `build` line and the `verify` line

#### Scenario: an invalid edit
- GIVEN a CLM repository whose `clm.local.toml` sets `max_builds = 0`
- WHEN `clm config list` runs in it
- THEN stdout is empty, stderr names `clm.local.toml` and `max_builds`, and it exits 1

#### Scenario: a key get knows
- GIVEN a repository with no settings files
- WHEN `clm config get` runs for each key `clm config list` prints, and for `colour`
- THEN each listed key's output is that key's lines of `clm config list` without the key and its tab, and `colour` fails naming it

### Requirement: settings-guide-covers-keys <!-- base: b06928f4f2d26e39 -->
The guide's page `docs/guide/settings.md` SHALL hold, for every key `clm config list` prints, a heading `` ### `<key>` ``. The check of the page SHALL report each key that has no such heading, and SHALL NOT report a heading naming a key `clm config list` doesn't print.

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

### Requirement: verify-here <!-- base: fb82756211f8af9c -->
This repository's `clm.toml` SHALL set `[verify]` with `check = ".github/scripts/ci-check.sh"`, `mode = "local"`, `triggers = ["queue-drained"]`, `every = { lands = 20 }`, `mutate = ".github/scripts/mutate.sh"`, `mutate_jobs = 6` and `mutate_minutes = 60`, and no other key of the table (`verify/verify-due`, `verify/mutation-pass`).

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN `clm config get verify` runs against it
- THEN it prints `mode=local,check=.github/scripts/ci-check.sh,triggers=queue-drained,every.lands=20,mutate=.github/scripts/mutate.sh,mutate_jobs=6,mutate_minutes=60` and the file's path

#### Needs
- ci/mutate-script

### Requirement: committed-settings <!-- base: 33f3996e031a0a74 -->
`load_committed(root, text, source, env)` SHALL return the settings `load(root, env)` returns for that root, except that the repository's layer SHALL be read from `text`, the contents of a `clm.toml` committed somewhere, or be absent when `text` is `None`, and never from `<root>/clm.toml`; the clone's `clm.local.toml`, the variables and the user's file SHALL be read as `load` reads them. Each value the text sets SHALL have `source`, such as `origin/main:clm.toml`, as its layer, and wherever `load` fails for a `clm.toml` holding that text (an unknown key, an invalid value, a repository-only rule broken, text that isn't TOML) it SHALL fail naming `source` and the key.

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

### Requirement: build-setup-here <!-- base: c69fd4cbf8fb81c5 -->
This repository's `clm.toml` SHALL set `[build]` with `setup = 'for d in plugin-panel capabilities/panel web web/site capabilities/web; do test ! -f "$d/package.json" || (cd "$d" && npm ci --ignore-scripts) || exit 1; done'`, a TOML literal string, so `clm setup` installs each of the repository's npm packages' locked dependencies into the build's own worktree, running no dependency's install script (`unattended-commands`), skipping a listed directory that holds no `package.json` and stopping at the first install that fails (`tests-run-in-their-own-tree`). Every directory that holds a tracked `package.json` SHALL be one of the setup's directories. The test that proves it SHALL hold the marker `clm:reads **/package.json` (`doc-readers-marked`), so a land that adds a `package.json` anywhere runs it.

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN `clm config get build` runs against it
- THEN it prints `setup=for d in plugin-panel capabilities/panel web web/site capabilities/web; do test ! -f "$d/package.json" || (cd "$d" && npm ci --ignore-scripts) || exit 1; done` and the file's path

#### Scenario: every package is installed
- GIVEN the files `git ls-files` lists in this repository
- WHEN the directory of each one named `package.json` is taken, and the words between `for d in ` and `; do` of the setup that `clm config get build` prints
- THEN each such directory is one of those words

### Requirement: env-setting <!-- base: 000d4bed3cdad9e7 -->
Loading SHALL take `env` only from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table of string values keyed by environment variable names, and SHALL give the variables that either file's table sets, each with the value and the layer of `clm.local.toml` when it sets that name and otherwise of `clm.toml`, in byte order of the names; with neither file setting it there SHALL be no variable. It SHALL fail naming the file that sets it and `env` for an `env` that isn't a table, a value that isn't a string or holds a NUL, a carriage return or a newline, a name that isn't an ASCII letter or `_` followed by ASCII letters, digits and `_`, or a name beginning `CLM_` or equal to `CI`, and SHALL fail naming the user's `config.toml` and `env` when that file sets it. `clm config get env` SHALL print one line per variable, `<NAME>=<value>` and the path of the file that set it, tab-separated, or an empty value and `default` when there is none. `committed_env(root, text, source)` SHALL give the variables the same way with the repository's table read from `text`, the contents of a `clm.toml` committed somewhere, or none when `text` is `None`, reading no other key of the text, so a commit whose other settings a later `clm` rejects still gives its variables; a broken `[env]` in the text SHALL fail naming `source` and `env`.

#### Scenario: local overrides one variable
- GIVEN a repository `clm.toml` with `[env]` setting `RELAY_SERVER_IMAGE = "relay:amd64"` and `RUST_LOG = "info"`, and a `clm.local.toml` with `[env]` setting only `RELAY_SERVER_IMAGE = "relay:arm64"`
- WHEN `clm config get env` runs
- THEN it prints two lines, `RELAY_SERVER_IMAGE=relay:arm64` with the path of `clm.local.toml`, then `RUST_LOG=info` with the path of `clm.toml`

#### Scenario: unset
- GIVEN a repository `clm.toml` with no `[env]` table and no `clm.local.toml`
- WHEN `clm config get env` runs
- THEN it prints an empty value and `default`

#### Scenario: a name CLM sets
- GIVEN a `clm.local.toml` with `[env]` setting `CLM_SLOT = "4"`
- WHEN the settings are loaded
- THEN it fails naming `clm.local.toml` and `env`

#### Scenario: not a string
- GIVEN a repository `clm.toml` with `[env]` setting `WORKERS = 4`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `env`

#### Scenario: from the user's file
- GIVEN a user `config.toml` with an `[env]` table
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `env`

#### Scenario: a committed table with other keys gone
- GIVEN a text setting `[env]` with `A = "1"` and a key no `clm` knows, and a `clm.local.toml` with `[env]` setting `B = "2"`
- WHEN `committed_env` reads it with `source` `abc123:clm.toml`
- THEN it gives `A=1` from `abc123:clm.toml` and `B=2` from `clm.local.toml`

### Requirement: retired-terms-detector <!-- base: 1e881e15b9bae57f -->
This repository's `clm.toml` SHALL name, after `unreferenced-public-items`, a detector `retired-terms` whose command is `["sh", "scripts/retired-terms.sh"]`, the root's scan for retired terms run with no path (`clm/retired-terms`), and whose hint says the script needs only `sh` and `grep`.

#### Scenario: named here
- GIVEN this repository's `clm.toml`
- WHEN the settings are loaded
- THEN `detectors` holds `unused-dependencies`, then `unreferenced-public-items`, then `retired-terms`, whose command is `sh`, `scripts/retired-terms.sh` and whose hint names `sh` and `grep`

#### Needs
- clm/retired-terms
- ci/detector-tools

### Requirement: commands-in-crate <!-- base: ecba065493a87664 -->
`clm-config` SHALL offer `clm config` as a call of its own that takes the command's arguments as the command line gives them, the CLM repository root `crates/clm` found from the current directory, or none outside one, the directory it runs in, which it doesn't read, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, what it prints on success to the output stream and an error to the error stream as one line `clm config: <error>`, and SHALL return the command's exit status, never ending the process itself; for arguments it can't parse it SHALL write the usage error to the error stream and return 2.

#### Scenario: a command called in a test
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN the call runs with `get defects` and two buffers, and again after `clm.toml` is changed to hold `defect = "off"`
- THEN the first returns 0 with `backlog`, a tab and the path of `clm.toml` in the output buffer and nothing in the error buffer, and the second returns 1 with nothing in the output buffer and one line starting `clm config: ` in the error buffer

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is given an option `clm config` doesn't have
- THEN it returns 2 with the usage error in the error buffer and nothing in the output buffer

## REMOVED Requirements

### Requirement: component-size-from-text <!-- base: 37254aaed238ea03 -->
