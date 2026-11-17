---
component: clm
base: { design: "c537d626ae058453" }
---

## MODIFIED Design

The children split the work into deterministic core (`model`, `check` with its children `check.coverage`, `check.affected`, `check.unreferenced` and `check.health`, `apply`, `context`), coordination (`orchestrator` with its children `orchestrator.run`, `orchestrator.checkpoint`, `orchestrator.triage` and `orchestrator.loop`, and `sync` with its child `sync.regenerate`), records and landing (`ledger`, `gate` with its child `gate.fingerprint`, `land` with its child `land.approval`, and `rename`, whose replay the gate trusts), plumbing (`vcs`, `host`), presentation (`view`), integration (`plugin` with `plugin.hooks`, which holds `plugin.hooks.guard` and its `plugin.hooks.guard.shell`, and `plugin.panel`, `ci`, and `web` with `web.worker` (with its children `web.worker.platform` and `web.worker.review`), `web.client-live` and `web.live`), settings (`config`), adoption (`init`), and the capability layer's tests (`capability`). The core has no knowledge of Claude Code, of any host or of the web. The plugin has no logic that must hold, because such logic belongs in the CLI (`deterministic-enforcement`). `web` reuses the core, the queue, the views and the ledger compiled to WebAssembly, so it shows what `clm` would report, and it decides nothing. `crates/clm` only parses arguments and dispatches: `clm config` to `clm-config`, `clm defect` to `clm-hooks`, `clm init`, `clm survey` and `clm guide` to `clm-init`, `clm status` and `clm view` to `clm-view-repo`, `clm land`, `clm approve` and `clm reconnect` to `clm-land`, `clm ledger` to `clm-ledger`, `clm effort`, `clm capabilities` and `clm architecture` to `clm-effort`, `clm machine` to `clm-machine`, passing it `max_builds`, `clm triage`, `clm draft` and `clm intake` to `clm-triage`, passing `clm intake` the `host` setting, `clm build` to `clm-loop`, passing it `max_builds` and its own executable as the `clm` it runs, `clm unreferenced` to `clm-unreferenced`, `clm rename` to `clm-rename`, `clm health` and `clm workaround` to `clm-health`, and `clm pr`, `clm branch` and `clm issue` to `clm-host`, passing each the settings it needs from `config`. Every command with a `--root` option defaults it to the CLM repository found from the current directory as `session-start` finds it: the first of the current directory and its parents that holds `specs/SPEC.md`, looking no further up than the first directory that holds `.git`. With none found, it is the current directory, except for `clm survey` and `clm unreferenced`, which then use the top of the git repository, so `/clm:init` maps the whole repository from any directory (`init-finds-its-repository`), and a project without CLM's specs gets its whole workspace checked. `clm init` takes no `--root`, but writes in the CLM repository found the same way, or at the top of the git repository when there is none (`init/init-root`). Running from a subdirectory then gives the same answer as running from the root. A linked worktree, such as `.clm/worktrees/<change>/<item>`, has its own `.git` file, so the search stays in the worktree. A CLM repository in a subdirectory of a larger git repository still works.

The repo declares its build environment once. `rust-toolchain.toml` pins the Rust toolchain to an exact version, with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target, and rustup installs it on first use on every machine and in every `clm run` worktree (`pinned-rust-toolchain`). Moving the pin is a code PR that fixes any new lints in the same diff. `.claude/settings.json` lists what unattended sessions may run beyond `clm run`'s own commands: `cargo`, a narrow set of npm commands that can't run a dependency's install script, and `.github/scripts/ci-check.sh`, for a session that runs the script itself (`unattended-commands`). A build runs exactly the checks `clm land` runs with `clm land --check-only` (`land-check-command`): Claude Code's worktree isolation refused the script's path in a build, and the `clm` command names none. The script installs Playwright's browser with `--with-deps` only when `CI` is `true` (`ci/browser-install`), so running it never reaches `sudo`. A test in `crates/clm/tests/` reads both files.

The root `.gitattributes` marks every lockfile the repository tracks, each `Cargo.lock` and each `package-lock.json`, `linguist-generated` (`lockfiles-generated`), so none counts toward its component's size (`check/node-size`): a session never reads a lockfile. The lines are written bare, by file name with no `/` (`Cargo.lock linguist-generated`, `package-lock.json linguist-generated`), so they mark a file of that name at any depth, a new npm package's included, and `git check-attr` reports each as `set`. `crates/clm` also dispatches `clm size` to `clm-check`, and `clm affected` to `clm-affected` with `config`'s `[affected] all` globs, and loads `config`'s `component_size` and passes it to `clm size` and to `clm rebase`'s split-item step, as it passes `detectors` to `clm check`, and loads `config`'s `health` and passes it to `clm health` and to `clm rebase`'s reconsider step the same way (`check.health`).

The root's `paths` name `crates/clm`'s manifest, build script, sources and top-level test files, not the whole crate, so that `crates/clm/tests/capability/`, the capability tests' target, belongs to `capability` alone (`check/paths-disjoint`). The tests of the components that have no crate sit beside it the same way, each directory a test target of this package that its component's `paths` name: `crates/clm/tests/ci_suite/` is `ci`'s, `crates/clm/tests/plugin_suite/` is `plugin`'s, `crates/clm/tests/plugin_agents_suite/` is `plugin.agents`'s and `crates/clm/tests/plugin_panel_suite/` is `plugin.panel`'s. They read workflows, scripts and plugin files and run no binary, so they need this package only to run in (`commands-live-in-their-crates`).

**Commands live in their crates** (`commands-live-in-their-crates`). A command's arguments, what it prints and its exit status belong to the component whose requirements state them. That component's crate defines the command's arguments, as a type the command-line parser reads, and offers the command as a call that parses them from the strings the command line gives, and takes the root, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status (each component's `commands-in-crate`). `crates/clm` builds its one parser from those types, applies what holds for every command (`upgrade-blocks-commands`, `none-remote-before-commands`, `root-from-cwd`, `closed-stdout`), loads the settings a crate is passed, and calls the crate with the process's own streams and environment, exiting with what the call returns (`routes-only`). The streams it passes fail as `println!` does when the reader has gone, so `closed-stdout` holds as before. `clm hook` is one command over two crates: `crates/clm` keeps the name, reads standard input once and routes `guard` to `clm-guard` and the other hooks to `clm-hooks`. A crate prints a violation as `clm-model` gives its line (`model/violation-line`), whether or not it depends on `check`. A crate that the web app compiles to WebAssembly offers its call in its native build only. Where a command's text is stated by a component below the crate that offers its call, as `clm status` and `clm view` are by `view`, `clm pending` and the lines of `clm coverage` by `check.coverage`, the lower component's tests assert the text its own crate returns, and the offering crate's tests that the call writes that text unchanged, with its stream and exit status. The commands whose behaviour the root's own requirements state stay as they are: `clm contribution`, `clm upgrade`, `clm verify`, `clm machine`, `clm agent-harness`, `clm build` and `clm fingerprint`.

Until `routes-only` is built, the top-level test files under `crates/clm/tests/` still hold tests of other components, which run the binary because Cargo gives it only to this package's tests. `changes/20261115-commands-in-their-crates/moved-tests.txt` (under `changes/archive/` once the change is complete) lists each one: the component it proves, its file, its name and whether it runs the binary. A component's build of `commands-in-crate` moves every test on its lines into its own crate under the same name, with the arguments it passes and everything it asserts unchanged, replacing only the helper that ran the binary by a call of the crate; it deletes the test from the root's file, with any helper nothing there uses any longer and the file itself when nothing is left in it, and deletes no test it hasn't moved. Its pull request says how many it moved against how many the list holds for it. In the same build it points its arm of `crates/clm/src/main.rs` at the call and deletes the code there that the call replaces. The land lets a build change a root test file that cites its component until `gate/own-files` is built without that clause, which waits for `routes-only` and for the backlog items that move the other tests; from then on a build changes its own files, the wiring and a capability test it serves, whatever a file cites. Where a component's Design or a requirement's wording still places its tests under `crates/clm/tests/`, this paragraph and the component's `commands-in-crate` hold.

Three cases need more than the helper. **A test on two components' lines, or one that also cites a requirement of the root**, is never deleted by the first build to reach it: that build writes a test of its own requirement in its own crate and takes its own marker off the test, which stays until no marker of another component is left on it; a test that cites a requirement of the root stays the root's. **A test that set the binary's environment or directory** (a `PATH` holding a stand-in for `git`, `gh` or `claude`, `TMPDIR`, a `GIT_` variable, the directory it ran in) sets them on its own process for the length of the call: `git` and the tools a command starts inherit the process's environment, not the lookup, and the tests run one per process (`ci/nextest-runner`), so no other test sees them. Code of the crate that reads the process's environment, directory or executable path itself reads it through the call's arguments instead. **A test of something only a process has** (a signal, a closed stream, the binary's own file, two commands running at once) stays where it is, and the build files a `deferred` item naming it, which blocks `clm/routes-only`. So does a test whose scenario runs another component's command on what this component made, as a test of `clm check` on what `clm init` wrote does: the crate can neither call a crate above it nor run the binary. Such a scenario is the root's to prove, and a spec change moves it into a requirement of the root (`commands-live-in-their-crates`): `init-result-accepted`, `check-across-components` and `land-as-a-process` hold those the first builds left, and `root-from-cwd` and `old-command-names-retired` the two that were the root's own rule already. A scenario that is wholly such moves whole, word for word. One that also states what its own component made is split at the clause: the component keeps what its crate can show and proves it there, and the root states the clause that runs the other command, in the same words. Leaving such a test with its item is this design and not a workaround: the build flags none with `clm workaround`, finishes the rest of its requirement, and one item per component names every test it left. The item is `deferred` and not a `question`, since the human has answered for the whole class and a question would show as waiting on them: its body says which tests stayed and why, and that it is due with the spec change that follows the build run. A scenario that says `clm <command>` runs, or that the `clm` binary does, states what a person sees and is not reworded: the crate's call proves it, and `routes-only` proves the binary adds nothing, so a build files no question for the wording.

`crates/clm/src/main.rs` is dispatch only, and the build of whichever component gains or changes a command edits it, with `crates/clm/src/upgrade_gate.rs`, the list of commands that run in a project behind on a required migration, and `crates/clm/Cargo.toml`, `Cargo.toml` and `Cargo.lock` when the component adds a crate: the root's own builds cover only the root's requirements. Those five files are the root's `wiring` (`model/wiring-globs`), and naming them turns the land's ownership rule on here (`land/build-holds-own-files`, `own-files-at-the-gate`). They are the files in which each component registers itself with the product; the root's other source files are its own behaviour and stay out (`own-files-at-the-gate` holds the measurement behind the list). `clm.toml` stays out too: it is `config`'s, and each setting this repository makes is a requirement of `config`. `upgrade`'s migrations stay out, and need no entry: a migration written today is `init`'s, beside the file it carries a change of (`migrations-live-with-init`). Two builds in one wave may both touch those files; `clm rebase`'s rebase reports the conflict and the session resolves it.

**Running CLM's own repository on its own build** (`dogfood-from-main`, `dev-install-script`). `scripts/dev-install.sh` is a POSIX `sh` script for clones of CLM's own repository, not something CLM ships or `clm init` writes: it makes a clone's sessions, and only those, run `main`'s `clm` and the checkout's plugin, where they would otherwise run the release; every other project on the machine keeps the release (`dev-install-scoped`). A human runs it once per clone; builds never run it. It works on the clone's *main worktree*, the first worktree `git worktree list --porcelain` lists (as `config` finds it for `clm.local.toml`), whichever worktree it runs from, because Claude Code reads `.claude/settings.local.json` only there and the hook lives in the git directory every worktree shares. It merges JSON with `jq` (`reuse-first`); everything else is `git`, `cargo`, `claude` and `sh`. It installs the binary in `<common>/clm-dev`, in the git directory every worktree shares, at `bin/clm`, which becomes `CLM_BIN`. Its Claude Code marketplace, named `clm-dev`, is the main worktree itself (`dev-marketplace-in-place`): the tracked `.claude-plugin/marketplace.json` with its `name` changed, written to `.clm/dev-marketplace.json` there, which git ignores, and added as a manifest file. Claude Code resolves a manifest file's `./` sources against the folder above the file's own, so the plugins load in place from the main worktree, inside the marketplace's root, and a skill edit applies at the next session. A plugin folder symlinked out of a marketplace doesn't do: Claude Code refuses a plugin hook module whose real path lies outside the marketplace, as the panel's does. The resolution was measured, not documented, so the setup reads `claude plugin list --json` and stops when a `clm-dev` plugin fails to load. `settings.local.json` enables those plugins and turns off their `@clm` namesakes, which the tracked settings enable, and a linked worktree reads that file from the main worktree, so one file covers the clone and its worktrees, while the `clm` marketplace record every project shares stays on GitHub. A plain `clm` outside a session finds nothing: only a session's `PATH` holds the shim. The rebuild installs where the settings' `CLM_BIN` points, so a clone set up before `dev-install-scoped` keeps being rebuilt until its human runs the script again, and `CLAUDE.md` tells a session to ask for that. **The rebuild follows `origin/main`** (`rebuild-on-origin-main`). `main` moves through `clm land`, which pushes it from whichever worktree lands, so the main worktree rarely merges and a `post-merge` hook there misses most of what lands. Every way `main` moves on this clone moves `refs/remotes/origin/main` (a land's push updates it, as does any fetch or pull, a rebasing one included), and a ref update in any worktree runs the `reference-transaction` hook in the git directory they share. So the hook is a `reference-transaction` hook of a few lines: on the `committed` state, when a line of its input names `refs/remotes/origin/main`, it runs the tracked script with `--rebuild`; on anything else it exits at once, since git runs it for every ref update. A change to the rebuild reaches every clone with the next pull, without reinstalling the hook. The hook marks itself with the line `# clm dev-install`, which is how the script tells its own hook from someone else's. The rebuild builds `origin/main`'s commit, not the main worktree's: it keeps its own source, `clm-install-src` in the common git directory, a `git clone --shared --no-checkout` of the clone that borrows its objects, checked out detached at the commit to build, so it never touches a worktree a human or a session works in, and is no worktree of the clone, so nothing that lists worktrees sees it; `build.rs` reads the commit there, so `clm --version` names `origin/main`'s commit. It skips the build when the installed binary's `--version` names `origin/main`'s commit, or a commit from which nothing under `crates/`, `Cargo.toml` or `Cargo.lock` changed. One rebuild runs at a time, holding the lock directory `clm-install.lock` (with its process ID) in the common git directory; a rebuild that finds the lock held by a running process leaves it to that one, which reads `origin/main` again once it has installed and builds again when it moved, so a burst of lands ends with the binary at the last of them. `cargo install` replaces the binary by renaming, so a `clm` already running keeps its own file, and `clm land` runs from a copy of itself (`land/land-binary-copy`). Every setup run deletes a `post-merge` hook holding `# clm dev-install`, which earlier runs wrote.

**Which build runs.** `crates/clm/build.rs` reads the commit the source was built from with `git rev-parse HEAD` in the crate's directory and passes its first 12 hex digits to the compiler; `clm --version` appends it. It finds the worktree's git directory and the common one with `git rev-parse --git-dir --git-common-dir` and declares `cargo:rerun-if-changed` on whichever of these exist: the worktree's `HEAD`, the common directory's `packed-refs`, and its `refs/heads/` directory, which cargo scans recursively and whose entries git replaces by rename on every commit. It never declares a path that doesn't exist, since cargo would then rerun the script, and relink `clm`, on every build. So a commit or a pull rebuilds the string without rebuilding anything else, a detached `HEAD` (`clm land`'s scratch tree, the release checkout) is covered by `HEAD` itself, and a source outside a git checkout, or a machine without `git`, builds with no commit and no error. The string's formatting, `version_string(package_version, commit)`, sits in `crates/clm/src/version.rs`, which `build.rs` includes with `#[path]` and which holds its own unit test; `main.rs` only hands clap the string the build script produced. The build script is not the binary, so `git-through-layer`, which is about what `clm` runs, doesn't cover it. Release binaries are built from a checkout of the release commit and so show it too.

**A reader that leaves** (`closed-stdout`). The Rust runtime ignores `SIGPIPE`, so a print to a stdout whose reader has gone fails, and `println!` turns the failure into a panic wherever it stands. `crates/clm/src/closed_stdout.rs` handles that panic in one place: `main` installs a panic hook, then runs the dispatch inside `catch_unwind`. The hook is silent for a message that is std's `failed printing to stdout: ` with an `(os error N)` for which `std::io::Error::from_raw_os_error(N).kind()` is `BrokenPipe`, since the error's text differs between platforms while its kind doesn't, and passes every other panic to the default hook. `main` exits 141 for that panic, the status a shell reports for a tool `SIGPIPE` killed, which no command's own exit codes use, and resumes unwinding for any other. It unwinds, not exits in the hook, so destructors still remove temporary trees and release locks. std's wording is matched because the toolchain is pinned (`pinned-toolchain`) and the rule's test runs the real binary, so a pin move that changes it fails that test. Every print to stdout is on the main thread. Restoring `SIGPIPE`'s default is not an option: it would also kill `clm` silently when a child that `clm-git`'s `run` feeds exits before reading.

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
| `ledger` | Approvals, checks and lands as records in per-writer ledger branches |
| `land` | `clm land`, `clm approve`, `clm reconnect`: landing on `main` from the machine that runs it |
| `land.approval` | Whether a branch's diff is approved or held, and `clm approve` (crate `clm-approval`) |
| `verify` | `clm verify`: a project's slow tests run after the land, at the moments it chooses, and their result recorded |
| `effort` | `clm effort record`, `clm capabilities` and `clm architecture`: the work each land spent, and what each capability and architectural constraint costs |
| `machine` | `clm machine`: the machine's CPU load, available memory and free disk over the last 30 minutes, judged against what the clone's builds use |
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



### Requirement: init-result-accepted
What `clm init` writes SHALL be a CLM repository the other commands accept, whatever `--lang` it was given. In a repository with no code, `clm check` SHALL pass on it, `clm hashes architecture`, `clm hashes capability` and `clm pending` SHALL list nothing, and `clm setup` SHALL exit 0 while there is no `package.json` and run `npm ci` once there is one. Where a detector's tool isn't installed, `clm check` SHALL report that the detector could not run, with the hint `clm init` wrote, never that it failed. What `clm init` writes is `init`'s to state and to prove in its own crate; each scenario here runs another component's command on it, which only a `clm` process shows, so each SHALL be proven by running the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: empty repository
- GIVEN an empty git repository whose `origin` is `https://github.com/o/r.git`
- WHEN `clm init --lang rust` runs, and then `clm check`
- THEN `clm check` exits 0

#### Scenario: human layers
- GIVEN an empty git repository
- WHEN `clm init` runs
- THEN `clm hashes architecture` and `clm hashes capability` list nothing, and `clm pending` lists nothing

#### Scenario: the written layers
- GIVEN an empty git repository
- WHEN `clm init` runs, and then `clm check`
- THEN `clm check` exits 0

#### Scenario: plugins already enabled
- GIVEN a git repository with no `specs/`, holding an untracked `.claude/settings.json` that sets `enabledPlugins` and `permissions.allow` `["Bash(make *)", "Bash(clm land *)"]`
- WHEN `clm init` runs, and then `clm check`
- THEN `clm check` exits 0

#### Scenario: empty repository, every language
- GIVEN an empty git repository, and on `PATH` a `cargo`, `npm` and `pytest` that each exit 1
- WHEN `clm init --lang rust --lang typescript --lang python` runs, then `clm check`
- THEN it exits 0, none of the three stubs ran, and `clm check`'s stderr names each detector as skipped

#### Scenario: dependencies not installed
- GIVEN a repository holding a `package.json` and no `node_modules/`
- WHEN `clm init --lang typescript` runs and then `clm check`
- THEN `clm check` exits 1 with a line starting `clm.toml: check/detectors: typescript-dead-code could not run:` and ending `; install with: npm ci (or npm install --save-dev knip when package.json doesn't list it)`

#### Scenario: a TypeScript project
- GIVEN a git repository with no code
- WHEN `clm init --lang typescript` runs, then `clm config get build` and `clm setup`
- THEN `clm config get build` prints `setup=test ! -f package.json || npm ci` and the path of `clm.toml`, `clm setup` exits 0, and `clm check` exits 0

#### Scenario: a package.json added later
- GIVEN the repository of *a TypeScript project* once it holds a `package.json`, and on `PATH` an `npm` that records its arguments and exits 0
- WHEN `clm setup` runs
- THEN it exits 0 and the stub recorded `ci`

#### Serves
- capability/adopt-clm

### Requirement: check-across-components
`clm check`, run as the `clm` binary, SHALL join what the other components give it. It SHALL report the findings of the ledger's audit (`ledger/verify`) when the checked-out branch is `main` and the repository has a ledger branch, and none of them on another branch. It SHALL run each detector `clm.toml` names as a child process, the project's own `clm unreferenced` and this repository's `scripts/retired-terms.sh` among them, and report one that fails as `check/detectors`, with what the detector printed on standard error. It SHALL accept a backlog item `clm defect` wrote. On this repository, `clm unreferenced` with the arguments of `clm.toml`'s `unreferenced-public-items` detector SHALL exit 0 (`config/unreferenced-detector`). Each scenario runs one component's command on what another made or offers, which only a `clm` process shows, so each SHALL be proven by running the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: check runs it on main
- GIVEN a repository on `main` with a ledger branch and the history of `ledger/verify`'s *push behind clm's back*, a `main` with a merge commit no `landed` record names
- WHEN `clm check` runs
- THEN it reports `ledger/unrecorded-land`

#### Scenario: named as a detector
- GIVEN a CLM fixture repository whose own checks pass, with a `clm.toml` detector `unreferenced-public-items` whose command is the `clm` binary under test with `unreferenced`, and a library with an unused `pub fn f`
- WHEN `clm check` runs
- THEN stdout contains `clm.toml: check/detectors: unreferenced-public-items failed (exit 1)` and stderr contains the line naming `f`

#### Scenario: a retired term in a spec fails the check
- GIVEN a fixture repository holding a copy of `scripts/retired-terms.sh`, a `clm.toml` naming only the `retired-terms` detector as this repository's names it, a `scripts/retired-terms.txt` holding `grommet stub`, and a `specs/` file reading `make it a grommet stub`
- WHEN `clm check` runs there
- THEN it prints that file, its line and `grommet stub`, and reports `clm.toml: check/detectors: retired-terms failed (exit 1)`

#### Scenario: clean with the list
- GIVEN this repository and the `clm` binary under test
- WHEN `clm unreferenced` runs with the arguments of the detector's command after `unreferenced`
- THEN it prints nothing and exits 0

#### Scenario: an item filed on a spec branch
- GIVEN a CLM repository with `defects = "backlog"` and a live change `20261022-x`, on branch `spec/20261022-x`
- WHEN `clm defect --title "Gate passes a stale task" --component gate` runs with a report on stdin, and then `clm check`
- THEN `clm check` accepts it

#### Scenario: an item for a component a live change creates
- GIVEN the same repository, with a live change whose delta for component `fresh` has `create: true`
- WHEN `clm defect --title "Fresh misfires" --component fresh` runs with a report on stdin, and then `clm check`
- THEN `clm check` accepts the item

#### Serves
- capability/catch-drift
- capability/no-dead-weight
- capability/report-defects
- capability/land-safely

### Requirement: land-as-a-process
`clm land`, run as the `clm` binary, SHALL run the gate and `clm check` from the copy of its own binary that `land/land-binary-copy` states and leave no copy behind, and two `clm land` processes of one clone SHALL never run their checks at once, whatever `CARGO_TARGET_DIR` each was given, the one that waits saying so, as `land/land-target-lock` states. `land` states both rules and proves what its crate's call can show; these scenarios need the binary's own file, or two processes at once, which only a `clm` process has, so each SHALL be proven by running the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: check deletes the binary
- GIVEN a `clm` binary copied to a scratch path, a repository whose `[land] check` deletes that scratch path, and `CLM_GATE` and `CLM_CHECK` unset
- WHEN the scratch `clm land` runs on a branch
- THEN it lands and prints `landed <sha> code`

#### Scenario: copy removed
- GIVEN an empty `TMPDIR` of the test's own
- WHEN `clm land` runs on a branch and lands
- THEN `TMPDIR` is empty again

#### Scenario: overlapping lands
- GIVEN two branches `impl/a` and `impl/b` of one clone, no `checked` records, and a `[land] check` that appends `start <branch>` to a log, sleeps 2 seconds and appends `end <branch>`
- WHEN `clm land impl/a` and `clm land impl/b` start at once, from two linked worktrees
- THEN in the log every `start <branch>` is followed by the `end` of that same branch before any other `start`, so the two checks never interleave, and the land that waited printed `waiting` naming `.clm/land` on stderr

#### Scenario: a caller's target takes the lock too
- GIVEN the same branches and check, and `CARGO_TARGET_DIR` set to a different directory of the caller's for each land
- WHEN `clm land impl/a` and `clm land impl/b` start at once
- THEN both land, the two checks never interleave in the log, and the land that waited printed `waiting` naming `.clm/land` on stderr

#### Scenario: held through the gate on the merge
- GIVEN the same two branches, a `[land] check` that appends `check <branch>` to a log and sleeps 2 seconds, and a fake `CLM_GATE` that appends `gate` to it and passes
- WHEN `clm land impl/a` and `clm land impl/b` start at once
- THEN the log reads `gate`, `check <one branch>`, `gate` and then `gate`, `check <the other>`, `gate`: both of a land's gate runs fall between its lock's taking and release, never inside the other land's

#### Serves
- capability/land-safely
- capability/work-concurrently

## MODIFIED Requirements



### Requirement: root-from-cwd <!-- base: 839330d917d4c5cf -->
Every `clm` command with a `--root` option SHALL, when it isn't given, use the first of the current directory and its parents that holds `specs/SPEC.md`, searching no further up than the first directory that holds `.git` (a directory or a file), and the current directory when there is none, except that `clm survey` and `clm unreferenced` SHALL then use the top of the git repository. An explicit `--root` SHALL be used as given.

#### Scenario: from a subdirectory
- GIVEN a CLM repository and the current directory `changes/` inside it
- WHEN `clm check` and `clm status` run without `--root`
- THEN each gives the same output and exit code as when run from the repository's top level

#### Scenario: from a worktree's subdirectory
- GIVEN a worktree of the repository on branch `impl/a` and the current directory `specs/` inside it
- WHEN `clm pending a` runs without `--root`
- THEN it reads the worktree's own files and reports the same as from the worktree's top level

#### Scenario: inside a larger repository
- GIVEN a git repository whose CLM repository is its subdirectory `tool/`, and the current directory `tool/changes/`
- WHEN `clm check` runs without `--root`
- THEN it checks `tool/`

#### Scenario: no CLM repository
- GIVEN a git repository with no `specs/SPEC.md` and the current directory `src/` inside it
- WHEN `clm check` runs without `--root`
- THEN it reports the same as `clm check --root .` run there

#### Scenario: survey with no CLM repository
- GIVEN a git repository with no `specs/SPEC.md` and the current directory `src/` inside it
- WHEN `clm survey` runs without `--root`
- THEN it prints the same as when run from the repository's top level

#### Scenario: from a subdirectory of a project without CLM
- GIVEN a git repository with no `specs/SPEC.md`, holding library `a` with an unused `pub fn f`
- WHEN `clm unreferenced` runs from `crates/a/src`
- THEN it prints the line for `a::f` with the path `crates/a/src/lib.rs`

#### Serves
- capability/see-project-state
- capability/adopt-clm
- capability/no-dead-weight

### Requirement: old-command-names-retired <!-- base: ad3289bebc7fe95f -->
The old names of the commands `renamed-commands` names, the subcommands `sync`, `checkpoint` and `page`, SHALL no longer run: `clm` given one of them SHALL fail as it fails for any unknown subcommand. `clm hash`, which `clm hashes` replaced (`apply/hash-command`), SHALL be no command either and SHALL fail the same way, and `clm --help` SHALL name `hashes` and no `hash`. `clm/retired-terms`'s list SHALL hold each old name written after `clm` and a space, and each outcome line that starts with an old name, so none comes back (`plain-terms`). No `clm:remove-by 20261101-rename-commands-contract` marker SHALL be left.

#### Scenario: an old name is refused
- GIVEN a branch with one commit on top of `origin/main`
- WHEN `clm` runs with the subcommand `sync`
- THEN it exits non-zero naming the unknown subcommand, and the branch is unchanged

#### Scenario: old name gone
- GIVEN a CLM repository with component `a`
- WHEN `clm hash a` runs
- THEN it exits non-zero, as for an unknown subcommand, and prints no hashes

#### Scenario: the old names retired
- GIVEN the repository
- WHEN the retired-terms test runs
- THEN its list holds the three old commands and the five old outcome lines, and it finds none of them

#### Needs
- orchestrator.checkpoint/checkpoint
- orchestrator.triage/draft
- orchestrator.triage/fold
- gate/up-to-date
- land/derived-specs
- land/land

### Requirement: doc-readers-marked <!-- base: 3125f2de9cbae5b9 -->
Each file of a crate under `crates/` that holds a test, an integration test under the crate's `tests/` or a unit-test module under its `src/`, that reads a file of this repository's own tree (not a fixture's) at a path `ledger`'s `doc_only` names, such as `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md`, a file under `specs/`, a page under `docs/guide/` or `docs/spec-format.md`, whether it opens the file or runs a command that reads it in this repository or prints it compiled in (`clm guide`), or that reads the repository's own `clm.toml`, SHALL hold a `clm:reads` marker (`check.affected/affected-readers`) whose glob matches every such path it reads, so a land that changes one of them runs it even when its code was already checked (`land/selected-tests`) or counts as checked because only `clm.toml`'s comments or upgrade notes changed (`land/settings-unchanged-is-checked`). A test that reads the guide's pages as a directory, such as the one that checks every page is in the compiled table, SHALL mark `docs/guide/**`. The rule is the root's, whatever crate holds the test, and its scenarios are proven by tests under `crates/clm/tests/` that run `clm-affected`'s `readers` on this repository's markers, since `clm` depends on every component and a crate below `check.affected` can't (`doc-readers-one-rule`).

#### Scenario: the retired-terms test reads the specs
- GIVEN the `clm:reads` markers of the files `check`'s coverage scan reads in this repository's tree
- WHEN `clm-affected`'s `readers` runs on them with `specs/SPEC.md` as the only differing path
- THEN it selects `crates/clm/tests/retired_terms.rs`

#### Scenario: the root documents' readers
- GIVEN the same markers
- WHEN `readers` runs on them with `ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md` and `README.md` as the differing paths
- THEN it selects `crates/clm/tests/retired_terms.rs` and `crates/clm/tests/readme.rs`

#### Scenario: other crates' readers of ARCHITECTURE.md
- GIVEN the same markers
- WHEN `readers` runs on them with `ARCHITECTURE.md` as the only differing path
- THEN it selects `crates/clm-init/tests/says_component.rs`, `crates/clm-model/tests/carried.rs` and `crates/clm-context/tests/carried.rs`

#### Scenario: the readers of clm.toml
- GIVEN the same markers
- WHEN `readers` runs on them with `clm.toml` as the only differing path
- THEN it selects `crates/clm/tests/cli.rs`, whose allow-list test reads `clm.toml`'s comments

#### Scenario: the guide's readers
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/guide/glossary.md` as the only differing path
- THEN it selects `crates/clm/tests/glossary.rs`, `crates/clm-init/tests/guide.rs` and `crates/clm/tests/retired_terms.rs`

#### Scenario: the guide's readers in other crates
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/guide/quickstart.md` as the only differing path
- THEN it selects `crates/clm-init/src/guide.rs`, whose tests compare each page's file with the compiled table, and `crates/clm-init/src/guide/entry_pages.rs`, whose test reads the quickstart

#### Scenario: the settings page's reader
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/guide/settings.md` as the only differing path
- THEN it selects `crates/clm-config/tests/settings_guide.rs`

#### Scenario: the spec format's readers
- GIVEN the same markers
- WHEN `readers` runs on them with `docs/spec-format.md` as the only differing path
- THEN it selects `crates/clm-init/tests/guide.rs`, `crates/clm/tests/retired_terms.rs` and `crates/clm-init/src/guide.rs`

#### Needs
- check.affected/affected-readers
- init/commands-in-crate

### Requirement: routes-only <!-- base: 851949a7fd2d69b6 -->
For every command a component offers as a call (`commands-in-crate`), `crates/clm` SHALL only parse the command line with the crate's own argument definitions, apply what holds for every command (`upgrade-blocks-commands`, `none-remote-before-commands`, `root-from-cwd`, `closed-stdout`), load the settings the crate is passed, and call the crate with the process's standard output, standard error and environment: it SHALL print nothing of its own for such a command and SHALL exit with the status the call returns. The top-level test files under `crates/clm/tests/` SHALL hold tests of the root's own requirements only, and the helpers no test there uses are removed with the tests that moved (`commands-live-in-their-crates`).

#### Scenario: a routed command
- GIVEN a fixture repository
- WHEN `clm hashes <component>` runs as a process there, and a test calls `clm-apply`'s call with the same arguments and two buffers
- THEN the process's standard output and standard error equal the buffers, and its exit status is what the call returned

#### Scenario: a status the call returns
- GIVEN a fixture repository whose check fails
- WHEN `clm check` runs as a process
- THEN it exits with the status `clm-check`'s call returns for that repository, and prints nothing the call didn't write

#### Scenario: the settings a routed command is passed
- GIVEN a fixture repository whose `clm.toml` sets `[affected]` `all = ["Cargo.lock"]`, on a branch that changes only `Cargo.lock`, and the same repository with `all = 3`
- WHEN `clm affected --base main` runs as a process in each
- THEN the first prints only `all Cargo.lock`, and the second exits non-zero with a line naming `clm.toml` and `check.affected/affected-all`, as `clm check` prints a violation

#### Scenario: every command routes
- GIVEN the commands `clm --help` lists
- WHEN each runs as a process with `--help`
- THEN each exits 0 and prints its own usage

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

#### Needs
- apply/commands-in-crate
- check/commands-in-crate
- check.affected/commands-in-crate
- check.health/commands-in-crate
- check.unreferenced/commands-in-crate
- config/commands-in-crate
- context/commands-in-crate
- effort/commands-in-crate
- gate/commands-in-crate
- host/commands-in-crate
- init/commands-in-crate
- land/commands-in-crate
- land.approval/commands-in-crate
- ledger/commands-in-crate
- orchestrator/commands-in-crate
- orchestrator.checkpoint/commands-in-crate
- orchestrator.run/commands-in-crate
- orchestrator.triage/commands-in-crate
- plugin.hooks/commands-in-crate
- plugin.hooks.guard/commands-in-crate
- rename/commands-in-crate
- sync/commands-in-crate
- view.repo/commands-in-crate

## REMOVED Requirements



### Requirement: rebase-warnings-prefixed <!-- base: 8240fcd818798e88 -->
