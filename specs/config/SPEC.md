---
title: Config
status: active
owners: ["@clunny"]
depends_on: []
paths: ["crates/clm-config/**", "clm.toml", ".gitignore"]
successor: ""
serves: [capability/configure-clm]
---

## Contract
CLM's settings (`layered-settings`, `local-settings`). Each key's value comes from the first layer that sets it:
1. its environment variable;
2. the clone's `clm.local.toml`: the file of that name in the CLM repository's directory in the clone's main worktree, whichever worktree of the clone `clm` runs in;
3. the repository's `clm.toml`, at its root;
4. the user's `${XDG_CONFIG_HOME:-$HOME/.config}/clm/config.toml`;
5. its default.

`clm.local.toml` belongs to one clone on one machine and is never committed (`clm init` lists it in `.gitignore`), so it may set every key below, `writer` and the repository's keys included. The *main worktree* is the first worktree `git worktree list --porcelain` lists; a linked worktree never holds an untracked file of the main one, so one `clm.local.toml` serves every worktree of the clone. The *CLM repository's directory* in it is the repository root's path relative to the top of its own worktree, taken in the main worktree, so a CLM repository in a subdirectory such as `tool/` reads `<main>/tool/clm.local.toml`.

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

A table (`[component_size]`, `[health]`, `[effort]`, `[land]`, `[affected]`, `[[detectors]]`) is one key: the first layer that sets the table supplies all of it, and layers never merge within a table, so a `clm.local.toml` with `[component_size] code_tokens = 60000` gives `spec_tokens` its default, not `clm.toml`'s value.

Loading the settings for a repository root (or none) gives each key's value and the layer it came from, or an error naming the file or variable and the key; an error in a file names the file that set the value, `clm.local.toml` or `clm.toml`. `clm config get <key>` prints them, naming the file the value came from.

## Design
Crate `clm-config`. Files are TOML, read with the `toml` crate into a struct that denies unknown fields. The user's file is read into a struct without the repository-only keys, the repository's into one without `writer`, and `clm.local.toml` into one with every key, so setting a key in the wrong file is an unknown-key error naming that file and the key. A missing file is skipped, a file that can't be parsed is an error naming it. A variable is read only when set: `CLM_AUTOSYNC` accepts `0`, `false`, `1` and `true` (any other value is an error, which leaves autosync off for that turn), `CLM_DEFECTS` the key's values, `CLM_WRITER` a slug (lowercase letters, digits and `-`), and `CLM_MAX_BUILDS` a whole number of at least `1` written in decimal digits. The environment and `XDG_CONFIG_HOME`/`HOME` are passed in, not read globally, so tests don't touch the process environment. Tests of commands and hooks that load settings set `XDG_CONFIG_HOME` to an empty directory, so the developer's own file never leaks in.

**Finding `clm.local.toml`** (`local-settings`). The crate runs no git, so it finds the main worktree from the files git keeps. From the repository root it walks up to the first directory holding `.git`, the top of the root's worktree. When `.git` is a directory, that top is the main worktree. When it is a file, the root is in a linked worktree: the file's `gitdir: <dir>` line names the worktree's directory under the main repository's git directory, whose `commondir` file names that git directory (relative to `<dir>`, usually `../..`), and the main worktree is the git directory's parent. The local file is `<main>/<root relative to its worktree's top>/clm.local.toml`. A root with no `.git` above it, or whose `gitdir` or `commondir` can't be read, reads `<root>/clm.local.toml`. A common git directory that isn't named `.git`, as a bare repository's is, has no main worktree, and then no local layer is read, from whichever worktree. `clm config get` names the file by that path, so a linked worktree's output names the main worktree's file.

`max_builds` is a TOML integer of at least `1`; a float or a string fails like `0` (`max-builds-setting`). It is read by the commands that count places (`orchestrator`'s `clm next`, `orchestrator.run`'s `clm run`, `orchestrator.triage`'s `clm draft`), each from the settings `crates/clm` or its own `load` gives it.

`writer`'s default is computed by the caller that needs it (`ledger`), from `git config user.name` of the repository, lowercased with runs of anything but letters and digits replaced by `-`; `clm-config` reports the key as unset with layer `default` and leaves the lookup to `ledger`, so the crate runs no git. `clm config get writer` prints the value when a layer sets it, or `(git user.name)` and `default`.

The repository root is whoever calls it decides: `clm config` uses the CLM repository found from the current directory as `plugin`'s `session-start` does, and none outside one.

`clm config get <key>` prints `<value>` and the layer, tab-separated: `env CLM_DEFECTS`, the file's path, or `default`. It exits 1 with the error on an unknown key or when the settings can't be loaded. For `detectors` it prints the detectors' names, comma-separated, or nothing. For `land` it prints `check=<command>` or nothing. For `affected` it prints `all=<glob>,<glob>`, or nothing. For `health` it prints `reconsider=<n>,scenarios=<n>,exceptions=<n>,stale_days=<n>`, followed by `,lcov=<path>` when it is set. For `effort` it prints `workaround_flag=<n>`.

This repository's `clm.toml` sets `defects = "backlog"`, `host = "github"`, `[land] check = ".github/scripts/ci-check.sh"`, `[affected] all = ["Cargo.toml", "Cargo.lock", "rust-toolchain.toml", "crates/clm/src/**", ".github/scripts/ci-check.sh"]` (the workspace's manifest and lockfile and the toolchain pin, which every crate builds with; the CLI's dispatch, which every test in `crates/clm/tests/` runs through, though each cites only the components it tests; and the check script, which does the selecting), `[health] lcov = "target/lcov.info"` (the tracefile `.github/scripts/coverage.sh` writes, `ci/coverage-script`, which `clm health`'s `unexercised` signal reads, `lcov-coverage-signal`; `health-lcov-here`) and two detectors: `unused-dependencies`, whose command is `["cargo-machete"]` (run directly, not as `cargo machete`: under `cargo run` or `cargo test` it inherits `CARGO_PKG_NAME` and reads `machete` as a path) with the hint `cargo install --locked --version <x.y.z> cargo-machete`, and `unreferenced-public-items`, whose command is `clm unreferenced` with this repository's allow-list (`public-item-detector`, `unreferenced-allow-then-narrow`) and whose hint is `scripts/dev-install.sh` (`dogfood-from-main`). Its `.gitignore` lists `clm.local.toml`, `.clm/` and `.claude/worktrees/`, as `clm init` writes them in other repositories (`init-ignores-session-files`), each as an exact line with no leading `/` (`local-settings-ignored`).

**The allow-list** (`unreferenced-detector`, `unreferenced-allow-then-narrow`). `unreferenced-public-items`'s command is a multi-line TOML array: the line `"clm", "unreferenced",`, then one `"--allow", "<entry>",` line per exception, in groups, each group under one or more `#` comment lines that give its reason, and nothing else. An entry names one item, `<crate>::<name>` or `<crate>::<Type>::<name>`, never a whole library, so a new unused `pub` item in any crate fails `clm check`. There are two reasons. *Kept*: the item is live by a route the name match can't see, and the comment says which. A type named in the signature of a public item other crates use needs no entry, since `check.unreferenced` counts that use (`unreferenced-signature-carriers`); nor does an item an exported macro's expansion names, since `check.unreferenced` counts that use (`unreferenced-carriers`), and a framework entry point the detector doesn't know, such as `#[event(fetch)]`, gets no `--allow` entry either: the attribute is named once with an `"--entry", "<attribute>",` line, under a comment saying which framework calls it, between the two command words and the first `--allow` pair (`unreferenced-entry-attributes`). *Pending*: the item waits for its component's clean-up, `backlog/narrow-public-items-<component>` (the component's ID with `.` replaced by `-`), and the comment names that item. Each crate's entries form their own groups, a pending group per component and a kept group per reason within it, separated by their comment lines, so builds of different components edit lines far enough apart for git to merge them. A stale entry is itself a finding (`check.unreferenced/allow-list`), so the list only shrinks: the build that narrows an item deletes its entry in the same diff, editing this file from that component's `impl/*` branch, as builds edit the root's dispatch files (each item names `config` among its `components` for that reason), and a build that finds a pending entry is in fact kept moves it under a kept comment.

A `check.unreferenced` change that stops reporting a kept item can't simply delete its entry in the same branch: `clm land` runs the tests with the branch's `clm`, which finds the entry stale, and then `clm check` with the installed `clm`, which needs it. Such a change orders the two steps as `settings-before-use` does for a key: the detector change lands first, a `human-task` item asks for `clm` to be reinstalled from that `main`, and then a branch deletes the entries.

The detector runs whichever `clm` is first on `PATH`. In `clm land` and in a session that is the clone's installed `clm`, built from `main` (`dogfood-from-main`), which has `clm unreferenced`; in `cargo test`'s `this_repository_is_clean` it is the binary under test (`clm/clean-check-runs-own-clm`), so the release workflow's `prepare` job, which installs no `clm`, runs it too. `.github/scripts/detector-tools.sh` installs nothing for it, and `ci/detector-tools` exempts a detector whose command is `clm`.

**A key this repository sets lands before its value** (`settings-before-use`). Every `clm` denies the keys it doesn't know, and the `clm` that runs `clm rebase`, `clm check` and `clm land` on a branch is the installed one, built from an earlier `main`, not the branch's. So a branch that both teaches this crate a key and sets it in this repository's `clm.toml` can't sync or land, and once on `main` it breaks the `clm` of any clone not yet rebuilt. The same holds for a new field of a table key and a new value of a key, when this repository sets them. A change that adds one of these and has this repository's `clm.toml` set it therefore gives two requirements of this component: one that parses it, with no scenario reading this repository's `clm.toml`, and one that sets it here. The build of this component that makes the parsing requirement current leaves the second out of its work and files, in its own branch, a new `human-task` backlog item `backlog/install-clm-for-<key>.md` (`kind: human-task`, `components: [config]`, `blocks: [config/<second-requirement>]`, `origin: <change-id>`), whose body says to install `clm` from a `main` that holds the parser (`scripts/dev-install.sh`, or a merging pull with its hook, `dogfood-from-main`) in every clone the human uses, and then delete the item. The build files it, since adding an item never makes a code branch mixed (`gate/backlog-additions`). Once the human has installed, a session lands the item's deletion on a backlog-only branch, which needs no review, and the next build writes the value. Both requirements are this component's, so the wait is the item, not a `#### Needs`, which can't name a requirement of its own component (`requirement-needs`). Removing a key this repository sets goes the other way: a requirement that drops the value lands first, and the parser's removal waits on the same kind of item.

## Requirements

### Requirement: layers
Loading the settings SHALL give each key the value of its environment variable when set, otherwise of the clone's `clm.local.toml` when it sets the key, otherwise of the repository's `clm.toml` when it sets the key, otherwise of the user's `config.toml` when it sets the key, otherwise its default, together with the layer it came from. A missing file SHALL be skipped. A table key (`component_size`, `land`, `detectors`) SHALL come whole from the first layer that sets it, never merged with another layer's.

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

### Requirement: invalid-settings
Loading SHALL fail, naming the file or variable and the key, when a file can't be parsed or has an unknown key, or a file or variable sets a key to a value it doesn't accept.

#### Scenario: unknown key
- GIVEN a repository `clm.toml` with `defect = "off"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `defect`

#### Scenario: bad value
- GIVEN `CLM_DEFECTS=always`
- WHEN the settings are loaded
- THEN it fails naming `CLM_DEFECTS`

### Requirement: config-get
`clm config get <key>` SHALL print the key's value and its layer (`env <VARIABLE>`, the file's path, or `default`), tab-separated, and exit 0. It SHALL exit 1 with the error when the key is unknown or the settings can't be loaded.

#### Scenario: from the repository
- GIVEN a CLM repository whose `clm.toml` sets `defects = "backlog"`
- WHEN `clm config get defects` runs in it
- THEN it prints `backlog` and the path of `clm.toml`, and exits 0

#### Scenario: unknown key
- GIVEN any directory
- WHEN `clm config get colour` runs
- THEN it exits 1 naming `colour`

### Requirement: repository-only
Loading SHALL take `detectors` only from the repository's `clm.toml` or the clone's `clm.local.toml`, defaulting to none, and SHALL fail naming the user's `config.toml` and the key when that file sets it. A detector without a `name` or with an empty `command` SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `detectors`.

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

#### Serves
- capability/no-dead-weight
- capability/configure-clm

### Requirement: host-setting
Loading SHALL take `host` only from the repository's `clm.toml` or the clone's `clm.local.toml`, defaulting to `github`, SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `host` for a value other than `none`, `git` or `github`, and SHALL fail naming the user's `config.toml` and the key when that file sets it.

#### Scenario: default
- GIVEN a repository with no `clm.toml`
- WHEN `clm config get host` runs
- THEN it prints `github` and `default`

#### Scenario: from the repository
- GIVEN a repository `clm.toml` with `host = "git"`
- WHEN `clm config get host` runs
- THEN it prints `git` and the file's path

#### Scenario: unknown host
- GIVEN a repository `clm.toml` with `host = "gitlab"` or `host = "azure"`
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

#### Serves
- capability/git-host
- architecture/host-adapters

### Requirement: approval-setting
Loading SHALL take `approval` only from the repository's `clm.toml` or the clone's `clm.local.toml`, defaulting to `any`, SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `approval` for a value other than `any` or `other`, and SHALL fail naming the user's `config.toml` and the key when that file sets it.

#### Scenario: default
- GIVEN a repository with no `clm.toml`
- WHEN `clm config get approval` runs
- THEN it prints `any` and `default`

#### Scenario: other
- GIVEN a repository `clm.toml` with `approval = "other"`
- WHEN `clm config get approval` runs
- THEN it prints `other` and the file's path

#### Scenario: from the user's file
- GIVEN a user `config.toml` setting `approval = "any"`
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `approval`

#### Serves
- capability/approve-intent
- architecture/approval-policy

### Requirement: land-check-setting
Loading SHALL take `land` only from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table whose key `check` is a non-empty string and whose optional key `target_cap_gb` is a positive integer, defaulting to no command and a cap of 15. It SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `land` for a `land` that isn't a table, for any other key in it, for a `check` that is missing or isn't a non-empty string, or for a `target_cap_gb` that isn't a positive integer, and SHALL fail naming the user's `config.toml` and the key when that file sets it. `clm config get land` SHALL print `check=<command>`, followed by `,target_cap_gb=<n>` when the table sets it, and the path of the file that sets it, or nothing and `default`.

#### Scenario: set
- GIVEN a repository `clm.toml` with `[land]` and `check = "scripts/clm-check.sh"`
- WHEN `clm config get land` runs
- THEN it prints `check=scripts/clm-check.sh` and the file's path

#### Scenario: cap set
- GIVEN a repository `clm.toml` with `[land]`, `check = "scripts/clm-check.sh"` and `target_cap_gb = 4`
- WHEN `clm config get land` runs
- THEN it prints `check=scripts/clm-check.sh,target_cap_gb=4` and the file's path, and the loaded settings' cap is 4

#### Scenario: cap defaults
- GIVEN a repository `clm.toml` with `[land]` and only `check = "scripts/clm-check.sh"`
- WHEN the settings are loaded
- THEN the land cap is 15

#### Scenario: zero cap
- GIVEN a repository `clm.toml` with `[land]`, `check = "scripts/clm-check.sh"` and `target_cap_gb = 0`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `land`

#### Scenario: empty command
- GIVEN a repository `clm.toml` with `[land]` and `check = ""`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `land`

#### Scenario: from the user's file
- GIVEN a user `config.toml` with a `[land]` table
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `land`

#### Serves
- capability/land-safely

### Requirement: writer-setting
Loading SHALL give `writer` the value of `CLM_WRITER` when set, otherwise of the clone's `clm.local.toml` when it sets it, otherwise of the user's `config.toml` when it sets it, otherwise none with layer `default`. It SHALL fail naming `clm.toml` and `writer` when the repository's `clm.toml` sets it, and a value that isn't a slug of lowercase letters, digits and `-` SHALL fail naming the variable or file and the key.

#### Scenario: from the variable
- GIVEN `CLM_WRITER=christopher`
- WHEN the settings are loaded
- THEN `writer` is `christopher` from the variable

#### Scenario: from the user's file
- GIVEN a user `config.toml` with `writer = "alice"`
- WHEN the settings are loaded with no variables set
- THEN `writer` is `alice` from the user's file

#### Scenario: from the local file
- GIVEN a user `config.toml` with `writer = "alice"` and a `clm.local.toml` with `writer = "bob"`
- WHEN the settings are loaded with no variables set
- THEN `writer` is `bob` from `clm.local.toml`

#### Scenario: from the repository
- GIVEN a repository `clm.toml` with `writer = "christopher"`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `writer`

#### Scenario: not a slug
- GIVEN `CLM_WRITER="Christopher L"`
- WHEN the settings are loaded
- THEN it fails naming `CLM_WRITER`

#### Scenario: unset
- GIVEN no files and no variables
- WHEN `clm config get writer` runs
- THEN it prints `(git user.name)` and `default`

#### Serves
- capability/work-concurrently
- architecture/per-writer-ledgers

### Requirement: local-settings
Loading SHALL read `clm.local.toml` from the CLM repository's directory in the clone's main worktree (the first worktree `git worktree list --porcelain` lists), whichever worktree of the clone the root is in, found without running git as the Design says. It SHALL accept every key there, `writer` and the repository-only keys included. A missing `clm.local.toml` SHALL add no layer; one that can't be parsed, has an unknown key or sets a key to a value it doesn't accept SHALL fail naming `clm.local.toml` and the key, as for `clm.toml`.

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

### Requirement: max-builds-setting
Loading SHALL give `max_builds` from `CLM_MAX_BUILDS`, `clm.local.toml`, `clm.toml` or the user's `config.toml`, in the layers' order, defaulting to `3`. A value that isn't a whole number of at least `1` SHALL fail naming the variable or file and `max_builds`. `clm config get max_builds` SHALL print the number and its layer.

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

#### Serves
- capability/work-concurrently

### Requirement: local-settings-ignored
This repository's `.gitignore` SHALL list `clm.local.toml`, `.clm/` and `.claude/worktrees/`, each as a line of its own, as `clm init` writes them in other repositories (`init/init-ignores-local-settings`).

#### Scenario: ignored here
- GIVEN this repository's `.gitignore`
- WHEN a test reads it
- THEN it has the lines `clm.local.toml`, `.clm/` and `.claude/worktrees/`

### Requirement: detector-timeout-setting
Loading SHALL give each detector a `timeout` in whole seconds: the value its `[[detectors]]` table sets, else `600`. A `timeout` that isn't a whole number of at least `1` SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `detectors`.

#### Scenario: default timeout
- GIVEN a repository `clm.toml` with one detector `machete`, command `["cargo-machete"]`, and no `timeout`
- WHEN the settings are loaded
- THEN `detectors` holds `machete` with a timeout of 600 seconds

#### Scenario: timeout set
- GIVEN a repository `clm.toml` with a detector `slow`, command `["sleep", "5"]` and `timeout = 2`
- WHEN the settings are loaded
- THEN `detectors` holds `slow` with a timeout of 2 seconds

#### Scenario: zero timeout
- GIVEN a `clm.local.toml` with a detector whose `timeout` is `0`
- WHEN the settings are loaded
- THEN it fails naming `clm.local.toml` and `detectors`

#### Serves
- capability/no-dead-weight

### Requirement: affected-setting
Loading SHALL take `affected` only from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table whose one key `all`, which it must hold, is an array of non-empty strings (possibly empty), defaulting to no glob when no layer sets the table. It SHALL fail naming the file that sets it and `affected` for an `affected` that isn't a table, for a missing `all`, for any other key in it, or for an `all` that isn't an array of non-empty strings, and SHALL fail naming the user's `config.toml` and the key when that file sets it. `clm config get affected` SHALL print `all=` followed by the globs, comma-separated (just `all=` for `all = []`), with the path of the file that sets it, or nothing and `default` when no layer sets it.

#### Scenario: set
- GIVEN a repository `clm.toml` with `[affected]` and `all = ["Cargo.lock", "rust-toolchain.toml"]`
- WHEN `clm config get affected` runs
- THEN it prints `all=Cargo.lock,rust-toolchain.toml` and the file's path

#### Scenario: empty glob
- GIVEN a repository `clm.toml` with `[affected]` and `all = [""]`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `affected`

#### Scenario: from the user's file
- GIVEN a user `config.toml` with an `[affected]` table
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `affected`

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN the settings are loaded
- THEN `affected` holds `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/clm/src/**` and `.github/scripts/ci-check.sh`

#### Serves
- capability/land-safely

### Requirement: unreferenced-detector
This repository's `clm.toml` SHALL name, after `unused-dependencies`, a detector `unreferenced-public-items` whose command is `"clm"`, `"unreferenced"`, then any `"--entry", "<attribute>"` pairs (`check.unreferenced/entry-attributes`) and then only `"--allow", "<entry>"` pairs, each entry naming one item (`<crate>::<name>` or `<crate>::<Type>::<name>`), never a whole library, and whose hint is `scripts/dev-install.sh`. The array SHALL hold the two command words on its first line and one pair per line after them, and every pair SHALL come under a `#` comment line within the array that gives its reason: kept (live by a route the name match can't see, saying which) or pending (naming `backlog/narrow-public-items-<component>`, `<component>` being the item's component ID with `.` replaced by `-`). With those arguments, `clm unreferenced` SHALL exit 0 on this repository, so every entry excuses a finding and no finding is unexcused. Every `backlog/narrow-public-items-<component>` a pending comment names SHALL exist. An entry for an item of `clm-config`, this component's own crate, SHALL be kept, never pending: this component's build narrows or removes the rest of its findings. Its tests SHALL live in `crates/clm/tests/cli.rs`, beside `this_repository_is_clean`, since one runs the `clm` binary.

#### Scenario: named here
- GIVEN this repository's `clm.toml`
- WHEN the settings are loaded
- THEN `detectors` holds `unused-dependencies` and then `unreferenced-public-items`, whose command starts `clm`, `unreferenced`, whose other arguments are pairs, `--entry` and an attribute name and then `--allow` and an entry, with every `--allow` entry holding `::`, and whose hint is `scripts/dev-install.sh`

#### Scenario: every entry has a reason
- GIVEN this repository's `clm.toml`
- WHEN a test reads the `unreferenced-public-items` command's array as text
- THEN its first line holds `"clm", "unreferenced",`, every other non-blank line is a `#` comment, one `"--entry", "<attribute>",` pair or one `"--allow", "<entry>",` pair, no `"--entry"` line comes after an `"--allow"` line, the first pair's line comes after a comment line, and every `backlog/narrow-public-items-<component>` a comment names is a file under `backlog/`, and no pending comment names a `config` item

#### Scenario: clean with the list
- GIVEN this repository and the `clm` binary under test
- WHEN `clm unreferenced` runs with the arguments of the detector's command after `unreferenced`
- THEN it prints nothing and exits 0

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

#### Needs
- clm/clean-check-runs-own-clm
- ci/detector-tools
- check.unreferenced/public-items
- check.unreferenced/references
- check.unreferenced/report
- check.unreferenced/entry-attributes

### Requirement: health-setting
Loading SHALL take `health` only from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table whose keys `reconsider`, `scenarios`, `exceptions` and `stale_days` are each optional positive integers defaulting to `3`, `6`, `4` and `30`, and whose key `lcov` is an optional string naming a path relative to the repository root: its components, as the platform reads them, are at least one ordinary name and otherwise only `.`, with no root, prefix or `..` component, and the string holds neither `,`, `=` nor a control character. It SHALL fail naming the file that sets it and `health` for a `health` that isn't a table, for any other key, for a count that isn't a positive integer or an `lcov` that isn't such a path, and SHALL fail naming the user's `config.toml` and the key when that file sets it. `clm config get health` SHALL print `reconsider=<n>,scenarios=<n>,exceptions=<n>,stale_days=<n>`, followed by `,lcov=<path>` when it is set, and the layer: the path of the file that sets the table, `default` otherwise.

#### Scenario: default
- GIVEN a repository with no `clm.toml`
- WHEN `clm config get health` runs
- THEN it prints `reconsider=3,scenarios=6,exceptions=4,stale_days=30` and `default`

#### Scenario: two keys set
- GIVEN a repository `clm.toml` with `[health]`, `reconsider = 5` and `lcov = "target/lcov.info"`
- WHEN `clm config get health` runs
- THEN it prints `reconsider=5,scenarios=6,exceptions=4,stale_days=30,lcov=target/lcov.info` and the file's path

#### Scenario: not a positive integer
- GIVEN a repository `clm.toml` with `[health]` and `reconsider = 0`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `health`

#### Scenario: lcov that isn't a path under the root
- GIVEN a repository `clm.toml` with `[health]` and `lcov` set to `""`, to `"."`, to `"/tmp/lcov.info"`, to `"../other/lcov.info"`, to `"cov=a,b.info"` or to `"cov\tinfo"`
- WHEN the settings are loaded
- THEN each fails naming `clm.toml` and `health`

#### Scenario: from the user's file
- GIVEN a user `config.toml` with a `[health]` table
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `health`

#### Serves
- capability/know-health

### Requirement: effort-setting
Loading SHALL take `effort` only from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table whose one key `workaround_flag` is an optional positive integer defaulting to `3`. It SHALL fail naming the file that sets it and `effort` for an `effort` that isn't a table, for any other key, or for a `workaround_flag` that isn't a positive integer, and SHALL fail naming the user's `config.toml` and the key when that file sets it. `clm config get effort` SHALL print `workaround_flag=<n>` and the layer: the path of the file that sets the table, `default` otherwise.

#### Scenario: default
- GIVEN a repository with no `clm.toml`
- WHEN `clm config get effort` runs
- THEN it prints `workaround_flag=3` and `default`

#### Scenario: set
- GIVEN a repository `clm.toml` with `[effort]` and `workaround_flag = 2`
- WHEN `clm config get effort` runs
- THEN it prints `workaround_flag=2` and the file's path

#### Scenario: not a positive integer
- GIVEN a repository `clm.toml` with `[effort]` and `workaround_flag = 0`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml` and `effort`

#### Scenario: from the user's file
- GIVEN a user `config.toml` with an `[effort]` table
- WHEN the settings are loaded
- THEN it fails naming `config.toml` and `effort`

#### Serves
- capability/review-capability-cost
- architecture/redesign-before-workaround

### Requirement: health-lcov-here
This repository's `clm.toml` SHALL set `[health]` with `lcov = "target/lcov.info"`, the tracefile `.github/scripts/coverage.sh` writes, and no other key of the table.

#### Scenario: this repository
- GIVEN this repository's `clm.toml`
- WHEN `clm config get health` runs against it
- THEN it prints `reconsider=3,scenarios=6,exceptions=4,stale_days=30,lcov=target/lcov.info` and the file's path

#### Serves
- capability/know-health

### Requirement: component-size-setting
Loading SHALL read the table `component_size` from the repository's `clm.toml` or the clone's `clm.local.toml`, as a table whose keys `spec_tokens` and `code_tokens` are each optional and default to `12000` and `40000`, failing, naming the file and `component_size`, for a `component_size` that isn't a table, for any other key, or for a value that isn't a positive integer, and failing, naming the user's `config.toml` and the key, when that file sets it; `clm config get component_size` SHALL print `spec_tokens=<n>,code_tokens=<n>` and the layer: the path of the file that sets the table, `default` otherwise. A file that sets the table's old name, which `20261102-rename-node-to-component` kept beside it, SHALL fail, naming the file, the old name and `component_size`, and `clm config get` given the old name SHALL fail as it fails for any unknown key. `clm_config::node_size_from(text)` SHALL read `[component_size]`, and fail for a text that sets the old name. No `clm:remove-by 20261102-rename-node-to-component-contract` marker SHALL be left.

#### Scenario: the new name
- GIVEN a repository `clm.toml` with `[component_size]` and `code_tokens = 60000`
- WHEN `clm config get component_size` runs
- THEN it prints `spec_tokens=12000,code_tokens=60000` and the file's path

#### Scenario: the old name refused
- GIVEN a repository `clm.toml` that sets the old name's table with `spec_tokens = 20000`
- WHEN the settings are loaded
- THEN it fails naming `clm.toml`, the old name and `component_size`

#### Serves
- capability/know-health
- architecture/slim-components
- architecture/plain-terms

### Requirement: component-size-from-text
`clm_config::node_size_from(text)` SHALL return the `component_size` a repository `clm.toml` with that text gives when no `clm.local.toml` or environment variable sets it: its `[component_size]` table, each missing key at its default, or both defaults when the text sets no `component_size`. It SHALL read the whole text as loading reads a repository `clm.toml`, and SHALL fail, naming `clm.toml` as the error's source, wherever loading a repository `clm.toml` with that text fails (an unknown key, an invalid value of any key, text that isn't TOML), so the `web` Worker shows sizes only where `clm size` would. The crate SHALL build for `wasm32-unknown-unknown`.

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

#### Serves
- capability/know-health
- architecture/slim-components

### Requirement: detector-requires-setting
Loading SHALL give each detector an optional `requires`: the path its `[[detectors]]` table sets, relative to the repository root, or none. A `requires` that isn't a non-empty string SHALL fail naming the file that sets it (`clm.toml` or `clm.local.toml`) and `detectors`.

#### Scenario: requires set
- GIVEN a repository `clm.toml` with a detector `knip`, command `["node_modules/.bin/knip"]` and `requires = "package.json"`
- WHEN the settings load
- THEN `detectors` holds `knip` requiring `package.json`

#### Scenario: requires left out
- GIVEN a repository `clm.toml` with one detector `machete`, command `["cargo-machete"]`, and no `requires`
- WHEN the settings load
- THEN `detectors` holds `machete` requiring nothing

#### Scenario: requires empty
- GIVEN a `clm.local.toml` with a detector whose `requires` is `""`
- WHEN the settings load
- THEN it fails naming `clm.local.toml` and `detectors`

#### Serves
- capability/adopt-clm
