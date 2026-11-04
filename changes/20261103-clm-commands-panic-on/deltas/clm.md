---
component: clm
base: { design: "0159f7ae21ad7b62" }
---

## MODIFIED Design

The children split the work into deterministic core (`model`, `check` with its children `check.coverage`, `check.affected`, `check.unreferenced` and `check.health`, `apply`, `context`), coordination (`orchestrator` with its children `orchestrator.run`, `orchestrator.checkpoint`, `orchestrator.triage` and `orchestrator.runner`, and `sync` with its child `sync.regenerate`), records and landing (`ledger`, `gate` with its child `gate.fingerprint`, `land` with its child `land.approval`, and `rename`, whose replay the gate trusts), plumbing (`vcs`, `host`), presentation (`view`), integration (`plugin` with `plugin.hooks`, which holds `plugin.hooks.guard` and its `plugin.hooks.guard.shell`, and `plugin.panel`, `ci`, and `web` with `web.worker` (with its children `web.worker.platform` and `web.worker.review`), `web.client-live` and `web.live`), settings (`config`), adoption (`init`), and the capability layer's tests (`capability`). The core has no knowledge of Claude Code, of any host or of the web. The plugin has no logic that must hold, because such logic belongs in the CLI (`deterministic-enforcement`). `web` reuses the core, the queue, the views and the ledger compiled to WebAssembly, so it shows what `clm` would report, and it decides nothing. `crates/clm` only parses arguments and dispatches: `clm config` to `clm-config`, `clm defect` to `clm-hooks`, `clm init`, `clm survey` and `clm guide` to `clm-init`, `clm status` and `clm view` to `clm-view-repo`, `clm land`, `clm approve` and `clm reconnect` to `clm-land`, `clm ledger` to `clm-ledger`, `clm effort`, `clm capabilities` and `clm architecture` to `clm-effort`, `clm triage` and `clm draft` to `clm-triage`, `clm runner` to `clm-runner`, passing it its own executable as the `clm` it runs, `clm unreferenced` to `clm-unreferenced`, `clm rename` to `clm-rename`, `clm health` and `clm workaround` to `clm-health`, and `clm pr`, `clm branch` and `clm issue` to `clm-host`, passing each the settings it needs from `config`. Every command with a `--root` option defaults it to the CLM repository found from the current directory as `session-start` finds it: the first of the current directory and its parents that holds `specs/SPEC.md`, looking no further up than the first directory that holds `.git`. With none found, it is the current directory, except for `clm survey` and `clm unreferenced`, which then use the top of the git repository, so `/clm:init` maps the whole repository from any directory (`init-finds-its-repository`), and a project without CLM's specs gets its whole workspace checked. `clm init` takes no `--root`, but writes in the CLM repository found the same way, or at the top of the git repository when there is none (`init/init-root`). Running from a subdirectory then gives the same answer as running from the root. A linked worktree, such as `.clm/worktrees/<change>/<item>`, has its own `.git` file, so the search stays in the worktree. A CLM repository in a subdirectory of a larger git repository still works.

The repo declares its build environment once. `rust-toolchain.toml` pins the Rust toolchain to an exact version, with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target, and rustup installs it on first use on every machine and in every `clm run` worktree (`pinned-rust-toolchain`). Moving the pin is a code PR that fixes any new lints in the same diff. `.claude/settings.json` lists what unattended sessions may run beyond `clm run`'s own commands: `cargo`, a narrow set of npm commands that can't run a dependency's install script, and `.github/scripts/ci-check.sh`, for a session that runs the script itself (`unattended-commands`). A build runs exactly the checks `clm land` runs with `clm land --check-only` (`land-check-command`): Claude Code's worktree isolation refused the script's path in a build, and the `clm` command names none. The script installs Playwright's browser with `--with-deps` only when `CI` is `true` (`ci/browser-install`), so running it never reaches `sudo`. A test in `crates/clm/tests/` reads both files.

The root `.gitattributes` marks `Cargo.lock`, `web/package-lock.json` and `plugin-panel/package-lock.json` `linguist-generated` (`lockfiles-generated`), so neither counts toward its component's size (`check/node-size`): a session never reads a lockfile. They are written bare (`Cargo.lock linguist-generated`), which `git check-attr` reports as `set`. `crates/clm` also dispatches `clm size` to `clm-check`, and `clm affected` to `clm-affected` with `config`'s `[affected] all` globs, and loads `config`'s `component_size` and passes it to `clm size` and to `clm rebase`'s split-item step, as it passes `detectors` to `clm check`, and loads `config`'s `health` and passes it to `clm health` and to `clm rebase`'s reconsider step the same way (`check.health`).

The root's `paths` name `crates/clm`'s manifest, build script, sources and top-level test files, not the whole crate, so that `crates/clm/tests/capability/`, the capability tests' target, belongs to `capability` alone (`check/paths-disjoint`).

`crates/clm/src/main.rs` is dispatch only, and the build of whichever component gains or changes a command edits it, and `Cargo.toml` and `Cargo.lock` when the component adds a crate: the root's own builds cover only the root's requirements. Two builds in one wave may both touch those files; `clm rebase`'s rebase reports the conflict and the session resolves it.

**Running CLM's own repository on its own build** (`dogfood-from-main`, `dev-install-script`). `scripts/dev-install.sh` is a POSIX `sh` script for clones of CLM's own repository, not something CLM ships or `clm init` writes: it makes a clone's sessions run `main`'s `clm` and the checkout's plugin, where they would otherwise run the release. A human runs it once per clone; builds never run it (`no-shared-install`). It works on the clone's *main worktree*, the first worktree `git worktree list --porcelain` lists (as `config` finds it for `clm.local.toml`), whichever worktree it runs from, because Claude Code reads `.claude/settings.local.json` only there and the hook lives in the git directory every worktree shares. It merges JSON with `jq` (`reuse-first`); everything else is `git`, `cargo` and `sh`. The binary goes where `cargo install` puts it, `${CARGO_INSTALL_ROOT:-${CARGO_HOME:-$HOME/.cargo}}/bin/clm`, which becomes `CLM_BIN`. **The rebuild follows `origin/main`** (`rebuild-on-origin-main`). `main` moves through `clm land`, which pushes it from whichever worktree lands, so the main worktree rarely merges and a `post-merge` hook there misses most of what lands. Every way `main` moves on this clone moves `refs/remotes/origin/main` (a land's push updates it, as does any fetch or pull, a rebasing one included), and a ref update in any worktree runs the `reference-transaction` hook in the git directory they share. So the hook is a `reference-transaction` hook of a few lines: on the `committed` state, when a line of its input names `refs/remotes/origin/main`, it runs the tracked script with `--rebuild`; on anything else it exits at once, since git runs it for every ref update. A change to the rebuild reaches every clone with the next pull, without reinstalling the hook. The hook marks itself with the line `# clm dev-install`, which is how the script tells its own hook from someone else's. The rebuild builds `origin/main`'s commit, not the main worktree's: it keeps its own source, `clm-install-src` in the common git directory, a `git clone --shared --no-checkout` of the clone that borrows its objects, checked out detached at the commit to build, so it never touches a worktree a human or a session works in, and is no worktree of the clone, so nothing that lists worktrees sees it; `build.rs` reads the commit there, so `clm --version` names `origin/main`'s commit. It skips the build when the installed binary's `--version` names `origin/main`'s commit, or a commit from which nothing under `crates/`, `Cargo.toml` or `Cargo.lock` changed. One rebuild runs at a time, holding the lock directory `clm-install.lock` (with its process ID) in the common git directory; a rebuild that finds the lock held by a running process leaves it to that one, which reads `origin/main` again once it has installed and builds again when it moved, so a burst of lands ends with the binary at the last of them. `cargo install` replaces the binary by renaming, so a `clm` already running keeps its own file, and `clm land` runs from a copy of itself (`land/land-binary-copy`). Every setup run deletes a `post-merge` hook holding `# clm dev-install`, which earlier runs wrote.

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

### Requirement: closed-stdout
When the reader of `clm`'s stdout has gone, a print to it SHALL end the command with a non-zero exit status and no panic text on stderr. A print to stdout that fails for any other reason SHALL stay a panic that names the reason.

#### Scenario: reader gone before the first print
- GIVEN a CLM repository with component `a`, and a stdout that is a pipe whose reading end is closed
- WHEN `clm context a` runs
- THEN it exits 141, and its stderr is empty

#### Scenario: a view
- GIVEN a CLM repository, and a stdout that is a pipe whose reading end is closed
- WHEN `clm status` runs
- THEN it exits non-zero, and its stderr holds no `panicked`

#### Scenario: a full disk still says why
- GIVEN a CLM repository with component `a`, on Linux, and stdout opened on `/dev/full`
- WHEN `clm context a` runs
- THEN it exits 101, and its stderr holds `failed printing to stdout` and `No space left on device`

#### Serves
- capability/see-project-state
- capability/build-from-spec
