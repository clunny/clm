---
component: clm
---

## MODIFIED Requirements

### Requirement: session-commands <!-- base: def407a1faad2638 -->
`.claude/settings.json`'s `permissions.allow` SHALL allow, as Bash commands, only `cargo`, `npm ci`, `npm install --ignore-scripts`, `npm run`, `npm test` and `npx playwright test`, each with any arguments, `.github/scripts/ci-check.sh` both bare and with any arguments, as `Bash(.github/scripts/ci-check.sh)` and `Bash(.github/scripts/ci-check.sh *)`, `Bash(clm setup)` and `Bash(clm setup *)`, `Bash(clm build)` and `Bash(clm build *)`, and the rules for unattended `clm` commands that `clm init` seeds (`init/init-seeds-permissions`), each of them. It names the two `clm setup` rules and the two `clm build` rules itself because `init/init-seeds-permissions` adds them to `clm init`'s list and needs this file to allow them first, so neither build leaves the other's test failing; once the list holds them they are among its rules. It SHALL NOT allow `npm install` without `--ignore-scripts`, any other `npx` command, `rustup`, `git` or `clm approve`.

#### Scenario: allowed commands
- GIVEN the repository
- WHEN a test reads `.claude/settings.json`
- THEN every `permissions.allow` entry is one of those commands or one of `clm init`'s unattended rules, and each of them is allowed, `Bash(clm setup)`, `Bash(clm setup *)`, `Bash(clm build)` and `Bash(clm build *)` among them

#### Scenario: bare npx refused
- GIVEN a settings file whose `permissions.allow` holds `Bash(npx *)`
- WHEN the test's check runs on it
- THEN it reports that entry

#### Scenario: a rule for approve refused
- GIVEN a settings file whose `permissions.allow` holds `Bash(clm approve *)`
- WHEN the test's check runs on it
- THEN it reports that entry

#### Serves
- capability/guard-sessions
- capability/build-from-spec

### Requirement: dev-install <!-- base: 97cbd5cbe639d0fe -->
`scripts/dev-install.sh`, run with no arguments from any worktree of a clone of CLM's repository, SHALL set the clone up so that it and its worktrees, and no other project on the machine, run `main`'s `clm` and the checkout's plugin (`dev-install-scoped`), in this order, and print each step it took. `<common>` is the clone's common git directory, as an absolute path.
1. It SHALL exit 1 with the reason, having changed nothing, when `jq`, `cargo` or `claude` is not on `PATH`, when the main worktree has no `.claude-plugin/marketplace.json` whose `name` is `clm`, or one listing a plugin whose `source` isn't a string starting `./`, when the main worktree's branch is not `main`, when its `.claude/settings.local.json` exists and isn't empty or one JSON object whose `env`, `enabledPlugins` and `extraKnownMarketplaces`, where present, are objects (naming the file), when `claude plugin marketplace list --json` fails or prints something other than a JSON array, or when a `reference-transaction` hook exists at `git rev-parse --git-path hooks/reference-transaction` that holds no line `# clm dev-install` and `--force` was not given.
2. It SHALL run `cargo install --locked --path crates/clm --root <common>/clm-dev --target-dir target/install` in the main worktree, and exit 1 with cargo's status message, changing nothing else, when it fails.
3. It SHALL write the `reference-transaction` hook at that path, executable, holding the line `# clm dev-install`, replacing any hook there. The hook SHALL exit 0, running nothing, unless its first argument is `committed` and a line of its standard input ends with ` refs/remotes/origin/main`; then it SHALL run the main worktree's `scripts/dev-install.sh --rebuild`. It SHALL delete the hook at `git rev-parse --git-path hooks/post-merge` when that holds `# clm dev-install`, and leave any other `post-merge` hook.
4. It SHALL write `<main worktree>/.clm/dev-marketplace.json`, the main worktree's `.claude-plugin/marketplace.json` with `name` set to `clm-dev`, creating `.clm/` when missing.
5. When `claude plugin marketplace list --json` holds an entry whose `name` is `clm`, `source` is `directory` and `path`, with symlinks resolved, is the main worktree's, it SHALL run `claude plugin marketplace remove clm` in `/`, outside every project, since Claude Code's removal also empties the marketplace's `enabledPlugins` and `extraKnownMarketplaces` entries in the settings of the project it runs in, the tracked `.claude/settings.json` among them, and then `claude plugin marketplace add clunny/clm`, and print that each project enabling the plugin installs it again at its next session.
6. It SHALL run, in the main worktree, `claude plugin marketplace add <main worktree>/.clm/dev-marketplace.json`, which also re-points a `clm-dev` record an earlier setup made, and, for each plugin `<name>` the marketplace lists, `claude plugin install <name>@clm-dev --scope local`, exiting 1 with the failing command when one fails. Then it SHALL delete `<common>/clm-dev/marketplace`, which earlier setups wrote, when it exists, and run `claude plugin list --json` in the main worktree, exiting 1 when that fails or prints something other than a JSON array, and exiting 1 naming each such plugin and printing its `errors` when an entry whose `id` is `<name>@clm-dev` for a plugin the marketplace lists and whose `projectPath` is the main worktree holds a non-empty `errors` array. Each of these exits leaves the settings file as it was; running the script again finishes the setup.
7. It SHALL set, in the main worktree's `.claude/settings.local.json`, `env.CLM_BIN` to `<common>/clm-dev/bin/clm` and, for each such plugin, `enabledPlugins."<name>@clm-dev"` to true and `enabledPlugins."<name>@clm"` to false, and delete `extraKnownMarketplaces.clm` (and `extraKnownMarketplaces` when that leaves it empty), creating the file and `.claude/` when missing and keeping every other key as it was.
8. When `${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/.crates.toml` records a package `clm` installed from a `path+file:` source, it SHALL run `cargo uninstall clm`. When a `clm` is still on `PATH` that isn't a plugin's shim (with `<dir>` the directory holding it, `<dir>/../.claude-plugin/plugin.json` doesn't exist), it SHALL print its path, and that it runs ahead of the plugin's shim in every project.
9. It SHALL print the installed binary's `--version`, say that the next session of this clone and its worktrees loads the plugin from the checkout, that the binary is rebuilt from `origin/main` whenever it moves, that sessions already running should be restarted, and that a plain `clm` outside a session doesn't find this build.
Run again on a clone already set up, it SHALL leave the settings file's content unchanged and the hook as it was. It SHALL leave every file git tracks in the clone as it was.

#### Scenario: fresh clone
- GIVEN a clone on `main` whose `.claude-plugin/marketplace.json` is named `clm` and lists a plugin `clm` with source `./plugin`, no hooks, no `.claude/settings.local.json`, and stub `cargo` and `claude` and real `jq` on `PATH`, the stub `claude` printing `[]` for `plugin list --json`
- WHEN `scripts/dev-install.sh` runs from a linked worktree of it
- THEN the stub `cargo` records `install --locked --path crates/clm --root <common>/clm-dev --target-dir target/install` run in the main worktree, the `reference-transaction` hook exists, is executable and holds `# clm dev-install`, the main worktree's `.clm/dev-marketplace.json` is the tracked manifest named `clm-dev`, `<common>/clm-dev/marketplace` doesn't exist, the stub `claude` records `plugin marketplace add <main worktree>/.clm/dev-marketplace.json`, then `plugin install clm@clm-dev --scope local` and then `plugin list --json`, run in the main worktree, and the main worktree's settings file sets `env.CLM_BIN` to `<common>/clm-dev/bin/clm`, `clm@clm-dev` to true and `clm@clm` to false

#### Scenario: an earlier symlinked marketplace replaced
- GIVEN a clone set up by an earlier script, with `<common>/clm-dev/marketplace` holding a manifest named `clm-dev` and a symlink `plugin` to the main worktree's `plugin`
- WHEN the script runs
- THEN `<common>/clm-dev/marketplace` is gone, `<common>/clm-dev/bin/clm` is still there, and the stub `claude` records `plugin marketplace add <main worktree>/.clm/dev-marketplace.json` and no `plugin marketplace remove`

#### Scenario: a plugin that fails to load stops it
- GIVEN a stub `claude` whose `plugin list --json` prints an entry with `id` `clm@clm-dev`, `projectPath` the main worktree and `errors` holding `Path escapes plugin directory`, first with no settings file and then with one holding `permissions`; and in turn: the same entry with `projectPath` another directory; the same entry with `errors` empty
- WHEN the script runs
- THEN the first two runs exit 1, naming `clm@clm-dev` and printing `Path escapes plugin directory`, the first writing no settings file and the second leaving it as it was; the other two exit 0

#### Scenario: nothing outside the clone holds the build
- GIVEN the fresh clone above, with `$CARGO_HOME/.crates.toml` recording no `clm`
- WHEN the script runs
- THEN nothing is written under `$CARGO_HOME/bin`, and the stub `claude` records no `plugin marketplace remove` and no `--scope user`

#### Scenario: an earlier setup moved
- GIVEN a settings file holding `env.CLM_BIN` at `$CARGO_HOME/bin/clm` and `extraKnownMarketplaces.clm` as a directory source at the main worktree, a stub `claude` listing `clm` as that directory source, and `$CARGO_HOME/.crates.toml` recording `clm` from a `path+file:` source
- WHEN the script runs
- THEN the stub `claude` records `plugin marketplace remove clm` and then `plugin marketplace add clunny/clm`, the stub `cargo` records `uninstall clm`, the settings file sets `env.CLM_BIN` to `<common>/clm-dev/bin/clm` and holds no `extraKnownMarketplaces`, and the output says each project installs the plugin again

#### Scenario: the tracked settings left alone
- GIVEN the earlier setup above, the main worktree tracking a `.claude/settings.json` that enables `clm@clm` and lists `extraKnownMarketplaces.clm` as the `github` source `clunny/clm`, and a stub `claude` that, on `plugin marketplace remove <name>`, deletes every `enabledPlugins` key ending `@<name>` and `extraKnownMarketplaces.<name>` from the `.claude/settings.json` of the directory it runs in, when there is one, as Claude Code does
- WHEN the script runs
- THEN the stub `claude` records `plugin marketplace remove clm` run in `/`, and `git status --porcelain --untracked-files=no` in the main worktree prints nothing

#### Scenario: a release install left in place
- GIVEN `$CARGO_HOME/.crates.toml` recording `clm` from a `git+` source, and a `clm` on `PATH`
- WHEN the script runs
- THEN the stub `cargo` records no `uninstall`, and the output names that `clm`'s path and says it runs ahead of the shim

#### Scenario: another clone's record kept
- GIVEN a stub `claude` listing `clm` as a `github` source, and then in turn as a `directory` source at another path
- WHEN the script runs
- THEN the stub `claude` records no `plugin marketplace remove`

#### Scenario: existing settings kept
- GIVEN a `.claude/settings.local.json` holding `permissions`, an `enabledPlugins` with another plugin and an `env` with another variable
- WHEN the script runs
- THEN all three are still there, beside `CLM_BIN` and the plugin entries, and running it a second time leaves the file's JSON unchanged

#### Scenario: refuses before changing anything
- GIVEN in turn: no `jq` on `PATH`; no `claude` on `PATH`; the main worktree on a branch `topic`; a `reference-transaction` hook without `# clm dev-install`; a settings file holding `{`
- WHEN the script runs without `--force`
- THEN it exits 1 naming the reason, the stub `cargo` was not run, and the hook and the settings file are as they were

#### Scenario: a failing claude command stops it
- GIVEN a stub `claude` that exits 1 on `plugin install`
- WHEN the script runs
- THEN it exits 1 naming that command, and the settings file is as it was

#### Scenario: foreign hook replaced with force
- GIVEN a `reference-transaction` hook without `# clm dev-install`
- WHEN the script runs with `--force`
- THEN it exits 0 and the hook holds `# clm dev-install`

#### Scenario: earlier post-merge hook removed
- GIVEN in turn: a `post-merge` hook holding `# clm dev-install`; a `post-merge` hook without it
- WHEN the script runs
- THEN the first is gone and the second is unchanged

#### Scenario: the hook fires on a land's push only
- GIVEN a clone set up by the script whose `origin` is a bare repository added with `git remote add`, so it has the default fetch refspec, with the main worktree's `scripts/dev-install.sh` replaced by a stub that records its arguments
- WHEN a linked worktree commits and pushes the commit to `origin`'s `main` with `git push origin <commit>:refs/heads/main`, and then commits on a branch `topic` and fetches with nothing new on `origin`
- THEN the stub was run once, with `--rebuild`, during the push

#### Serves
- architecture/pin-working-method
- capability/install-and-release

### Requirement: dev-install-remove <!-- base: 2899ca2ffaf36ffa -->
`scripts/dev-install.sh --remove` SHALL undo the setup on the clone's main worktree: delete the `reference-transaction` hook and the `post-merge` hook, each when it holds `# clm dev-install` (and leave any other hook); when `<common>/clm-dev` or `<main worktree>/.clm/dev-marketplace.json` exists, exit 1 with the reason, before changing anything, when `claude` isn't on `PATH`, and otherwise run, in the main worktree, `claude plugin uninstall <name>@clm-dev --scope local` for each plugin that `.clm/dev-marketplace.json` lists, or, when it is missing, `<common>/clm-dev/marketplace/.claude-plugin/marketplace.json`, which an earlier setup wrote (none when the file it reads isn't a manifest it can read), and then, in `/`, as step 5 of `clm/dev-install` runs its removal, `claude plugin marketplace remove clm-dev`, going on when one of them fails, and delete `<common>/clm-dev` and `.clm/dev-marketplace.json`; delete from `.claude/settings.local.json` `env.CLM_BIN`, every `enabledPlugins` key ending in `@clm-dev`, for each plugin `<name>` the main worktree's `.claude-plugin/marketplace.json` lists, `enabledPlugins."<name>@clm"` when its value is false, and `extraKnownMarketplaces.clm`, removing `env`, `enabledPlugins` or `extraKnownMarketplaces` when that leaves it empty, and keep every other key. It SHALL also do what steps 5 and 8 of `clm/dev-install` do, step 5 only when `claude` is on `PATH`, so a clone on an earlier setup leaves nothing behind. It SHALL NOT delete `clm-install-src`, and SHALL leave every file git tracks in the clone as it was. It SHALL print that the clone's next session runs the release, and exit 0, also when there was nothing to remove.

#### Scenario: undone
- GIVEN a clone that `scripts/dev-install.sh` set up, whose settings file also holds `permissions`, and a stub `claude`
- WHEN `scripts/dev-install.sh --remove` runs
- THEN the hook, `<common>/clm-dev` and the main worktree's `.clm/dev-marketplace.json` are gone, the stub `claude` records `plugin uninstall clm@clm-dev --scope local` and `plugin marketplace remove clm-dev`, the settings file holds `permissions` and none of `env`, `enabledPlugins` and `extraKnownMarketplaces`, the output says the next session runs the release, and it exits 0

#### Scenario: an earlier setup undone
- GIVEN a settings file holding `env.CLM_BIN` at `$CARGO_HOME/bin/clm` and `extraKnownMarketplaces.clm`, no `<common>/clm-dev`, a stub `claude` listing `clm` as a directory source at the main worktree, and `$CARGO_HOME/.crates.toml` recording `clm` from a `path+file:` source
- WHEN `scripts/dev-install.sh --remove` runs
- THEN the stub `claude` records `plugin marketplace remove clm` and then `plugin marketplace add clunny/clm`, the stub `cargo` records `uninstall clm`, the settings file holds neither entry, and it exits 0

#### Scenario: the tracked settings left alone
- GIVEN in turn: the clone of `undone`; the earlier setup of `an earlier setup undone`; each with the main worktree tracking a `.claude/settings.json` that enables `clm@clm` and lists `extraKnownMarketplaces.clm`, and the stub `claude` of `clm/dev-install`'s `the tracked settings left alone`
- WHEN `scripts/dev-install.sh --remove` runs
- THEN the stub `claude` records each `plugin marketplace remove` run in `/`, and `git status --porcelain --untracked-files=no` in the main worktree prints nothing

#### Scenario: already gone from Claude Code
- GIVEN a clone set up by the script and a stub `claude` that exits 1 on every `plugin` command
- WHEN `scripts/dev-install.sh --remove` runs
- THEN `<common>/clm-dev` and the settings entries are gone, and it exits 0

#### Scenario: foreign hooks kept
- GIVEN a `reference-transaction` hook and a `post-merge` hook, neither holding `# clm dev-install`
- WHEN `scripts/dev-install.sh --remove` runs
- THEN both hooks are unchanged and it exits 0

#### Serves
- architecture/pin-working-method
- capability/install-and-release

### Requirement: health-isolated <!-- base: 82b4324b71513766 -->
The crates `clm-check`, `clm-gate` and `clm-land` SHALL NOT depend on `clm-health`, directly or through any crate they depend on, counting normal and build dependencies and not dev-dependencies, so no check, gate or land can read what `clm health` reports (`health-advises`). A test reads the workspace's dependency graph (`cargo metadata`) and fails naming the crate and the path that reaches `clm-health`.

#### Scenario: health reached through a dependency
- GIVEN a workspace where `clm-gate` depends on a crate that depends on `clm-health`
- WHEN the test runs
- THEN it fails naming `clm-gate` and the path to `clm-health`

#### Scenario: today's workspace
- GIVEN this repository's workspace
- WHEN the test runs
- THEN it passes

#### Serves
- capability/know-health

### Requirement: dev-install-rebuild <!-- base: a526b30180509762 -->
`scripts/dev-install.sh --rebuild`, which the hook runs, SHALL bring the installed `clm` up to `origin/main` in the background and exit 0 without waiting for it. The installed `clm` is the file the main worktree's `.claude/settings.local.json` names in `env.CLM_BIN` when that path ends `/bin/clm`, and otherwise `<common>/clm-dev/bin/clm`, and `<root>` is that path without its `/bin/clm`. With `<common>` the common git directory and the main worktree both found, as absolute paths, before anything else, from the worktree git runs the hook in, it SHALL exit 0, starting nothing, when `refs/remotes/origin/main` doesn't resolve, or when the directory `<common>/clm-install.lock` exists and either has no file `pid` or names in `<common>/clm-install.lock/pid` a process that is running. Otherwise it SHALL take that lock with `mkdir` (first deleting one whose `pid` names a process that isn't running), and exit 0 starting nothing when `mkdir` fails; print one line naming `<common>/clm-install.log`; and start a background process, its output appended to that log and with `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` and `GIT_PREFIX` unset, that writes its own process ID to the lock's `pid` and repeats these steps until `origin/main` names the commit its last step built or found current:
1. Read `origin/main`'s commit `<c>`.
2. Take `<c>` as current when the installed binary's `--version` ends with a parenthesised commit that is `<c>`'s first 12 hex digits, or one for which `git diff --name-only <commit> <c> -- crates Cargo.toml Cargo.lock` succeeds and lists nothing. A binary that is missing, whose `--version` names no commit, or whose commit the clone doesn't have is not current.
3. Otherwise check out `<c>`, detached, in `<common>/clm-install-src`, made first with `git clone --shared --no-checkout <main worktree> <common>/clm-install-src` when missing, and run `cargo install --locked --path <common>/clm-install-src/crates/clm --root <root> --target-dir <main worktree>/target/install`; when that fails, log cargo's status and stop.
Then it SHALL remove the lock, also when it stopped on a failure, and, unless it stopped on a failure, run `--rebuild` again when `origin/main` no longer names the commit its last step built or found current, so a move that came while the lock was being released isn't missed.

#### Scenario: a Rust change landed
- GIVEN a clone set up by the script, a stub `cargo`, an installed stub `clm` whose `--version` names commit `A`, and `origin/main` at `B`, where `A..B` changes `crates/clm/src/main.rs`
- WHEN `scripts/dev-install.sh --rebuild` runs and the lock is gone again
- THEN it exited 0 having printed the log's path, the stub `cargo` was run once with `install --locked --path <common>/clm-install-src/crates/clm --root <common>/clm-dev --target-dir <main worktree>/target/install`, writing to the log, `<common>/clm-install-src`'s `HEAD` is `B`, and the main worktree's `HEAD` and files are as they were

#### Scenario: nothing to rebuild
- GIVEN in turn: an installed `clm` naming `A` where `A..B` changes only `specs/SPEC.md`; an installed `clm` naming `B`; no `refs/remotes/origin/main`
- WHEN `scripts/dev-install.sh --rebuild` runs and the lock is gone again
- THEN it exits 0 and the stub `cargo` is not run

#### Scenario: one rebuild at a time
- GIVEN in turn: `<common>/clm-install.lock/pid` holding the ID of a running process; `<common>/clm-install.lock` with no `pid`
- WHEN `scripts/dev-install.sh --rebuild` runs
- THEN it exits 0, the stub `cargo` is not run and the lock is unchanged

#### Scenario: a dead rebuild's lock replaced
- GIVEN `<common>/clm-install.lock/pid` holding the ID of a process that has exited, and a Rust change to build
- WHEN `scripts/dev-install.sh --rebuild` runs and the lock is gone again
- THEN the stub `cargo` was run once

#### Scenario: main moved during a rebuild
- GIVEN a stub `cargo` that installs a stub `clm` naming the commit `<common>/clm-install-src` is checked out at, records that commit on each run, and on its first run moves `refs/remotes/origin/main` from `B` to `C`, a commit after `B` that changes `Cargo.lock`
- WHEN `scripts/dev-install.sh --rebuild` runs and the lock is gone again
- THEN the stub was run twice, at `B` and then at `C`

#### Scenario: a clone set up before moving
- GIVEN a settings file holding `env.CLM_BIN` at `$CARGO_HOME/bin/clm`, an installed stub `clm` there naming `A`, and a Rust change in `A..B`
- WHEN `scripts/dev-install.sh --rebuild` runs and the lock is gone again
- THEN the stub `cargo` was run once with `--root $CARGO_HOME`

#### Serves
- architecture/pin-working-method
- capability/install-and-release
