---
component: clm
base: { frontmatter: "429221e18bbc0329", design: "ee11b790c19e04fa" }
---

## MODIFIED Frontmatter

```yaml
paths: ["Cargo.toml", "Cargo.lock", "crates/clm/Cargo.toml", "crates/clm/build.rs", "crates/clm/src/**", "crates/clm/tests/*.rs", "rust-toolchain.toml", ".claude/settings.json", ".gitattributes", "scripts/dev-install.sh", "README.md", "CONTRIBUTING.md", "LICENSE"]
```

## MODIFIED Design

The children split the work into deterministic core (`model`, `check` with its children `check.coverage`, `check.affected`, `check.unreferenced` and `check.health`, `apply`, `context`), coordination (`orchestrator` with its children `orchestrator.run`, `orchestrator.checkpoint`, `orchestrator.triage` and `orchestrator.runner`, and `sync` with its child `sync.regenerate`), records and landing (`ledger`, `gate` with its child `gate.fingerprint`, `land` with its child `land.approval`, and `rename`, whose replay the gate trusts), plumbing (`vcs`, `host`), presentation (`view`), integration (`plugin` with `plugin.hooks`, which holds `plugin.hooks.guard` and its `plugin.hooks.guard.shell`, and `plugin.panel`, `ci`, and `web` with `web.worker` (with its children `web.worker.platform` and `web.worker.review`), `web.client-live` and `web.live`), settings (`config`), adoption (`init`), and the capability layer's tests (`capability`). The core has no knowledge of Claude Code, of any host or of the web. The plugin has no logic that must hold, because such logic belongs in the CLI (`deterministic-enforcement`). `web` reuses the core, the queue, the views and the ledger compiled to WebAssembly, so it shows what `clm` would report, and it decides nothing. `crates/clm` only parses arguments and dispatches: `clm config` to `clm-config`, `clm defect` to `clm-hooks`, `clm init`, `clm survey` and `clm guide` to `clm-init`, `clm status` and `clm view` to `clm-view-repo`, `clm land`, `clm approve` and `clm reconnect` to `clm-land`, `clm ledger` to `clm-ledger`, `clm effort`, `clm capabilities` and `clm architecture` to `clm-effort`, `clm triage` and `clm draft` to `clm-triage`, `clm runner` to `clm-runner`, passing it its own executable as the `clm` it runs, `clm unreferenced` to `clm-unreferenced`, `clm rename` to `clm-rename`, `clm health` and `clm workaround` to `clm-health`, and `clm pr`, `clm branch` and `clm issue` to `clm-host`, passing each the settings it needs from `config`. Every command with a `--root` option defaults it to the CLM repository found from the current directory as `session-start` finds it: the first of the current directory and its parents that holds `specs/SPEC.md`, looking no further up than the first directory that holds `.git`. With none found, it is the current directory, except for `clm survey` and `clm unreferenced`, which then use the top of the git repository, so `/clm:init` maps the whole repository from any directory (`init-finds-its-repository`), and a project without CLM's specs gets its whole workspace checked. `clm init` takes no `--root`, but writes in the CLM repository found the same way, or at the top of the git repository when there is none (`init/init-root`). Running from a subdirectory then gives the same answer as running from the root. A linked worktree, such as `.clm/worktrees/<change>/<item>`, has its own `.git` file, so the search stays in the worktree. A CLM repository in a subdirectory of a larger git repository still works.

The repo declares its build environment once. `rust-toolchain.toml` pins the Rust toolchain to an exact version, with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target, and rustup installs it on first use on every machine and in every `clm run` worktree (`pinned-rust-toolchain`). Moving the pin is a code PR that fixes any new lints in the same diff. `.claude/settings.json` lists what unattended sessions may run beyond `clm run`'s own commands: `cargo`, a narrow set of npm commands that can't run a dependency's install script, and `.github/scripts/ci-check.sh`, for a session that runs the script itself (`unattended-commands`). A build runs exactly the checks `clm land` runs with `clm land --check-only` (`land-check-command`): Claude Code's worktree isolation refused the script's path in a build, and the `clm` command names none. The script installs Playwright's browser with `--with-deps` only when `CI` is `true` (`ci/browser-install`), so running it never reaches `sudo`. A test in `crates/clm/tests/` reads both files.

The root `.gitattributes` marks `Cargo.lock`, `web/package-lock.json` and `plugin-panel/package-lock.json` `linguist-generated` (`lockfiles-generated`), so neither counts toward its component's size (`check/node-size`): a session never reads a lockfile. They are written bare (`Cargo.lock linguist-generated`), which `git check-attr` reports as `set`. `crates/clm` also dispatches `clm size` to `clm-check`, and `clm affected` to `clm-affected` with `config`'s `[affected] all` globs, and loads `config`'s `component_size` and passes it to `clm size` and to `clm rebase`'s split-item step, as it passes `detectors` to `clm check`, and loads `config`'s `health` and passes it to `clm health` and to `clm rebase`'s reconsider step the same way (`check.health`).

The root's `paths` name `crates/clm`'s manifest, build script, sources and top-level test files, not the whole crate, so that `crates/clm/tests/capability/`, the capability tests' target, belongs to `capability` alone (`check/paths-disjoint`).

`crates/clm/src/main.rs` is dispatch only, and the build of whichever component gains or changes a command edits it, and `Cargo.toml` and `Cargo.lock` when the component adds a crate: the root's own builds cover only the root's requirements. Two builds in one wave may both touch those files; `clm rebase`'s rebase reports the conflict and the session resolves it.

**Running CLM's own repository on its own build** (`dogfood-from-main`, `dev-install-script`). `scripts/dev-install.sh` is a POSIX `sh` script for clones of CLM's own repository, not something CLM ships or `clm init` writes: it makes a clone's sessions, and only those, run `main`'s `clm` and the checkout's plugin, where they would otherwise run the release; every other project on the machine keeps the release (`dev-install-scoped`). A human runs it once per clone; builds never run it (`no-shared-install`). It works on the clone's *main worktree*, the first worktree `git worktree list --porcelain` lists (as `config` finds it for `clm.local.toml`), whichever worktree it runs from, because Claude Code reads `.claude/settings.local.json` only there and the hook lives in the git directory every worktree shares. It merges JSON with `jq` (`reuse-first`); everything else is `git`, `cargo`, `claude` and `sh`. Everything it installs lies in `<common>/clm-dev`, in the git directory every worktree shares: the binary at `bin/clm`, which becomes `CLM_BIN`, and at `marketplace/` a Claude Code marketplace named `clm-dev`, made from the tracked `.claude-plugin/marketplace.json` with each plugin's folder symlinked to the main worktree, so its plugins load in place and a skill edit applies at the next session. `settings.local.json` enables those plugins and turns off their `@clm` namesakes, which the tracked settings enable, and a linked worktree reads that file from the main worktree, so one file covers the clone and its worktrees, while the `clm` marketplace record every project shares stays on GitHub. A plain `clm` outside a session finds nothing: only a session's `PATH` holds the shim. The rebuild installs where the settings' `CLM_BIN` points, so a clone set up before `dev-install-scoped` keeps being rebuilt until its human runs the script again, and `CLAUDE.md` tells a session to ask for that. **The rebuild follows `origin/main`** (`rebuild-on-origin-main`). `main` moves through `clm land`, which pushes it from whichever worktree lands, so the main worktree rarely merges and a `post-merge` hook there misses most of what lands. Every way `main` moves on this clone moves `refs/remotes/origin/main` (a land's push updates it, as does any fetch or pull, a rebasing one included), and a ref update in any worktree runs the `reference-transaction` hook in the git directory they share. So the hook is a `reference-transaction` hook of a few lines: on the `committed` state, when a line of its input names `refs/remotes/origin/main`, it runs the tracked script with `--rebuild`; on anything else it exits at once, since git runs it for every ref update. A change to the rebuild reaches every clone with the next pull, without reinstalling the hook. The hook marks itself with the line `# clm dev-install`, which is how the script tells its own hook from someone else's. The rebuild builds `origin/main`'s commit, not the main worktree's: it keeps its own source, `clm-install-src` in the common git directory, a `git clone --shared --no-checkout` of the clone that borrows its objects, checked out detached at the commit to build, so it never touches a worktree a human or a session works in, and is no worktree of the clone, so nothing that lists worktrees sees it; `build.rs` reads the commit there, so `clm --version` names `origin/main`'s commit. It skips the build when the installed binary's `--version` names `origin/main`'s commit, or a commit from which nothing under `crates/`, `Cargo.toml` or `Cargo.lock` changed. One rebuild runs at a time, holding the lock directory `clm-install.lock` (with its process ID) in the common git directory; a rebuild that finds the lock held by a running process leaves it to that one, which reads `origin/main` again once it has installed and builds again when it moved, so a burst of lands ends with the binary at the last of them. `cargo install` replaces the binary by renaming, so a `clm` already running keeps its own file, and `clm land` runs from a copy of itself (`land/land-binary-copy`). Every setup run deletes a `post-merge` hook holding `# clm dev-install`, which earlier runs wrote.

**Which build runs.** `crates/clm/build.rs` reads the commit the source was built from with `git rev-parse HEAD` in the crate's directory and passes its first 12 hex digits to the compiler; `clm --version` appends it. It finds the worktree's git directory and the common one with `git rev-parse --git-dir --git-common-dir` and declares `cargo:rerun-if-changed` on whichever of these exist: the worktree's `HEAD`, the common directory's `packed-refs`, and its `refs/heads/` directory, which cargo scans recursively and whose entries git replaces by rename on every commit. It never declares a path that doesn't exist, since cargo would then rerun the script, and relink `clm`, on every build. So a commit or a pull rebuilds the string without rebuilding anything else, a detached `HEAD` (`clm land`'s scratch tree, the release checkout) is covered by `HEAD` itself, and a source outside a git checkout, or a machine without `git`, builds with no commit and no error. The string's formatting, `version_string(package_version, commit)`, sits in `crates/clm/src/version.rs`, which `build.rs` includes with `#[path]` and which holds its own unit test; `main.rs` only hands clap the string the build script produced. The build script is not the binary, so `git-through-layer`, which is about what `clm` runs, doesn't cover it. Release binaries are built from a checkout of the release commit and so show it too.

**A reader that leaves** (`closed-stdout`). The Rust runtime ignores `SIGPIPE`, so a print to a stdout whose reader has gone fails, and `println!` turns the failure into a panic wherever it stands. `crates/clm/src/closed_stdout.rs` handles that panic in one place: `main` installs a panic hook, then runs the dispatch inside `catch_unwind`. The hook is silent for a message that is std's `failed printing to stdout: ` with an `(os error N)` for which `std::io::Error::from_raw_os_error(N).kind()` is `BrokenPipe`, since that holds on Windows's release too where the text differs, and passes every other panic to the default hook. `main` exits 141 for that panic, the status a shell reports for a tool `SIGPIPE` killed, which no command's own exit codes use, and resumes unwinding for any other. It unwinds, not exits in the hook, so destructors still remove temporary trees and release locks. std's wording is matched because the toolchain is pinned (`pinned-toolchain`) and the rule's test runs the real binary, so a pin move that changes it fails that test. Every print to stdout is on the main thread. Restoring `SIGPIPE`'s default is not an option: it would also kill `clm` silently when a child that `clm-git`'s `run` feeds exits before reading.

**Component map.** One line per component; each component's Contract says the rest. Adding or splitting a component edits this table, and needs no review.

| Component | Responsibility |
|---|---|
| `clm` (root) | The product: plugin + `clm` CLI |
| `model` | Parse specs and changes into the spec graph; content hashes |
| `check` | Deterministic checks: schema, graph integrity, drift, test coverage, staleness |
| `check.coverage` | Markers and coverage: which requirements are current or pending, which changes are complete |
| `check.affected` | `clm affected`: the components and files a change can break, read from the spec graph, so a land runs only their tests |
| `check.unreferenced` | `clm unreferenced`: a dead-code detector for public items nothing else in the workspace uses |
| `check.health` | `clm health`: one advisory report of the repository's health, and `clm workaround`: a build's flag that a requirement was hard to build |
| `gate` | `clm gate`: whether a branch may land, and whether it needs review |
| `gate.fingerprint` | `clm fingerprint`: the fingerprint an approval pins, from two tree listings (crate `clm-fingerprint`) |
| `rename` | `clm rename`: a change's recorded renames carried out across specs, references and test markers, and the replay the gate trusts |
| `apply` | `clm apply` and `clm hashes`: a change's deltas folded into `specs/` |
| `context` | Deterministic context assembler (ancestors + one-hop contracts) |
| `vcs` | The one place that runs `git` and `gh` (crate `clm-git`); every component that shells out calls it |
| `orchestrator` | Component queue, component and backlog claims, proposals |
| `orchestrator.run` | `clm run`: the prompt and launch of an unattended build |
| `orchestrator.checkpoint` | `clm wrap-up`: what a session leaves behind |
| `orchestrator.triage` | `clm triage` and `clm draft`: the backlog grouped into unattended drafts |
| `orchestrator.runner` | `clm runner`: unattended headless builds on a machine the human owns |
| `ledger` | Approvals, checks and lands as records in per-writer ledger branches |
| `land` | `clm land`, `clm approve`, `clm reconnect`: landing on `main` from the machine that runs it |
| `land.approval` | Whether a branch's diff is approved or held, and `clm approve` (crate `clm-approval`) |
| `effort` | `clm effort record`, `clm capabilities` and `clm architecture`: the work each land spent, and what each capability and architectural constraint costs |
| `host` | The host adapter: pull requests, remote branch deletion and issues on GitHub or no host |
| `view` | Views for people: `clm status` and `clm view` (spec, flight, blockers) as charts, SVG and JSON |
| `sync` | Git integration: autosync, merge drivers, conflict resolution |
| `sync.regenerate` | The regenerated `specs/` for `clm rebase` and `clm land`, its stale-delta merges and report (crate `clm-regenerate`) |
| `plugin` | Claude Code plugin: packaging, shim, skills, subagents |
| `plugin.hooks` | The rules behind `clm hook session-start`, `clm hook autosync` and `clm defect` |
| `plugin.hooks.guard` | `clm hook guard`: which Bash commands a session's guard asks about or blocks |
| `plugin.hooks.guard.shell` | How the guard reads a Bash command as the shell would |
| `plugin.panel` | The `clm-panel` plugin: `/clm:view` and the CLM pane |
| `ci` | The release workflow on GitHub Actions, and this repository's check script |
| `web` | Optional hosted web app: the browser app, the deploy, the invited users |
| `web.worker` | The Worker behind `web`: sign-in, the API, the webhook, the caches |
| `web.worker.platform` | The layer the Worker's routes stand on: platform traits, HTTP, access, the GitHub client, file fetches, keys, live state, ledger reading and the web-ledger append |
| `web.worker.review` | A spec branch in the Worker: the spec-branch list, the review, approve, comments and change requests |
| `web.client-live` | The browser app's live socket, Notify me switch and push handlers |
| `web.live` | The pushes from `web`: the Durable Object that tells open pages of changes, and Web Push |
| `config` | Settings: environment, the clone's `clm.local.toml`, the repo's `clm.toml`, the user's `config.toml`, defaults |
| `capability` | The capabilities' black-box tests, one file per capability (`human-layer-proof`) |
| `init` | Adoption: `clm init` sets up a repo's tooling, `clm survey` maps an existing codebase, `clm guide` prints the guide |

## ADDED Requirements

### Requirement: contributing-doc
`CONTRIBUTING.md` at the repository's root SHALL tell a contributor to CLM to run `scripts/dev-install.sh` once per clone, with what it needs on `PATH` (`jq`, `cargo`, `claude`), say what it changes (the binary and marketplace under the clone's git directory, `.claude/settings.local.json`, the `reference-transaction` hook, Claude Code's user-level marketplace list gaining `clm-dev`), that the clone and its worktrees then run `origin/main`'s `clm` and the main checkout's plugin while other projects run the release, that a plain `clm` outside a session finds nothing, so `cargo run -q -p clm --` runs it in the clone, with `<common>/clm-dev/bin` put on `PATH` for a `clm check` outside a session, since its `unreferenced-public-items` detector runs the first `clm` on `PATH`, and that `scripts/dev-install.sh --remove` undoes it.

#### Scenario: a contributor reads it
- GIVEN this repository's `CONTRIBUTING.md`
- WHEN a test reads it
- THEN it names `scripts/dev-install.sh`, `--remove`, `jq`, `cargo`, `claude`, `clm-dev`, `settings.local.json`, `reference-transaction`, `cargo run -q -p clm --` and `clm-dev/bin`

#### Serves
- capability/install-and-release

## MODIFIED Requirements

### Requirement: dev-install <!-- base: 295dabede2d19654 -->
`scripts/dev-install.sh`, run with no arguments from any worktree of a clone of CLM's repository, SHALL set the clone up so that it and its worktrees, and no other project on the machine, run `main`'s `clm` and the checkout's plugin (`dev-install-scoped`), in this order, and print each step it took. `<common>` is the clone's common git directory, as an absolute path.
1. It SHALL exit 1 with the reason, having changed nothing, when `jq`, `cargo` or `claude` is not on `PATH`, when the main worktree has no `.claude-plugin/marketplace.json` whose `name` is `clm`, or one listing a plugin whose `source` isn't a string starting `./`, when the main worktree's branch is not `main`, when its `.claude/settings.local.json` exists and isn't empty or one JSON object whose `env`, `enabledPlugins` and `extraKnownMarketplaces`, where present, are objects (naming the file), when `claude plugin marketplace list --json` fails or prints something other than a JSON array, or when a `reference-transaction` hook exists at `git rev-parse --git-path hooks/reference-transaction` that holds no line `# clm dev-install` and `--force` was not given.
2. It SHALL run `cargo install --locked --path crates/clm --root <common>/clm-dev --target-dir target/install` in the main worktree, and exit 1 with cargo's status message, changing nothing else, when it fails.
3. It SHALL write the `reference-transaction` hook at that path, executable, holding the line `# clm dev-install`, replacing any hook there. The hook SHALL exit 0, running nothing, unless its first argument is `committed` and a line of its standard input ends with ` refs/remotes/origin/main`; then it SHALL run the main worktree's `scripts/dev-install.sh --rebuild`. It SHALL delete the hook at `git rev-parse --git-path hooks/post-merge` when that holds `# clm dev-install`, and leave any other `post-merge` hook.
4. It SHALL write `<common>/clm-dev/marketplace/.claude-plugin/marketplace.json`, the main worktree's with `name` set to `clm-dev`, and for each plugin it lists a symlink at `<common>/clm-dev/marketplace/<source>` to `<main worktree>/<source>`.
5. When `claude plugin marketplace list --json` holds an entry whose `name` is `clm`, `source` is `directory` and `path`, with symlinks resolved, is the main worktree's, it SHALL run `claude plugin marketplace remove clm` and then `claude plugin marketplace add clunny/clm`, and print that each project enabling the plugin installs it again at its next session.
6. It SHALL run, in the main worktree, `claude plugin marketplace add <common>/clm-dev/marketplace` and, for each plugin `<name>` the marketplace lists, `claude plugin install <name>@clm-dev --scope local`, exiting 1 with the failing command when one fails, leaving the settings file as it was; running the script again finishes the setup.
7. It SHALL set, in the main worktree's `.claude/settings.local.json`, `env.CLM_BIN` to `<common>/clm-dev/bin/clm` and, for each such plugin, `enabledPlugins."<name>@clm-dev"` to true and `enabledPlugins."<name>@clm"` to false, and delete `extraKnownMarketplaces.clm` (and `extraKnownMarketplaces` when that leaves it empty), creating the file and `.claude/` when missing and keeping every other key as it was.
8. When `${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/.crates.toml` records a package `clm` installed from a `path+file:` source, it SHALL run `cargo uninstall clm`. When a `clm` is still on `PATH` that isn't a plugin's shim (with `<dir>` the directory holding it, `<dir>/../.claude-plugin/plugin.json` doesn't exist), it SHALL print its path, and that it runs ahead of the plugin's shim in every project.
9. It SHALL print the installed binary's `--version`, say that the next session of this clone and its worktrees loads the plugin from the checkout, that the binary is rebuilt from `origin/main` whenever it moves, that sessions already running should be restarted, and that a plain `clm` outside a session doesn't find this build.
Run again on a clone already set up, it SHALL leave the settings file's content unchanged and the hook as it was.

#### Scenario: fresh clone
- GIVEN a clone on `main` whose `.claude-plugin/marketplace.json` is named `clm` and lists a plugin `clm` with source `./plugin`, no hooks, no `.claude/settings.local.json`, and stub `cargo` and `claude` and real `jq` on `PATH`
- WHEN `scripts/dev-install.sh` runs from a linked worktree of it
- THEN the stub `cargo` records `install --locked --path crates/clm --root <common>/clm-dev --target-dir target/install` run in the main worktree, the `reference-transaction` hook exists, is executable and holds `# clm dev-install`, `<common>/clm-dev/marketplace/.claude-plugin/marketplace.json` is named `clm-dev`, `<common>/clm-dev/marketplace/plugin` is a symlink to the main worktree's `plugin`, the stub `claude` records `plugin marketplace add <common>/clm-dev/marketplace` and then `plugin install clm@clm-dev --scope local` run in the main worktree, and the main worktree's settings file sets `env.CLM_BIN` to `<common>/clm-dev/bin/clm`, `clm@clm-dev` to true and `clm@clm` to false

#### Scenario: nothing outside the clone holds the build
- GIVEN the fresh clone above, with `$CARGO_HOME/.crates.toml` recording no `clm`
- WHEN the script runs
- THEN nothing is written under `$CARGO_HOME/bin`, and the stub `claude` records no `plugin marketplace remove` and no `--scope user`

#### Scenario: an earlier setup moved
- GIVEN a settings file holding `env.CLM_BIN` at `$CARGO_HOME/bin/clm` and `extraKnownMarketplaces.clm` as a directory source at the main worktree, a stub `claude` listing `clm` as that directory source, and `$CARGO_HOME/.crates.toml` recording `clm` from a `path+file:` source
- WHEN the script runs
- THEN the stub `claude` records `plugin marketplace remove clm` and then `plugin marketplace add clunny/clm`, the stub `cargo` records `uninstall clm`, the settings file sets `env.CLM_BIN` to `<common>/clm-dev/bin/clm` and holds no `extraKnownMarketplaces`, and the output says each project installs the plugin again

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
- capability/pin-working-method
- capability/install-and-release

### Requirement: dev-install-remove <!-- base: 4ecdc62e9194655a -->
`scripts/dev-install.sh --remove` SHALL undo the setup on the clone's main worktree: delete the `reference-transaction` hook and the `post-merge` hook, each when it holds `# clm dev-install` (and leave any other hook); when `<common>/clm-dev` exists, exit 1 with the reason, before changing anything, when `claude` isn't on `PATH`, and otherwise run, in the main worktree, `claude plugin uninstall <name>@clm-dev --scope local` for each plugin its marketplace lists and `claude plugin marketplace remove clm-dev`, going on when one of them fails, and delete `<common>/clm-dev`; delete from `.claude/settings.local.json` `env.CLM_BIN`, every `enabledPlugins` key ending in `@clm-dev`, for each plugin `<name>` the main worktree's `.claude-plugin/marketplace.json` lists, `enabledPlugins."<name>@clm"` when its value is false, and `extraKnownMarketplaces.clm`, removing `env`, `enabledPlugins` or `extraKnownMarketplaces` when that leaves it empty, and keep every other key. It SHALL also do what steps 5 and 8 of `clm/dev-install` do, step 5 only when `claude` is on `PATH`, so a clone on an earlier setup leaves nothing behind. It SHALL NOT delete `clm-install-src`. It SHALL print that the clone's next session runs the release, and exit 0, also when there was nothing to remove.

#### Scenario: undone
- GIVEN a clone that `scripts/dev-install.sh` set up, whose settings file also holds `permissions`, and a stub `claude`
- WHEN `scripts/dev-install.sh --remove` runs
- THEN the hook and `<common>/clm-dev` are gone, the stub `claude` records `plugin uninstall clm@clm-dev --scope local` and `plugin marketplace remove clm-dev`, the settings file holds `permissions` and none of `env`, `enabledPlugins` and `extraKnownMarketplaces`, the output says the next session runs the release, and it exits 0

#### Scenario: an earlier setup undone
- GIVEN a settings file holding `env.CLM_BIN` at `$CARGO_HOME/bin/clm` and `extraKnownMarketplaces.clm`, no `<common>/clm-dev`, a stub `claude` listing `clm` as a directory source at the main worktree, and `$CARGO_HOME/.crates.toml` recording `clm` from a `path+file:` source
- WHEN `scripts/dev-install.sh --remove` runs
- THEN the stub `claude` records `plugin marketplace remove clm` and then `plugin marketplace add clunny/clm`, the stub `cargo` records `uninstall clm`, the settings file holds neither entry, and it exits 0

#### Scenario: already gone from Claude Code
- GIVEN a clone set up by the script and a stub `claude` that exits 1 on every `plugin` command
- WHEN `scripts/dev-install.sh --remove` runs
- THEN `<common>/clm-dev` and the settings entries are gone, and it exits 0

#### Scenario: foreign hooks kept
- GIVEN a `reference-transaction` hook and a `post-merge` hook, neither holding `# clm dev-install`
- WHEN `scripts/dev-install.sh --remove` runs
- THEN both hooks are unchanged and it exits 0

#### Serves
- capability/pin-working-method
- capability/install-and-release

### Requirement: dev-install-rebuild <!-- base: 1f1802fc40e0783c -->
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
- capability/pin-working-method
- capability/install-and-release
