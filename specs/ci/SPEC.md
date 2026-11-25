---
title: CI
status: active
owners: ["@clunny"]
depends_on: [check, check.affected, vcs, land, host]
paths: [".github/**", ".config/nextest.toml", "crates/clm/tests/ci_suite/**"]
successor: ""
serves: [capability/land-safely, architecture/land-locally]
---

## Contract
What this repository runs on GitHub Actions, and its check, coverage and mutation scripts (`land-locally`, `host-ci-for-releases`): a release workflow a human dispatches (bump the version, build prebuilt `clm` binaries, publish them as a GitHub Release and deploy the `web` app), a site workflow a human dispatches (build and deploy the public site, `site-worker-of-its-own`), and three scripts under `.github/scripts/`, each run from the repository's root. GitHub runs no workflow on a pull request or a push: checks run and branches land through `clm land`, on the machine that runs it. Neither `ci-check.sh` nor a workflow runs `coverage.sh` or `mutate.sh` as a command of its own.

- `ci-check.sh`, the one command that runs every check of this repository, which `clm.toml` names as `[land] check` and `[verify] check`. It reads `CLM_SELECTION`, a file whose `file <path>` lines select the tests to run, every test when it is unset or empty, names no regular file, or names one that starts with an `all` line (`selected-tests`); `CLM_LAND_DIR`, the directory a land keeps its cargo output in (`land-dir`); and `CI`, set to `true` only on a hosted runner (`browser-install`). For each phase it prints `::group::<phase>`, a `::clm-selected::<path>` line for each selected file that turned the phase on, and `::endgroup::` on standard output (`step-markers`), and `ci-check: <phase> <n>s` on standard error; on exit, `ci-check: phases`, a `<n>s <phase>` line per phase and `<n>s total` when it passes, or only `ci-check: failed in <phase> after <n>s, <n>s total` when it fails (`phase-times`). It exits 0 when every check passes, and otherwise with the status of the command that failed.
- `coverage.sh` writes `target/lcov.info`, the lcov file `clm health`'s `unexercised` signal reads (`lcov-coverage-signal`, `coverage-by-script`, `coverage-script`). A person or a session runs it by hand when they want that signal fresh, and the mutation pass at its end.
- `mutate.sh`, the command of this repository's mutation pass, which `clm.toml` names as `[verify] mutate` (`mutate-script`). It reads `CLM_MUTATE`, the plan `clm verify` writes; `CLM_MUTATE_RESULT`, the file it writes one verdict line to for each block, `<id> killed`, `<id> survived <n>` or `<id> not-judged`, as soon as the block is judged; `CLM_MUTATE_JOBS`, the most blocks it judges at once; `CLM_SLOT`, the slot its first job takes; and `CLM_LAND_DIR`, where it leaves the tracefile of the `coverage.sh` it runs after the blocks. It exits 0 whatever the verdicts, and non-zero, writing no verdict for the block it failed in, when a tool fails.

## Design
**The check script.** `.github/scripts/ci-check.sh` runs fmt, clippy, tests, then the `web` checks: the wasm builds and the `web/` and `plugin-panel/` suites (`web-checks`). The panel's `npm test` type-checks `plugin-panel/` with `tsc` first, against the panel's own declarations of the plugin API (`plugin.panel/type-check`, `panel-types-hand-written`), so a type error in the panel fails the check as a failing test does; `tsc` and `claude plugin validate` catch different faults, and neither replaces the other. After the panel's tests it runs `claude plugin validate plugin-panel`, which reads the panel's manifest, hooks module and type contract as Claude Code loads them, so a module Claude Code would refuse to load fails the check instead of landing (`panel-validate`). It runs with neither `--strict` nor `--profile`: only what the engine would refuse fails it, not warnings or the plugin directory's listing checks (a LICENSE, an icon); the panel's own tests strip types and never load the module as the engine does. Without `claude` on `PATH`, as for a person without Claude Code, the validation is skipped (`panel-validate-skips-without-claude`). The `clm` plugin, `plugin/`, is validated the same way, `claude plugin validate plugin` with the same flags and the same skip, whenever every test is selected or the selection names a file under `plugin/` (`plugin-validate`, `plugin-validate-at-land`): its manifest, hooks, skills, agents and command are read by the same loader, and a frontmatter or hook Claude Code refuses would otherwise land on its tests' word lists alone. The validator is whatever `claude` the landing machine has, unpinned: that is the Claude Code that loads the plugin there, so its verdict is the one that matters, and a refusal after an auto-update is a real finding about the plugin, fixed as code; a land that touches nothing under `plugin/` or `plugin-panel/` never runs either validation, so an update can't fail a land of unrelated code. It doesn't run `clm check`: `clm land` runs that itself after the command, and a session runs it on its own. It first runs `.github/scripts/detector-tools.sh`, which installs the tools behind `clm.toml`'s detectors (only `cargo-machete`, with `cargo install --locked --version <x.y.z> cargo-machete`, the newest release at build time), because `this_repository_is_clean` runs `clm check`, which runs the detectors; `~/.cargo/bin` keeps the install from repeating. The step that runs `web/`'s browser tests first installs Playwright's Chromium, in `web/` (`browser-install`). The install is idempotent: it fetches only the Chromium revision `web/`'s `@playwright/test` pins, into Playwright's per-machine cache, and does nothing when that revision is already there, so a land pays for a download only when the pin moves. Without `--with-deps` it needs no `sudo`, so `clm land` and an unattended session run it; Chromium's system libraries are a once-per-machine step a human takes (`npx playwright install-deps chromium`), and only `--with-deps`, which a hosted runner that sets `CI` would need, reaches `sudo`. No workflow runs the script and `clm land` unsets `CI`, so only a human who sets it, or a hosted runner, reaches that path. Installing on every run keeps a cached browser one revision behind the pin from failing every `web` e2e test with `Executable doesn't exist` and no hint of the fix. npm runs no dependency's install script (`unattended-commands`). `.claude/settings.json` allows the script by this path (`clm/session-commands`).

**Selected tests** (`selected-tests`, `graph-test-selection`). When `CLM_SELECTION` names a file, as `clm land` sets it, the script runs only the tests that file's `file` lines call for; fmt, clippy, the detector tools and the wasm builds always run, since they check the whole tree and are quick beside the tests. When the variable is unset or empty, or the file's first line starts `all`, it runs every test (`cargo nextest run --workspace` and `cargo test --doc --workspace`, both npm suites and both capability packages): a session, the release's `prepare` job and a human running it by hand get the full suite. Lines other than `file <path>` (`check.affected`'s component lines, or anything unexpected) select nothing. The selection maps paths to targets as `selected-tests` says. The package of `crates/<dir>/` is the first `name = "<package>"` line after the `[package]` line of its `Cargo.toml`; a `src/` file selects `--lib` when the package has `src/lib.rs` and `--bins` when it has `src/main.rs` or `src/bin/`; a selected package with no tests is fine: every run passes `--no-tests=pass`. A file under `plugin-panel/` also selects `claude plugin validate plugin-panel`, a file under `plugin/` selects `claude plugin validate plugin`, and any other file selects nothing. `web/site/`'s suite (`web.site`) runs after `web/`'s only when every test runs or the selection names a file under `web/site/` (`web-checks`): `web` depends on `web.site`, not the other way round, so nothing else under `web/` changes what the site's tests check. `capabilities/web/` still runs whenever `web/`'s suite does, since it drives the Worker as deployed and a Worker change can break it.

It runs one `cargo nextest run --no-tests=pass -p <package> <targets>` per package, in package order, so a `--test` flag never applies to a package that lacks the target, and none when no package is selected. A selected run leaves out doctests, as `nextest` never runs them. A file under `crates/<dir>/` with no `Cargo.toml` (a deleted crate) selects nothing. The script reads the file with shell builtins only, so the selection costs no build. They run in every full run, through `cargo test --doc`, and this repository has few. `this_repository_is_clean` cites the root's requirements, which nearly every change affects, and `clm land` runs `clm check` after the command anyway.

**Capability packages** (`web-checks`, `capability-browser-packages`). `capabilities/web/` and `capabilities/panel/` are npm packages of `capability`'s own, holding the capability tests a person reaches through a browser or a pane (`capability`). Neither depends on `web` or `plugin.panel` in the spec graph, so the selection reaches them only through their own files, and the script adds them where they test what changed: it runs `capabilities/web/` whenever it runs `web/`'s suite, and `capabilities/panel/` whenever it runs `plugin-panel/`'s. Running `capabilities/web/` is, in `capabilities/web/` after `web/`'s `npm ci` (its tests build the app with `web/`'s own dependencies): `npm ci --ignore-scripts`, the Playwright Chromium install `browser-install` describes, and `npm test -- --retries=1` (`capability-web-retries`), which Playwright reads as its `--retries=1`. A test that fails runs once more from the start, in a fresh browser context, and the package fails only when a test fails on both attempts; Playwright's output names a test that passed only on its retry `flaky`, with the first attempt's error. The machine that runs the script runs other builds' compiles and suites at the same time (`build-places`), and each of the package's tests starts its own Worker in Miniflare and lands branches with `clm land`, so under that load a test can spend its 30 s assertion timeout on a step that takes a second on an idle machine and pass when run again; the retry keeps such a failure from costing a whole land's check. The retry is the script's, not the package's config, so a person or a build that runs `npm test` in the package by hand sees every failure. `web/`'s suite and `capabilities/panel/` run with no retry: a failure under load there is fixed at its cause. Running `capabilities/panel/` is `npm ci --ignore-scripts` and `npm test` there, after `claude plugin validate plugin-panel`. Each runs after the suite it follows. Both versions of `@playwright/test` are pinned alike, so the second install fetches nothing.

**No evals** (`cut-evals`, `no-model-steps`). The script runs no model session and no `clm` command. `claude plugin validate`, which loads the plugins and runs no model, runs.

Whether the command runs at all, and for which tests, is `clm land`'s to decide (`land/selected-tests`, `doc-readers-marked`). There is no other doc-only shortcut, prebuilt binary or warm cache: there is no runner to keep warm, and the cost of a check is this machine's time.

**Coverage** (`coverage-script`, `coverage-by-script`). `.github/scripts/coverage.sh` writes `target/lcov.info`, one lcov tracefile for the whole repository. Nothing in `clm land`, `ci-check.sh` or the release workflow runs it: an instrumented run rebuilds every crate in a target directory of its own and runs the full suite, which would double a land's time for a signal that is advisory (`health-advises`). The mutation pass runs it after the plan (`mutate-script`), in the verify tree, and copies the tracefile to `$CLM_LAND_DIR/lcov.info`, which is `.clm/verify-build/lcov.info` under the clone's main checkout and what `clm.toml` names under `[health] lcov` (`config/health-lcov-here`); run by hand from a checkout's root, it leaves the tracefile in that checkout's `target/`, to be copied there by hand. It installs what it needs the way `detector-tools.sh` does: it runs `.github/scripts/detector-tools.sh` first, since the suite's `this_repository_is_clean` runs `clm check`; then `.github/scripts/test-tools.sh`, since the Rust tests run under `cargo-nextest` here as everywhere (`nextest-runner`); then `rustup component add llvm-tools-preview`, which `cargo llvm-cov` needs and `rust-toolchain.toml` doesn't list, a no-op once the component is there; then `cargo install --locked --version <x.y.z> cargo-llvm-cov`, the newest release at build time, which `~/.cargo/bin` keeps from repeating. Then, in order:
- `mkdir -p target/coverage`, then `cargo llvm-cov nextest --workspace --no-rustc-wrapper --lcov --output-path target/coverage/cargo.info`: `cargo llvm-cov`'s `nextest` subcommand runs the tests under `cargo-nextest`, one process per test, as the land check does. Plain `cargo llvm-cov` runs them through `cargo test`, each test binary's tests on threads of one process, which once hung a verify for over an hour and a half (`one-runner-for-the-rust-tests` records the run): tests that put a fake `git` on their own process's `PATH`, which `commands-live-in-their-crates` takes as safe because they run one per process, ran side by side, stacked each other's directories on the one `PATH`, and two fakes, each finding the real `git` by stripping only its own directory, ran each other without end. The run covers no doctest, as before. It builds in `target/llvm-cov-target/`, so the ordinary build stays warm; `cargo llvm-cov clean --workspace` frees that directory. `cargo llvm-cov` writes `SF:` paths that are absolute under the tree it ran in; the script strips that tree's root from each, as `$PWD` gives it and as `pwd -P` does, since the tool writes real paths and a checkout may be reached through a symbolic link, so the tracefile reads the same in whichever checkout `clm health` runs: the signal strips only the root of its own checkout, and a path left absolute under the verify tree matches no component's files there and is dropped without a word. `--no-rustc-wrapper` makes `cargo llvm-cov` pass its instrumentation in `RUSTFLAGS` rather than through a `RUSTC_WRAPPER` and its `__CARGO_LLVM_COV_*` variables. The suite's tests that build a crate for `wasm32-unknown-unknown` run a nested `cargo build` that inherits the test's environment, and they already remove `RUSTFLAGS` and `CARGO_ENCODED_RUSTFLAGS`, since coverage flags break a wasm build (`E0463: can't find crate for profiler_builtins`). With the wrapper, which `cargo llvm-cov` 0.9 uses by default, the instrumentation reached those nested builds anyway. The flag's cost: the dependencies are instrumented too, so the run rebuilds more. The report still covers only the workspace's crates. `--ignore-run-fail` would write a tracefile even when a test fails, so the script doesn't use it.
- In `web/` and then `plugin-panel/`: `npm ci --ignore-scripts`; `@vitest/coverage-v8` installed at the version of the `vitest` that installed, read from `npx vitest --version` (`vitest/<x.y.z>`), with `npm install --no-save --ignore-scripts @vitest/coverage-v8@<x.y.z>`, which adds it to `node_modules/` and leaves `package.json` and the lockfile as they are, so neither directory's component carries a dependency only this script uses; then `npx vitest run` with `--coverage.enabled`, `--coverage.provider=v8`, `--coverage.reporter=lcovonly`, `--coverage.reportsDirectory=../target/coverage/<dir>` and `--coverage.include` naming the directory's sources (`web/`'s `src/**`; `plugin-panel/`'s `src/**` and `hooks/**/*.tsx`, not `hooks.json`), one flag per glob, so a file no test loads is reported with every line at 0 rather than left out. The reporter writes `lcov.info` there with `SF:` paths relative to the directory; the script prefixes each relative one with `web/` or `plugin-panel/`, and strips the root from an absolute one. `web/`'s Playwright suite runs no part in it: browser coverage would need another instrumentation, and the unit suites cover the Worker's and the app's modules.
- The three tracefiles concatenated, in that order, as `target/lcov.info`: lcov is a sequence of records, so concatenation is a merge.

`set -euo pipefail`, so a suite or install that fails stops the script with a non-zero exit and leaves the previous `target/lcov.info` as it was until the concatenation. Everything it writes is under `target/`, which `.gitignore` already holds. It is proven as `ci-check.sh` is, run in a temporary tree with fakes for `cargo`, `rustup`, `npm` and `npx` first on `PATH` that record their arguments, the `cargo` and `npx` fakes writing a tracefile where the script told them to; the fakes never need the real tools. npm runs no dependency's install script (`unattended-commands`). `.claude/settings.json` allows neither `rustup` nor `npx vitest` (`clm/session-commands`), so a session that runs the script meets Claude Code's permission prompt, which is right for a by-hand step, and a build never runs it: the build proves the script on the fakes.

**Toolchain** (`pinned-rust-toolchain`). The release workflow installs the toolchain `rust-toolchain.toml` names with `rustup show`, rather than `dtolnay/rust-toolchain@stable`. The file pins an exact version with `rustfmt`, `clippy` and the `wasm32-unknown-unknown` target, so the release builds with the same compiler as a local session. The release `build` job adds its matrix target on top with `rustup target add <target>`.

**The release workflow.** `.github/workflows/release.yml` runs only on `workflow_dispatch` (`release-by-dispatch`), with one required input, `version`. It has `permissions: contents: write` and `concurrency: { group: release, cancel-in-progress: false }`, so two releases never run at once. It and the site workflow are the only jobs on a hosted runner (`host-ci-for-releases`, `site-worker-of-its-own`); each runs a few times a week at most. Every job runs on `ubuntu-latest`: no job needs a macOS runner (`cross-built-macos-binaries`). The `build` matrix lists only `target`, and the job's `runs-on` is `ubuntu-latest`, since no column would differ between rows. Its jobs run in order, each needing the one before:
1. `prepare` runs `.github/scripts/release-prepare.sh <version>`. The logic lives in a script so that it can be run against a temporary repository whose `origin` is a local bare repository, with a fake `gh` on `PATH`. The script fails unless the `release` preconditions hold. Among them, every target `plugin/bin/clm` maps a platform to is a `target:` in the build matrix of `.github/workflows/release.yml` (`shim-assets-before-release`): the script reads both files as text, the shim's `target=<target>` assignments and the matrix's `target: <target>` lines, so a shim that would ask for an asset the release doesn't hold is found before anything is built.

   When `version` is higher than the workspace version, the script:
   - sets `[workspace.package] version` in `Cargo.toml`, and the `version` in `plugin/.claude-plugin/plugin.json`, in `plugin/.codex-plugin/plugin.json` and in `plugin-panel/.claude-plugin/plugin.json`, so `claude plugin update` delivers both Claude Code plugins (`plugin.panel/panel-package`) and Codex's plugin update delivers the Codex package (`plugin/codex-package`), and stages all three with the bump;
   - updates the workspace's entries in `Cargo.lock` (`cargo update --workspace`);
   - runs the Rust tests as `nextest-runner` says, `cargo nextest run --workspace --no-tests=pass` and then `cargo test --doc --workspace`, with the tools the job installed in its steps before the script, `.github/scripts/detector-tools.sh` and then `.github/scripts/test-tools.sh`;
   - builds `clm` from the bumped tree, checks with it that no requirement of `upgrade` or `init` is pending, and adds the release's upgrade corpus entry and lock lines to the bump (`release-records-upgrade-corpus`);
   - commits `Release v<version>` as `github-actions[bot]` and force-pushes it to the branch `release/v<version>`, replacing whatever an earlier failed run left there.

   Its output `sha` is that commit, or `github.sha` when `version` equals the workspace version.
2. `build` checks out `sha` and builds `clm` with `--release --locked -p clm` for each target on `ubuntu-latest`, with `cargo zigbuild` (`static-linux-binaries`, `cross-built-macos-binaries`). Zig is the C compiler and linker for every target, so one mechanism builds all four with nothing from `apt` and no macOS SDK. The job installs Zig from PyPI (`pip install ziglang==<x.y.z>`) and `cargo-zigbuild` with `cargo install --locked --version <x.y.z> cargo-zigbuild`, each pinned to its newest release at build time; no step builds with plain `cargo build`.
   - `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl`: after building, the step fails unless `file` reports the binary `statically linked` or `static-pie linked`;
   - `aarch64-apple-darwin` and `x86_64-apple-darwin`: cross-built with `MACOSX_DEPLOYMENT_TARGET` set to `11.0` for both. That is the oldest macOS Rust supports on `aarch64` and the oldest Zig 0.17.0 targets; Claude Code, which CLM runs in, needs a newer macOS than that, so no Mac that can run CLM is left out. Without it, Zig 0.17.0 marks the binary as needing macOS 15.0. Zig's linker gives the `aarch64` binary the ad-hoc signature Apple Silicon requires before it runs anything; an `x86_64` binary needs none. The build warns that it finds no `xcrun` or macOS SDK, which it doesn't need: `clm` links only `libSystem`, `libiconv` and `libcharset`, which Zig carries stubs for, and no Apple framework.

   `clm survey`'s tree-sitter grammars are C, which the `cc` crate compiles at build time (`tree-sitter-analysis`), so every target needs a C compiler for it: Zig, for each. The binaries are self-contained, so no one who installs `clm` needs a compiler. Each target packs `clm-<target>.tar.gz`, containing just `clm`, and `clm-<target>.tar.gz.sha256`, written by `sha256sum`, and passes them on as a workflow artifact.
3. `publish` runs `gh release create v<version> --target <sha> --generate-notes` with every asset, so the release is published with all its binaries at once. Then, when there was a bump, it merges `release/v<version>` into `main` and pushes `main`, then deletes the branch. The push is a fast-forward when `main` hasn't moved and a merge commit otherwise, and it retries if the push races. Only the release workflow changes the version, so the merge can't conflict. If the push still fails, the release is out but `main` names the old version, and a human runs the `git` commands the job prints (`release`). This push is the one push to `main` that isn't a `clm land`; `clm ledger verify` knows a release merge by its `Release v` message and doesn't report it, so `main` still audits clean.
4. `web` (`web-deploy`) builds `web/` and deploys the app's Worker and its assets with `wrangler deploy`, using the repository secrets `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID`. It runs after `publish`, so a failed deploy doesn't undo the release. It can be rerun on its own. It doesn't build or deploy the site.

A run that fails before `publish` publishes nothing and leaves `main` as it was, so it can be dispatched again. Events made with the workflow token start no workflows. That's why one dispatched workflow does every step. Deploying `web` on release, not on merge, matches how the plugin ships. With every target built on Linux, the release could run on this machine instead of Actions; it stays on Actions until `host-ci-for-releases` is revisited.

**The site workflow** (`site-deploy`, `site-worker-of-its-own`). `.github/workflows/site.yml` runs only on `workflow_dispatch`, with no inputs, from `main`. It has `permissions: contents: read` and `concurrency: { group: site, cancel-in-progress: false }`. One job checks out `main`, runs `npm ci --ignore-scripts` and `npm run build` in `web/site/` with `UMAMI_WEBSITE_ID` from the Actions variable and `GITHUB_TOKEN` the workflow's own, then `npx wrangler deploy` in `web/site/`, `wrangler` being a dev dependency of the site's own package (`web.site`), with the same two Cloudflare secrets. Publishing a post is setting its `draft` to `false` on a branch, landing it, and dispatching this workflow; a CLM release never publishes the site, and this workflow never touches the app.

The Linux binaries are static `-musl` builds, so they run on any Linux of their architecture, whatever its glibc. There is no `-gnu` asset: a plugin version's shim downloads only its own version's release, so a shim that names a `-gnu` asset only ever reads a release that has one. There is no Windows asset (`no-windows-support`): the shim maps no Windows platform, so no shim of this version or a later one asks for one, and a plugin version that still maps one reads only its own version's release.

The code key lives in `ledger`, landing in `land` and approval pinning in the ledger's fingerprints.

**Running the tests** (`nextest-runner`, `test-tools`, `tests-time-bounded`, `one-runner-for-the-rust-tests`). The Rust tests run under `cargo-nextest`, which runs a package's test binaries at once, one process per test, where `cargo test` runs the binaries one after another, each binary's tests on threads of one process. Every test of this repository must therefore pass while the others run beside it on one host, which `tests-run-concurrently` asks of every project, and a test may change its own process's environment or directory for the length of a call, since no other test shares the process (`commands-live-in-their-crates`). That holds only where nextest runs the tests, so every run of this repository's Rust tests other than its doctests goes through it: `ci-check.sh`'s, `coverage.sh`'s through `cargo llvm-cov nextest`, `mutate.sh`'s for each mutant, and the release's `prepare` job's; no script or workflow runs them with plain `cargo test`. `nextest` doesn't run doctests, so a full run follows it with `cargo test --doc --workspace`. `.github/scripts/test-tools.sh` installs `cargo-nextest` (`cargo install --locked --version <x.y.z> cargo-nextest`, the newest release at build time, which `~/.cargo/bin` keeps from repeating). `ci-check.sh` and `coverage.sh` run it after `detector-tools.sh` and before their first `cargo` line, and the release's `prepare` job in a step of its own after the `detector-tools.sh` step and before `release-prepare.sh`, which costs the release a few minutes to build `nextest` on its hosted runner. It is a script of its own so that `detector-tools.sh` keeps installing only the detectors' tools. `mutate.sh` installs nothing for nextest: it runs after `[verify] check`, whose `ci-check.sh` has installed it (`mutate-script`).

A test that hangs fails instead of holding the run for ever (`tests-time-bounded`). `.config/nextest.toml`, the file nextest reads from the workspace root wherever it runs, sets the default profile's `slow-timeout` to `{ period = "60s", terminate-after = 15 }`: nextest reports a test still running after 60 seconds as slow and ends it after fifteen such periods, 15 minutes, counting it failed. `.config/nextest.toml` is one of this repository's own settings files, which only `ci`'s build sets, so `tests-time-bounded` states its value, as `config`'s requirements state this repository's `clm.toml`: the third case in which a requirement about a file's value stays (`requirements-state-what-the-contract-shows`). The land check reads the value, and nothing runs the real `nextest` against a test that hangs (`land-checks-stay-fast`): a test that waits out a bound is slow and flaky under load, and a verify or a mutation pass that meets a hung test shows the bound working. The bound reaches every run above, the copies `cargo mutants` tests in and a session's own `cargo nextest run`, with nothing on any command line. 15 minutes is meant to lie far past the slowest test of the suite, even instrumented and on a loaded machine, so that the bound ends only a test that will not end by itself. A test that needs longer is a test to make faster, not a reason to raise the bound. A bound on the whole check or mutation command was not taken, as for npm installs (`npm-installs-bounded`): it must be loose enough for the slowest whole run and names no test, where nextest's names the test that hung. The few doctests run under `cargo test --doc` stay unbounded.

**Phase times** (`phase-times`). The script times each phase it runs, by the shell's `SECONDS`: the tool installs (one phase, `tools`), fmt, clippy, the wasm builds, the Rust tests, and each npm suite, capability package and plugin validation. After a phase it prints `ci-check: <phase> <n>s` to stderr. An `EXIT` trap, which keeps the script's exit status, prints on a pass `ci-check: phases`, one `<n>s <phase>` line per phase and `<n>s total`, and on a failure only `ci-check: failed in <phase> after <n>s, <n>s total`: `clm land` shows the last 40 lines of a failed check (`land/land-check-log`), and a summary after the failure would push the failure's own lines out of them. Each phase is also a step `clm land` times (`step-markers`, `land/step-times`): the script prints `::group::<phase>` on stdout as the phase starts and `::endgroup::` as it ends, and, in a selected run, `::clm-selected::<path>` inside a phase for each selected file that made the script run it: for the Rust tests, each `file` line that selected a package; for a suite, capability package or validation, the selected files under the directory that turned it on, which for `web/site/` are its own selected files, and for `capabilities/web/` the files that turned `web/` on and its own selected files. The phases every run has (tools, fmt, clippy, wasm) and a full run name none: nothing selected them.

**Mutation** (`mutate-script`, `mutation-after-land`). `.github/scripts/mutate.sh` is this repository's `[verify] mutate` command. Its tests put fakes of `cargo` and `npx` on `PATH`, so they prove its plumbing and say nothing of whether the real tools accept what it gives them; three things cover that (`mutation-after-land` records the run that showed the need). The filter names a test target as nextest's `binary_id(=<package>::<target>)`. A tool that fails for any reason but its mutants' outcomes fails the script, so `clm verify` records nothing and the requirements stay due, since a `not-judged` on record is never looked at again while the test is unchanged. And before the plan the script judges a fixture of its own with each tool the plan needs: a function with one test that notices a change to it and one that doesn't, written to a temporary directory for `cargo-mutants`, and for StrykerJS inside the npm directory it is about to judge, so the fixture meets that directory's own vitest and TypeScript. A wrong verdict on the fixture fails the script before it judges anything. The fixture is written by the script and tracked nowhere, so it adds no `package.json` or `Cargo.toml` to the repository. Nothing in the land check runs the real tools (`land-checks-stay-fast`): the fakes prove the script's plumbing at the land, and the fixture proves the tools at each pass. StrykerJS 10.1.1 calls `ts.parseConfigFileTextToJson`, which TypeScript 7 doesn't have, when it finds a `tsconfig.json`; the build finds the configuration under which it judges the fixture, by a StrykerJS option or by moving the script's pinned StrykerJS version, never by changing the TypeScript or vitest an npm directory installs, which are `web`'s and `plugin.panel`'s, and leaves the requirement unbuilt with a question item if there is none. StrykerJS mutates no vitest test file, one whose name ends in `.test.ts` or `.test.tsx`, though a component owns its test files and the plan names them on `code` lines: StrykerJS instruments each file it mutates, and in a test file that puts a `vi.mock` call, which vitest hoists and accepts only at the module's top level, inside an expression, so the file fails vitest's first run and every requirement its tests cite is left unjudged (`mutation-after-land`); and a mutant of a test's own helper is caught however little the test asserts, which would hide the code's survivors, as the Rust path's test-file mutants are kept apart for. Each tool keeps its own default timeout for a mutant's tests, which counts a mutant that hangs as caught; nextest's bound on a test (`tests-time-bounded`) holds in each copy, and the coverage run after the plan is bounded the same way; the script sets no other bound but the limit on its npm installs (`npm-installs-bounded`), and the pass holds no land (`mutation-after-land`).

**A rule in a test file** (`mutate-script`). `cargo mutants` mutates the files of a crate's library and binary targets and never a test target's, so a requirement whose rule is code in its test's own file had no mutant its tests could notice. `vcs/one-git-layer`'s rule is the scan of every crate in `crates/clm-git/tests/scan.rs`: the pass once filed it unproven after trying mutants of `crates/clm-git/src/`, which its tests never run, though a pass by hand of the scan's 21 mutants killed every one (`mutation-after-land` records the case). The `clm:reads` markers its build then added would have left it `not-judged`, so a later weakening of the scan's tests would go unseen. So when a block's code files offer no mutant its tests run, the script judges it by the mutants of its tests' own files under a crate's `tests/`. Only then: a test file's functions include the helpers that build its fixtures, and a mutant of one is caught however little the test asserts about the rule, so tried beside the code's mutants it would turn their survivors into `killed`. `cargo mutants --Zmutate-file <file> --json`, at the pinned version, lists a file's mutants as `--list --json` does, with each one's span, replacement and diff, and skips `#[test]` functions as the tool always does, so a test's own assertions are never mutated and an empty test runs no line a listed mutant changes. The option is one the tool marks unstable, for debugging its mutant generation, which is why the Rust fixture holds a test file of its own: a version whose listing changes fails the fixture before any block. `cargo llvm-cov` leaves files under `tests/` out of its report unless given `--no-default-ignore-filename-regex`, which the run that finds the lines passes; the report then names the dependencies' files too, and the script reads only the block's. The JSON names a mutant's file by its base name only, so the script keys each mutant by the path it gave, the tree's copy of the file, whose spans then match the copy it mutates; it writes a mutant by applying its `diff` to that file with `patch`, since the span's columns count characters, not bytes. A test that runs another component's code as well is left to the code path, where it reads as before: it may be the misplaced test the pass should report, and its file's helpers would catch mutants however little it asserts. The test-file path sits within the Rust path, so it judges only a block whose `code` lines name Rust files. The script writes each mutant into a copy of the tree it makes the first time a job needs one, one per job number in its temporary directory, so blocks at once and `cargo mutants`' own copies never see it, and the copy's index marks every tracked file unchanged as the tree's does, so git there reports nothing changed. It runs the block's tests there with `cargo nextest run --no-tests=pass` over the block's test packages, under its filterset and its job's `CARGO_TARGET_DIR`, and puts the file back. nextest's exit status says the outcome: 0 every test passed, 100 one failed, nextest's bound on a test (`tests-time-bounded`) ending a hung one as failed, 101 the build failed, which `cargo mutants` calls unviable and counts neither way; any other status fails the script, after the file is put back. It tries no mutant after the first caught, since each try rebuilds the block's test packages. The run that finds the lines is the baseline: it fails when the block's tests fail unmutated, so a mutant is never counted caught by a test that fails anyway.

**Bounded npm installs** (`npm-installs-bounded`). On 2026-11-15 an `npm ci` in the verify tree's coverage pass sat for over 11 minutes on a connection to the registry that had stalled silently, until a person killed it; nothing else would have ended it, and the build loop waited on that verify. The same stall in `ci-check.sh` would hold the clone's check lock and every land behind it. So the three scripts run every `npm ci` and `npm install` through `.github/scripts/npm-install.sh`, which each finds beside itself: it runs `npm` with the arguments it was given in the directory it was called in, ends it at a limit (300 seconds, or the whole seconds `NPM_INSTALL_TIMEOUT` names, which the tests set to 5, against a fake that stalls for 600, to prove the limit quickly: only a stalled run is timed closely, and a healthy fake has 5 seconds to exit on a loaded machine, `land-checks-stay-fast`) with `SIGTERM` and, 10 seconds later, `SIGKILL`, says so on stderr, and runs it once more. A stall is the network's fault and both commands are safe to run again (`npm ci` replaces `node_modules/`, `npm install --no-save` adds to it), so one retry makes the stall a passing run a few minutes late; a second run ended at the limit fails with status 124, the status coreutils `timeout` gives, and a run that fails by itself is never retried, since a broken lockfile fails the same way twice. With `--ignore-scripts` an install runs no package script, so ending the `npm` process ends the install. Not taken: npm's `fetch-timeout` and `fetch-retries`, whose defaults were in force during the stall and which time one request, not the install, so no test with a fake `npm` could prove a bound; coreutils `timeout`, which macOS lacks, while `ci-check.sh` runs on its bash 3.2, so the script ends the install with the shell's own job handling; and a limit on the whole check or mutation command, which must be loose enough for a loaded machine and can't retry the one step that stalled (`backlog/a-hung-land-check-holds-every-land-of-the-clone`). `npm test` and `npx` are not installs and run as before. The `npm ci` of `clm.toml`'s `[build] setup` (`config/build-setup-here`) and of the two workflows stay as they are: the first holds one build, whose session runs `clm setup` under its own tool's time limit, and GitHub ends a job at its own limit. Each script names it by an absolute path it computes once from its own, since most installs run after a `cd` into a package. Whatever the script starts to keep the time is ended before it exits, so a run that ends by itself leaves no process holding its output open. The script's tests live with the other scripts' in `crates/clm/tests/ci_suite/ci_check.rs`, whose temporary tree copies it beside them, with a fake `npm` that counts its runs and replaces itself with a sleep far past the limit they set (`exec sleep`), since the script ends the `npm` process alone and a sleep left behind as its child would hold the test's pipes open. The fakes still record each `npm` run themselves, and the script records nothing, so the existing tests' recorded lines stay as they are.

**Requirements proven by reading a file.** `release`, `web-deploy`, `site-deploy` and `node24-actions` state what the release and site workflows do. Only GitHub runs a workflow, so no run here can show what one does: a test parses the workflow file, and that reading is the requirement's proof (`requirements-state-what-the-contract-shows`, as amended by `20261121-wave-3-verify-by-the-rule`). It reaches no other file: everything `release-prepare.sh` and the other scripts do is proven by running them. The issue form a person who isn't invited is sent to, `.github/ISSUE_TEMPLATE/early-access.yml`, is `web`'s to state (`web/sign-in-early-access`), whose test reads it. `tests-time-bounded` states a value of this repository's own settings file, as below.

**Where the real tools run.** Eight requirements are proven at the land against stand-ins for the tools their scripts start, which record the arguments and answer as a scenario says; none of the real tools runs in a test (`land-checks-stay-fast`). Where each real tool runs:
- `panel-validate` and `plugin-validate`: the land check, which runs the landing machine's own `claude` whenever the selection turns either validation on, and every verify, which runs every test.
- `nextest-runner`: every land check and verify, through `ci-check.sh`, and each release's `prepare` job and each mutation pass, through `release-prepare.sh`, `coverage.sh` and `mutate.sh`.
- `coverage-script`: each mutation pass, which runs the real `coverage.sh` after its blocks and fails when it fails.
- `mutate-script`: each mutation pass, whose fixtures judge the real `cargo-mutants` and StrykerJS before any block.
- `release-records-upgrade-corpus`: each release, whose `prepare` job builds the real `clm` and runs `clm pending`, `make.sh` and `clm upgrade --lock`; a failure there publishes nothing.
- `browser-install`: the land check, which runs the real `npx playwright install chromium` whenever it runs `web/`'s or `capabilities/web/`'s suite. The `--with-deps` install, reached only with `CI=true`, runs nowhere: no workflow runs the script, and it needs `sudo`.
- `capability-web-retries`: the land check runs the real `npm test -- --retries=1` whenever it runs `capabilities/web/`, so Playwright's reading of the flag is proven there; a test that fails once and passes on its retry, or fails on both, runs nowhere, since making one fail on purpose takes a browser run of tens of seconds.

The two that run nowhere wait on a part of the check that only `[verify]` runs (`backlog/every-test-runs-in-the-land-check-the-browser-suites-included`): a run of `ci-check.sh` with `CI=true` over `web/` where `sudo` asks for no password, and, in a fixture package, a Playwright spec that fails on its first attempt and passes on its second beside one that fails both, run with `npm test -- --retries=1`.

## Requirements

### Requirement: release
The release workflow SHALL run only on `workflow_dispatch`, with a `version` input, never two runs at once, and never cancelling a run in progress. It SHALL fail, publishing nothing and leaving `main` unchanged, unless all of these hold: it runs from `main`; `version` is `MAJOR.MINOR.PATCH`, three whole numbers with no leading zero and nothing before or after them, failing otherwise naming `MAJOR.MINOR.PATCH`; `version` is not lower than the workspace version, the `[workspace.package] version` of `Cargo.toml`, failing naming `Cargo.toml` when it reads none; no release or tag `v<version>` exists, here or on `origin`, failing when it can't list the releases or `origin`'s tags; `plugin/bin/clm` maps at least one platform to a target, failing naming it when it maps none, and every target it downloads an asset for is a target of the workflow's build matrix; and, when `version` equals the workspace version, `crates/clm-upgrade/corpus/v<version>.tar.gz` exists, since a release without a bump records no corpus entry (`release-records-upgrade-corpus`). Otherwise it SHALL:
- when `version` is higher than the workspace version, commit a bump of the workspace version, `Cargo.lock`, `plugin/.claude-plugin/plugin.json`, `plugin/.codex-plugin/plugin.json` and `plugin-panel/.claude-plugin/plugin.json` to `version`, failing naming a file whose version it can't set, with the release's upgrade corpus entry and lock lines (`release-records-upgrade-corpus`), run the tests on that commit, failing and pushing nothing when a test fails, and push the commit to the branch `release/v<version>`, replacing whatever that branch held;
- build `clm` from that commit for `x86_64-unknown-linux-musl`, `aarch64-unknown-linux-musl`, `x86_64-apple-darwin` and `aarch64-apple-darwin`, and for no Windows target (`no-windows-support`), every target on a Linux runner with `cargo zigbuild` and none on a macOS runner (`cross-built-macos-binaries`), each macOS target built for macOS `11.0` or later, failing unless each Linux binary is statically linked;
- publish the release `v<version>` at that commit with `clm-<target>.tar.gz`, holding `clm`, and `clm-<target>.tar.gz.sha256` for every target, and, when publishing fails, delete the release it left, so the version can be dispatched again, and fail;
- only after publishing, bring the bump commit into `main`, with a commit message starting `Release v`, trying three times and, when every try fails, failing and printing the `git` commands that bring it in; once `main` is pushed, a failure to delete `release/v<version>` SHALL fail nothing.

Every job that runs `cargo` SHALL first install the toolchain `rust-toolchain.toml` names, with `rustup show`, and no other, and no step SHALL write the `version` input into the command it runs: the input reaches a step's shell only through the step's environment, so a dispatched value can't run as a command.

No workflow but the release workflow and the site workflow (`site-deploy`) SHALL exist under `.github/workflows/`, so GitHub runs nothing on a push or a pull request.

#### Scenario: lower version
- GIVEN workspace version `0.2.0`
- WHEN the release workflow's prepare step runs with `version` `0.1.0`
- THEN it fails, calls no `gh release create`, and pushes nothing

#### Scenario: existing release
- GIVEN workspace version `0.1.0` and a `gh` that reports a release `v0.2.0`
- WHEN the prepare step runs with `version` `0.2.0`
- THEN it fails and pushes nothing

#### Scenario: shim names an unbuilt asset
- GIVEN workspace version `0.1.0`, a build matrix without `x86_64-unknown-linux-gnu`, and a `plugin/bin/clm` that maps `Linux-x86_64` to `x86_64-unknown-linux-gnu`
- WHEN the prepare step runs with `version` `0.2.0`
- THEN it fails naming `x86_64-unknown-linux-gnu`, and pushes nothing

#### Scenario: workflow shape
- GIVEN the release workflow file
- WHEN it is parsed
- THEN it triggers only on `workflow_dispatch` with a required `version` input, its build matrix lists exactly the four targets above, with no `-gnu` and no `windows` target and no `windows-latest` runner, every target builds on `ubuntu-latest` with `cargo zigbuild` and no build step runs plain `cargo build`, no job runs on a `macos-*` runner, the Linux targets check the binary is statically linked, the two macOS builds set `MACOSX_DEPLOYMENT_TARGET` to `11.0` and the Linux targets' builds set none, the matrix rows hold only `target`, and the job that pushes to `main` runs `gh release create` before the push

#### Scenario: only workflows
- GIVEN `.github/workflows/`
- WHEN its files are listed
- THEN it holds `release.yml` and `site.yml` and nothing else

#### Scenario: same version
- GIVEN workspace version `0.1.0`, no release or tag `v0.1.0`, and `crates/clm-upgrade/corpus/v0.1.0.tar.gz`
- WHEN the prepare step runs with `version` `0.1.0`
- THEN it succeeds with `sha` set to the dispatched commit, and pushes nothing

#### Scenario: higher version
- GIVEN workspace version `0.1.0` and no release `v0.2.0`
- WHEN the prepare step runs with `version` `0.2.0`
- THEN `release/v0.2.0` on the remote holds one new commit setting `0.2.0` in `Cargo.toml`, `Cargo.lock`, `plugin/.claude-plugin/plugin.json`, `plugin/.codex-plugin/plugin.json` and `plugin-panel/.claude-plugin/plugin.json`, and the remote's `main` is unchanged

#### Scenario: same version without a corpus entry
- GIVEN workspace version `0.1.0`, no release or tag `v0.1.0`, and no `crates/clm-upgrade/corpus/v0.1.0.tar.gz`
- WHEN the prepare step runs with `version` `0.1.0`
- THEN it fails naming the corpus entry, and pushes nothing

#### Scenario: malformed version
- GIVEN workspace version `0.1.0`
- WHEN the prepare step runs with `version` `1.2`, `v1.2.3`, `1.2.3-rc1`, `1.2.3.4`, empty, `0.2.0+1`, `0.01.0` and `00.2.0`, each in turn
- THEN each time it fails naming `MAJOR.MINOR.PATCH`, calls no `gh release create`, and pushes nothing

#### Scenario: release list fails
- GIVEN workspace version `0.1.0` and a `gh` that exits 1, and again a `gh` that lists no release and an `origin` whose tags can't be listed
- WHEN the prepare step runs with `version` `0.1.0`
- THEN each time it fails, calls no `gh release create`, and pushes nothing

#### Scenario: shim maps nothing
- GIVEN workspace version `0.1.0` and a `plugin/bin/clm` that maps no platform to a target
- WHEN the prepare step runs with `version` `0.1.0`
- THEN it fails naming `plugin/bin/clm`, and pushes nothing

#### Scenario: version not set
- GIVEN workspace version `0.1.0` and a `plugin/.claude-plugin/plugin.json` whose version is written `"version" : "0.1.0"`, and again a `Cargo.toml` with no `[workspace.package] version`
- WHEN the prepare step runs with `version` `0.2.0`
- THEN each time it fails naming that file, `plugin.json` or `Cargo.toml`, and pushes nothing

#### Scenario: failing tests
- GIVEN workspace version `0.1.0` and a test of the workspace that fails
- WHEN the prepare step runs with `version` `0.2.0`
- THEN it fails, its stderr holds the failing test's output, and it pushes nothing

#### Scenario: an earlier run's branch
- GIVEN workspace version `0.1.0` and an `origin` whose `release/v0.2.0` holds a commit an earlier run left
- WHEN the prepare step runs with `version` `0.2.0`
- THEN `origin`'s `release/v0.2.0` is one commit ahead of `main`, the bump, and holds no file of the earlier commit

#### Scenario: one release at a time
- GIVEN the release workflow file
- WHEN it is parsed
- THEN its `concurrency` group is `release` and it doesn't cancel a run in progress

#### Scenario: a failed upload
- GIVEN the release workflow file
- WHEN it is parsed
- THEN the step that runs `gh release create` runs `gh release delete` for `v<version>` and exits non-zero when the create fails, and no step of its job continues on error or runs on `always()`

#### Scenario: the merge into main fails
- GIVEN the release workflow file
- WHEN its step named `merge into main` is parsed
- THEN it tries the merge and the push three times, then prints the `git` commands that merge `release/v<version>` into `main` and push it and exits 1, and a failing deletion of the branch after the push doesn't fail it

#### Scenario: the version is never interpolated
- GIVEN the release workflow file
- WHEN it is parsed
- THEN no step's `run` holds `inputs.version`

#### Scenario: the pinned toolchain
- GIVEN the release workflow file
- WHEN it is parsed
- THEN every job with a step that runs `cargo`, `release-prepare.sh` or `detector-tools.sh` runs `rustup show` before it, with no condition and not continuing on error, and no step uses `dtolnay/rust-toolchain`

#### Serves
- capability/install-and-release

### Requirement: web-checks
`.github/scripts/ci-check.sh` SHALL build `clm-web` for `wasm32-unknown-unknown`, build `clm-model` for it, build `clm-check`, `clm-orchestrator`, `clm-view` and `clm-ledger` for it with default features disabled, and run the `web/` and `plugin-panel/` test suites, failing if any of these fail. Whenever it runs `web/`'s suite it SHALL then run `npm ci --ignore-scripts` and `npm test` in `web/site/` (`web.site`), only when every test is selected (`selected-tests`) or the selection names a file under `web/site/`, and then in `capabilities/web/`, and whenever it runs `plugin-panel/`'s suite it SHALL then run them in `capabilities/panel/`, failing if either fails; a package directory without a `package.json` SHALL run nothing (`capability-browser-packages`).

#### Scenario: every check
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, and a `package.json` in `web/site/`, `capabilities/web/` and `capabilities/panel/`
- WHEN it runs
- THEN `cargo` ran fmt, clippy, the Rust tests as `nextest-runner` states, and `build --target wasm32-unknown-unknown` for each crate above, and `npm` ran `ci` and `test` in `web/`, `web/site/`, `plugin-panel/`, `capabilities/web/` and `capabilities/panel/`, each package after the suite it follows

#### Scenario: a web change leaves the site out
- GIVEN the same fakes, a `package.json` in `web/site/` and `capabilities/web/`, and a `CLM_SELECTION` file holding only `file web/src/app.ts`
- WHEN it runs
- THEN `npm` ran `ci` and `test` in `web/` and then in `capabilities/web/`, and nothing in `web/site/`

#### Scenario: a site change runs the site
- GIVEN the same fakes and package files, and a `CLM_SELECTION` file holding only `file web/site/guide/index.md`
- WHEN it runs
- THEN `npm` ran `ci` and `test` in `web/`, then in `web/site/`, then in `capabilities/web/`

#### Scenario: packages not built yet
- GIVEN the same fakes and no `package.json` under `web/site/` or `capabilities/`
- WHEN it runs
- THEN `npm` ran in `web/` and `plugin-panel/` only, and the script exits 0

#### Scenario: failing capability package
- GIVEN the same fakes, with both packages present and an `npm` that exits 1 on `test` in `capabilities/panel/`
- WHEN it runs
- THEN the script exits non-zero

#### Scenario: broken wasm build
- GIVEN the same fakes, the `cargo` fake failing one of the wasm builds above, each build in turn
- WHEN it runs
- THEN each time it exits non-zero and that build is the last command it ran

### Requirement: web-deploy
When a release is published, the release workflow SHALL deploy the `web` app's Worker and its assets with `wrangler deploy`, using only the Cloudflare credentials held in repository secrets, and SHALL NOT build or deploy the public site (`site-worker-of-its-own`).

#### Scenario: release
- GIVEN a published release `v<workspace version>`
- WHEN the release workflow runs
- THEN its `web` job runs `wrangler deploy` with `CLOUDFLARE_API_TOKEN` from secrets, and no step of it names `web/site`

#### Serves
- capability/follow-from-browser
- architecture/web-optional

### Requirement: detector-tools
`.github/scripts/detector-tools.sh` SHALL install the tools behind `clm.toml`'s detectors: today only `cargo-machete`, with `cargo install --locked --version <x.y.z> cargo-machete`. A detector whose command's first word is `clm` SHALL have no install line: it runs the clone's own `clm`, or the binary under test in `this_repository_is_clean` (`clm/clean-check-runs-own-clm`). Nor SHALL a detector whose command is `sh` and the path of a file in the repository, such as `retired-terms` (`config/retired-terms-detector`): the script needs only `sh` and `grep`, which every machine that runs the checks has. `.github/scripts/ci-check.sh` SHALL run the script before any other `cargo` command it runs, and the release workflow's `prepare` job SHALL run it before `release-prepare.sh`. No other file SHALL install a detector's tool.

#### Scenario: detector tool installed
- GIVEN `.github/scripts/detector-tools.sh`
- WHEN its text is read
- THEN a line installs `cargo-machete` with `--locked` and an exact `--version`

#### Scenario: every detector's tool
- GIVEN this repository's `clm.toml`, and `.github/scripts/detector-tools.sh` with a fake on `PATH` that records its arguments for `cargo` and for the first word of the hint of each detector whose command doesn't start with `clm`, nor with `sh` followed by the path of a file of the repository
- WHEN the script runs
- THEN every detector whose command doesn't start with `clm`, nor with `sh` followed by the path of a file that exists in the repository, has a hint ending in its command's first word, and the script ran that hint's command, word for word; and no command it ran installs `clm` or `sh`

#### Scenario: check script installs the tools
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, and every test selected
- WHEN it runs
- THEN `cargo`'s first call is the install `detector-tools.sh` runs, `install --locked --version <x.y.z> cargo-machete`, it comes before `fmt`, and the only other `cargo install` the run made is `test-tools.sh`'s

#### Scenario: release installs the tools
- GIVEN the release workflow file
- WHEN its `prepare` job is read
- THEN a step runs `.github/scripts/detector-tools.sh`, after the toolchain step and before the step that runs `release-prepare.sh`

#### Serves
- capability/no-dead-weight
- architecture/no-dead-code

### Requirement: node24-actions
Every action the release and site workflows run SHALL be one GitHub runs on Node.js 24, never one it runs on Node.js 20: `actions/checkout` at `v5`, `actions/upload-artifact` at `v6`, `actions/download-artifact` at `v7` and `actions/setup-node` at `v5`, or a later major of each, named by its major. The land check SHALL fail on a workflow that runs any other action, or one of these at an earlier major or at a ref that names no major, naming what the step's `uses:` gives, until a spec change adds that action's first Node.js 24 major to this list.

#### Scenario: release workflow actions
- GIVEN the release workflow file
- WHEN its `uses:` values are read
- THEN each names `actions/checkout`, `actions/upload-artifact`, `actions/download-artifact` or `actions/setup-node`, at a major no lower than `v5`, `v6`, `v7` and `v5` respectively

#### Scenario: Node.js 20 action
- GIVEN a copy of the release workflow with one step changed to `actions/checkout@v4`
- WHEN its `uses:` values are read
- THEN it fails, naming `actions/checkout@v4`

#### Serves
- capability/install-and-release

### Requirement: selected-tests
`.github/scripts/ci-check.sh` SHALL run every test, with the commands `nextest-runner` states, when `CLM_SELECTION` is unset or empty, names a path where there is no regular file (nothing, or a directory), or names a file that starts with an `all` line. Otherwise it SHALL run, among the tests, only these, for the file's `file <path>` lines, ignoring every other line: for `crates/<dir>/tests/<name>.rs`, that package's `--test <name>`; for any other file under `crates/<dir>/tests/`, that package's `--tests`; for a file under `crates/<dir>/src/`, that package's unit tests (`--lib`, `--bins`, whichever targets it has); for any other file under `crates/<dir>/`, that package's `--tests`; for a file under `web/` or `plugin-panel/`, that directory's `npm ci` and `npm test`, and the capability package `web-checks` runs after it; for a file under `capabilities/web/` or `capabilities/panel/`, that package as `web-checks` runs it, after `web/`'s `npm ci` for `capabilities/web/`; each suite and package at most once; with one `cargo nextest run --no-tests=pass -p <package>` with those targets per selected package, the package being the one `crates/<dir>/Cargo.toml` names in its `[package]` section, so a file under a `crates/<dir>/` whose `Cargo.toml` is missing or names no package selects nothing, and no Rust test run when none is selected. A Rust test run that fails SHALL fail the script, and it SHALL run nothing after that run. Its fmt, clippy, detector-tool and wasm-build steps SHALL run whatever the selection.

#### Scenario: one integration test
- GIVEN the script with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, both capability packages present, and `CLM_SELECTION` naming a file with the component line of `orchestrator.run` and `file crates/clm/tests/run.rs`
- WHEN it runs
- THEN `cargo` ran fmt, clippy and the wasm builds, and one `nextest run` with `--no-tests=pass -p clm --test run`, and no `test`, and `npm` ran no suite and no package

#### Scenario: helper and source files
- GIVEN the same fakes and a selection with `file crates/clm-land/tests/common/mod.rs` and `file crates/clm-check/src/lib.rs`
- WHEN it runs
- THEN `cargo nextest run` ran once with `-p clm-check --lib` and once with `-p clm-land --tests`

#### Scenario: web file
- GIVEN the same fakes and a selection with only `file web/src/app.ts`
- WHEN it runs
- THEN `npm` ran `ci` and `test` in `web/` and then in `capabilities/web/` only, and `cargo` ran no `test` and no `nextest`

#### Scenario: capability package file
- GIVEN the same fakes and a selection with `file capabilities/panel/watch-beside-chat.test.ts` and `file capabilities/web/follow-from-browser.spec.ts`
- WHEN it runs
- THEN `npm` ran `ci` and `test` in `capabilities/panel/`, `ci` in `web/`, and `ci` and `test` in `capabilities/web/`, each once, never `test` in `web/` or in `plugin-panel/`, and `cargo` ran no `test` and no `nextest`

#### Scenario: everything
- GIVEN the same fakes and a selection whose one line is `all Cargo.lock`, and again with `CLM_SELECTION` unset
- WHEN it runs
- THEN each time `cargo nextest run --workspace` and then `cargo test --doc --workspace` ran, and both npm suites and both capability packages ran

#### Scenario: empty selection
- GIVEN the same fakes and an empty selection file
- WHEN it runs
- THEN `cargo` ran fmt, clippy and the wasm builds and no `test` or `nextest`, and `npm` ran no suite and no package

#### Scenario: a missing selection file
- GIVEN the same fakes, and `CLM_SELECTION` naming a path where there is no file, and again a directory
- WHEN it runs
- THEN each time it runs every test, as in *everything*

#### Scenario: a deleted crate
- GIVEN the same fakes and a selection with `file crates/gone/src/lib.rs` and `file crates/gone/tests/x.rs`, `crates/gone/` holding no `Cargo.toml`, and a file of a crate whose `Cargo.toml` holds a `[workspace]` section and no `[package]` one
- WHEN it runs
- THEN `cargo` ran no `nextest run` and no `test`

#### Scenario: a failing selected test
- GIVEN the same fakes, a selection with `file crates/clm-check/src/lib.rs` and `file crates/clm-land/src/lib.rs`, and a `cargo` whose `nextest run` for `clm-check` exits 1
- WHEN it runs
- THEN it exits non-zero, and that `nextest run` is the last command it ran

### Requirement: panel-validate
Wherever `.github/scripts/ci-check.sh` runs `plugin-panel/`'s tests, it SHALL also run `claude plugin validate plugin-panel` and fail when that command exits non-zero. When no `claude` is on `PATH`, it SHALL print `skipped claude plugin validate: claude is not on PATH` to stderr and go on as if the step had passed. Every `claude plugin validate` the script runs, this one and `plugin-validate`'s, SHALL run with an empty standard input (`/dev/null`), so an unattended land never waits on a prompt.

#### Scenario: a refused module fails the check
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo` and `npm`, and a fake `claude` that records its arguments and exits 1
- WHEN it runs with every test selected
- THEN `claude` ran with `plugin validate plugin-panel`, and the script exits non-zero

#### Scenario: a valid module passes
- GIVEN the same fakes, with a `claude` that exits 0
- WHEN it runs with every test selected
- THEN `claude` ran with `plugin validate plugin-panel`, and the script exits 0

#### Scenario: no claude on PATH
- GIVEN the same fakes for `cargo` and `npm`, and no `claude` on `PATH`
- WHEN it runs with every test selected
- THEN its stderr holds `skipped claude plugin validate: claude is not on PATH`, and it exits 0

#### Scenario: panel not selected
- GIVEN the same fakes, with a recording `claude`, and a `CLM_SELECTION` file naming only `crates/clm/src/main.rs`
- WHEN it runs
- THEN `claude` didn't run

#### Scenario: no prompt
- GIVEN the same fakes, with a `claude` that reads a line from its standard input, records whether it read one, and exits 0, and the script's own standard input a pipe holding a line `y`, then closed
- WHEN it runs with every test selected
- THEN `claude` ran with `plugin validate plugin-panel` and with `plugin validate plugin`, each time reading no line, and the script exits 0

### Requirement: plugin-validate
When `.github/scripts/ci-check.sh` runs with every test selected (`CLM_SELECTION` unset or empty, or its file starting with an `all` line), or with a `CLM_SELECTION` file holding a `file <path>` line whose path is under `plugin/`, it SHALL run `claude plugin validate plugin` and fail when that command exits non-zero. It SHALL NOT run it for a selection with no such line. When no `claude` is on `PATH`, it SHALL print `skipped claude plugin validate: claude is not on PATH` to stderr and go on as if the step had passed.

#### Scenario: a refused plugin fails the check
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx`, a fake `claude` that records its arguments and exits 1 when they are `plugin validate plugin`, and a `CLM_SELECTION` file naming `file plugin/skills/propose/SKILL.md`
- WHEN it runs
- THEN `claude` ran with `plugin validate plugin`, and the script exits non-zero

#### Scenario: a valid plugin passes
- GIVEN the same fakes, with a `claude` that exits 0
- WHEN it runs with every test selected
- THEN `claude` ran with `plugin validate plugin` and with `plugin validate plugin-panel`, and the script exits 0

#### Scenario: no claude on PATH
- GIVEN the same fakes for `cargo`, `npm` and `npx`, no `claude` on `PATH`, and a selection naming `file plugin/hooks/hooks.json`
- WHEN it runs
- THEN its stderr holds `skipped claude plugin validate: claude is not on PATH`, and it exits 0

#### Scenario: plugin not selected
- GIVEN the same fakes, with a recording `claude`, and a `CLM_SELECTION` file naming only `file crates/clm/src/main.rs`
- WHEN it runs
- THEN `claude` never ran with `plugin validate plugin`

### Requirement: browser-install
`.github/scripts/ci-check.sh` SHALL run `npx playwright install chromium` in `web/` before `web/`'s tests, and in `capabilities/web/` before that package's tests, adding `--with-deps` only when the `CI` environment variable is `true`, and SHALL fail without running those tests when the install exits non-zero. It SHALL run no install in a directory whose tests aren't selected.

#### Scenario: in CI
- GIVEN `.github/scripts/ci-check.sh`, with `CI=true` and fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments
- WHEN it runs with every test selected
- THEN `npx` is called with `playwright install --with-deps chromium` before `web/`'s `npm test`

#### Scenario: outside CI
- GIVEN the same script and fakes, with `CI` unset, or set to `false`, `1` or empty
- WHEN it runs with every test selected
- THEN `npx` is called with `playwright install chromium`, never with `--with-deps`, before `web/`'s `npm test`, and `web/`'s `npm test` still runs

#### Scenario: capability package
- GIVEN the same script and fakes, with `CI` unset and a `package.json` in `capabilities/web/`
- WHEN it runs with every test selected
- THEN `npx` is called with `playwright install chromium` in `capabilities/web/` before that package's `npm test`

#### Scenario: failing install
- GIVEN the same script and fakes, with `CI` unset and an `npx` that exits 1 on `playwright install chromium`
- WHEN it runs with every test selected
- THEN the script exits non-zero, and `web/`'s `npm test` never runs

#### Scenario: web not selected
- GIVEN the same script and fakes, with `CI` unset and a `CLM_SELECTION` file naming only `crates/clm/src/main.rs`
- WHEN it runs
- THEN `npx` is never called with `playwright install`

### Requirement: coverage-script
`.github/scripts/coverage.sh` SHALL write `target/lcov.info`, one lcov tracefile holding the workspace's coverage from `cargo llvm-cov nextest --workspace --no-rustc-wrapper --lcov` and the `web/` and `plugin-panel/` vitest suites' from `@vitest/coverage-v8`, every `SF:` path naming its file relative to the repository root: it SHALL strip the root of the tree it runs in, both as `$PWD` names it and as `pwd -P` does, from each path `cargo llvm-cov` writes absolutely under it, so the tracefile reads the same from any checkout of the repository. It SHALL NOT run the Rust tests through plain `cargo llvm-cov` or `cargo test`, so they run one process per test as `nextest-runner` requires. Before the Rust tests it SHALL run `.github/scripts/detector-tools.sh`, then `.github/scripts/test-tools.sh`, `rustup component add llvm-tools-preview` and `cargo install --locked --version <x.y.z> cargo-llvm-cov`; after them, in each npm directory, before its vitest suite, `npm ci --ignore-scripts` and then `npm install --no-save --ignore-scripts @vitest/coverage-v8@<x.y.z>` at the version `npx vitest --version` prints there, so no `package.json` or lockfile changes; when that prints no version it SHALL fail, writing `coverage.sh: no vitest version in <dir>` to stderr, before it installs `@vitest/coverage-v8` there. It SHALL exit non-zero when any of these commands fails, a test that `tests-time-bounded` ends included, and SHALL never pass `--ignore-run-fail`. No command `.github/scripts/ci-check.sh` runs, nor any workflow step, SHALL be it or `cargo llvm-cov`: `.github/scripts/mutate.sh` runs it (`mutate-script`).

#### Scenario: one tracefile from three suites
- GIVEN the script with fakes on `PATH` for `cargo`, `rustup`, `npm` and `npx` that record their arguments, the `cargo` fake writing a record for `<tree>/crates/clm/src/main.rs` at the `--output-path` it is given, and the `npx` fake printing `vitest/5.0.5` for `--version` and writing `lcov.info` with a record for `src/app.ts` under the `--coverage.reportsDirectory` it is given
- WHEN it runs from the tree's root
- THEN `rustup` ran `component add llvm-tools-preview`, `cargo` ran `install --locked --version <x.y.z> cargo-nextest` and `install --locked --version <x.y.z> cargo-llvm-cov` before `llvm-cov nextest --workspace --no-rustc-wrapper --lcov`, and no `test` and no `llvm-cov` without `nextest`, `npm` ran `ci --ignore-scripts` and then `install --no-save --ignore-scripts @vitest/coverage-v8@5.0.5` in `web/` and in `plugin-panel/`, `npx` ran `vitest run` with `--coverage.enabled` in both, and `target/lcov.info` holds records for `crates/clm/src/main.rs`, `web/src/app.ts` and `plugin-panel/src/app.ts`, in that order, none of its `SF:` lines naming `<tree>`, and the script exits 0

#### Scenario: a failing suite fails the script
- GIVEN the same fakes, with an `npx` that exits 1 for `vitest run` in `plugin-panel/`
- WHEN it runs
- THEN it exits non-zero

#### Scenario: a failing Rust run fails the script
- GIVEN the same fakes, with a `cargo` that exits 100 for `llvm-cov nextest`, as nextest does when a test fails or is ended at its bound
- WHEN it runs
- THEN it exits non-zero, `npm` ran nothing after it, and `target/lcov.info` is as it was before the run

#### Scenario: nothing else runs it
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, a stand-in `coverage.sh` beside it that records each run, and every test selected; and the two workflow files
- WHEN the script runs and the workflows are parsed
- THEN no recorded call runs `llvm-cov` or the stand-in `coverage.sh`, and no workflow step runs either

#### Scenario: pinned tools
- GIVEN the script with the fakes of *one tracefile from three suites*
- WHEN it runs
- THEN `cargo` installed `cargo-machete`, then `cargo-nextest`, then `cargo-llvm-cov`, each with `--locked` and an exact `--version`, before its call `llvm-cov nextest`, which passed `--no-rustc-wrapper`, no call passed `--ignore-run-fail`, and each `@vitest/coverage-v8` install passed `--no-save`

#### Scenario: no vitest version
- GIVEN the same fakes, with an `npx` that prints nothing for `vitest --version` in `plugin-panel/`
- WHEN it runs
- THEN it exits non-zero, stderr holds `coverage.sh: no vitest version in plugin-panel`, `npm` installed no `@vitest/coverage-v8` in `plugin-panel/`, and `target/lcov.info` is as it was before the run

#### Serves
- capability/know-health

### Requirement: no-model-steps
`.github/scripts/ci-check.sh` SHALL start no model session and run no eval: it SHALL NOT run `clm superpowers eval`, `claude plugin eval` or `plugin/evals/` scripts, whatever `CLM_SELECTION` names (`cut-evals`). A selection naming `crates/clm-run/superpowers.pin` SHALL select `clm-run`'s tests as any file of that crate does, and one naming a file under `plugin/evals/`, deleted or not, SHALL select only `claude plugin validate plugin`.

#### Scenario: the pin changes
- GIVEN the script with fakes on `PATH` for `cargo`, `npm`, `npx` and `claude` that record their arguments, and a selection naming `file crates/clm-run/superpowers.pin`
- WHEN it runs
- THEN `cargo` ran `nextest run --no-tests=pass -p clm-run --tests` and never `run`, and the script exits 0

#### Scenario: a deleted eval file
- GIVEN the same fakes and a selection naming only `file plugin/evals/superpowers-unattended/run.sh`, a path that doesn't exist
- WHEN it runs
- THEN `claude` ran only with `plugin validate plugin`, `cargo` never ran `run` or `test`, and the script exits 0

### Requirement: no-workflow-pushes-main
No workflow under `.github/workflows/` SHALL push anything to `main`, by any means, but the release's merge of its version bump on the lines of `release.yml`'s step named `merge into main`: the one push to `main` that isn't a `clm land` (`land-locally`), which `clm ledger verify` accepts (`ledger/verify`). The land check SHALL fail on any line outside that step that runs a `git push` whose destination is `main`, by refspec (`main`, `HEAD:main`, `refs/heads/main`, `+main` or a `:main` suffix), `--all` or `--mirror`, naming the file and line; a `git push` inside a quoted string, such as an `echo` telling a human what to run, counts like any other line outside that step.

#### Scenario: a workflow pushes main
- GIVEN a workflow file with a step running `git push origin HEAD:main`
- WHEN the land check reads it
- THEN it fails naming the file and line

#### Scenario: the release merge
- GIVEN this repository's `.github/workflows/`
- WHEN the land check reads it
- THEN it passes, the release's `merge into main` push being the one exception

#### Serves
- architecture/land-locally

### Requirement: capability-web-retries
`.github/scripts/ci-check.sh` SHALL run `capabilities/web/`'s tests as `npm test -- --retries=1`, so that Playwright runs a failed test once more and the script fails only when a test fails on both attempts. It SHALL run the `test` script of `web/`, `plugin-panel/` and `capabilities/panel/` with no retry.

#### Scenario: the web package retries
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, and a `package.json` in `capabilities/web/` and in `capabilities/panel/`
- WHEN it runs
- THEN `npm` ran `test -- --retries=1` in `capabilities/web/`, and `test` with no other argument in `web/`, `plugin-panel/` and `capabilities/panel/`

#### Scenario: failing on both attempts
- GIVEN the same fakes, with an `npm` that exits 1 on `test` in `capabilities/web/`
- WHEN it runs
- THEN the script exits non-zero

### Requirement: site-deploy
`.github/workflows/site.yml` SHALL run only on `workflow_dispatch`, build `web/site/` from `main` with `UMAMI_WEBSITE_ID` set from the repository's Actions variable of that name and `GITHUB_TOKEN` the workflow's own (`web.site/github-link`), and deploy it by running `npx wrangler deploy` in `web/site/`, using only the Cloudflare credentials held in repository secrets, with `permissions` granting no more than `contents: read` and `concurrency` group `site`, so the released site, and no other build, reports to the operator's analytics (`site-analytics-umami`).

#### Scenario: dispatched only
- GIVEN the repository
- WHEN `.github/workflows/site.yml` is parsed
- THEN its only trigger is `workflow_dispatch`, it checks out `main`, its `concurrency` group is `site`, and its deploy step runs `npx wrangler deploy` in `web/site/` with `CLOUDFLARE_API_TOKEN` from secrets

#### Scenario: website ID and token
- GIVEN the site workflow
- WHEN its build step is parsed
- THEN its environment sets `UMAMI_WEBSITE_ID` to `${{ vars.UMAMI_WEBSITE_ID }}` and `GITHUB_TOKEN` to `${{ github.token }}`, the job's `permissions` grant no more than `contents: read`, and no file under `.github/` holds a literal website ID

#### Serves
- capability/publish-posts
- capability/see-site-use
- capability/learn-clm

### Requirement: release-records-upgrade-corpus
When `release-prepare.sh` bumps the version, it SHALL build `clm` from the bumped tree, run `crates/clm-upgrade/corpus/make.sh <version> <clm>` and `<clm> upgrade --lock <version>` with it, and add their output to the bump commit, before the tests run on that commit: the corpus entry, `crates/clm-upgrade/migrations.lock`, and `crates/clm-init/migrations.lock` when the lock step left one. A failure of either SHALL fail `prepare`, publishing nothing. Before it runs either, it SHALL run that `clm` as `<clm> pending upgrade` and then as `<clm> pending init`, making both runs whatever the first gave and leaving their standard error as it is, and fail, writing no lock line and pushing nothing, when either exits non-zero or prints a line on standard output, whatever the line, a blocked requirement's line, ending ` blocked <blocker>`, included. It SHALL report a run that exits non-zero, as `clm pending` does with 1 when the specs don't load, after printing the violations on standard output, and with 4 when a required migration isn't taken, as `release: clm pending <component> exited <status>`, whatever it printed; and the lines a run that exits 0 printed under `release: requirements of upgrade or init are pending:`, each as printed; both runs' reports on stderr, `upgrade`'s first, and then exit 1. So a release never freezes a migration whose requirement is still to be built (`upgrade/shipped-migrations-frozen`).

#### Scenario: a release with a new migration
- GIVEN a temporary repository at version `0.2.0`, with a fake `cargo` whose built `clm` prints nothing on stdout and a checkout line on stderr for `pending upgrade` and `pending init`, exiting 0, and whose `upgrade --lock` appends a line for migration `b`, and a `make.sh` that writes its entry
- WHEN `release-prepare.sh 0.3.0` runs
- THEN the commit on `release/v0.3.0` holds `corpus/v0.3.0.tar.gz` and the lock line for `b`

#### Scenario: a release with a migration of init's
- GIVEN the same repository, with a built `clm` whose `upgrade --lock` writes `crates/clm-init/migrations.lock` with a line for migration `c`
- WHEN `release-prepare.sh 0.3.0` runs
- THEN the commit on `release/v0.3.0` holds that file with the line for `c`

#### Scenario: corpus entry fails
- GIVEN the same repository, with a `make.sh` that exits 1
- WHEN `release-prepare.sh 0.3.0` runs
- THEN it exits non-zero and pushes no `release/v0.3.0`

#### Scenario: a migration's requirement is pending
- GIVEN the same repository, with a built `clm` whose `pending upgrade` prints `upgrade/x 1234abcd unbuilt` on stdout and a checkout line on stderr, and again one whose `pending init` prints `init/y 5678abcd unbuilt blocked backlog/q`, one whose `pending init` exits 4, and one whose `pending upgrade` prints `specs/a/SPEC.md: model/x: broken` and exits 1 while its `pending init` prints `init/y 5678abcd unbuilt`
- WHEN `release-prepare.sh 0.3.0` runs
- THEN each time it exits 1 and `clm` ran both `pending upgrade` and `pending init`; stderr holds the line printed, `clm pending init exited 4`, and for the last `clm pending upgrade exited 1` and then `init/y 5678abcd unbuilt`, and not the violation line; `clm` ran no `upgrade --lock` and `make.sh` didn't run, and it pushes no `release/v0.3.0`

#### Needs
- upgrade/corpus-entry-script
- upgrade/shipped-migrations-frozen
- clm/upgrade-command
- check.coverage/pending-report
- view.repo/arguments-and-exit

#### Serves
- capability/upgrade-clm
- architecture/upgrades-carry-migrations

### Requirement: nextest-runner
Every script and workflow of this repository that runs the Rust tests SHALL run them under `cargo nextest`, one process per test, and SHALL run no test but a doctest with `cargo test`, since its tests may change their own process's environment for the length of a call (`one-runner-for-the-rust-tests`). `.github/scripts/ci-check.sh`'s full run SHALL run `cargo nextest run --workspace --no-tests=pass` and then `cargo test --doc --workspace`, and its selected run the commands `selected-tests` states; `.github/scripts/release-prepare.sh` SHALL run, where it runs the tests on the bump commit, `cargo nextest run --workspace --no-tests=pass` and then `cargo test --doc --workspace`; `.github/scripts/coverage.sh` SHALL run them as `coverage-script` states.

#### Scenario: full run
- GIVEN the script with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, and `CLM_SELECTION` unset
- WHEN it runs
- THEN `cargo` ran `nextest run --workspace --no-tests=pass`, then `test --doc --workspace`, and no other `test`

#### Scenario: the release's tests
- GIVEN `.github/scripts/release-prepare.sh` run with a version above the workspace's, against a temporary repository whose `origin` is a local bare repository, with fakes for `gh` and `cargo` on `PATH`, the `cargo` fake recording its arguments
- WHEN it runs
- THEN `cargo` ran `nextest run --workspace --no-tests=pass`, then `test --doc --workspace`, both before the bump commit was pushed, and no other `test`

#### Scenario: no plain cargo test
- GIVEN the two workflow files
- WHEN they are parsed
- THEN no step runs `cargo test` without `--doc`, and none runs `cargo llvm-cov` without its `nextest` subcommand

### Requirement: test-tools
`.github/scripts/test-tools.sh` SHALL install `cargo-nextest` with `cargo install --locked --version <x.y.z> cargo-nextest`, an exact version, and nothing else. `.github/scripts/ci-check.sh` and `.github/scripts/coverage.sh` SHALL each run it after `.github/scripts/detector-tools.sh` and before any other `cargo` command they run, and the release workflow's `prepare` job SHALL run it in a step after the step that runs `.github/scripts/detector-tools.sh` and before the step that runs `.github/scripts/release-prepare.sh`.

#### Scenario: the install
- GIVEN `.github/scripts/test-tools.sh` with a `cargo` on `PATH` that records its arguments
- WHEN it runs
- THEN `cargo` ran once, `install --locked --version <x.y.z> cargo-nextest`, with an exact version

#### Scenario: run before the tests
- GIVEN `.github/scripts/ci-check.sh` and `.github/scripts/coverage.sh`, each with fakes on `PATH` that record their arguments
- WHEN each runs
- THEN in each `cargo`'s second call installs `cargo-nextest`, after the install of `cargo-machete` that `detector-tools.sh` runs and before any other `cargo` call

#### Scenario: in the release
- GIVEN the release workflow
- WHEN its text is read
- THEN the `prepare` job has a step running `.github/scripts/test-tools.sh` after the step running `.github/scripts/detector-tools.sh` and before the step running `.github/scripts/release-prepare.sh`

### Requirement: phase-times
`.github/scripts/ci-check.sh` SHALL time each phase it runs: the tool installs, as one phase `tools`, then `cargo fmt`, `cargo clippy`, the wasm builds, the Rust tests, and each npm suite, capability package and `claude plugin validate` it runs, named and ordered `tools`, `fmt`, `clippy`, `wasm`, `tests`, `plugin-panel`, `validate plugin-panel`, `capabilities/panel`, `validate plugin`, `web`, `web/site` and `capabilities/web`. After each it SHALL print `ci-check: <phase> <n>s` to stderr, `<n>` the whole seconds it took. On exit it SHALL keep its exit status and print to stderr: when it passes, `ci-check: phases`, then one line `<n>s <phase>` per phase, in order, then `<n>s total`; when a phase fails, only `ci-check: failed in <phase> after <n>s, <n>s total`; and when it fails outside any phase, as when it can't read the file `CLM_SELECTION` names, which fails it before its first phase, only `ci-check: failed in setup after <n>s, <n>s total`, its time counted from its start. It SHALL print nothing of these to stdout. Every `<n>` SHALL be a count of whole seconds, never negative: a time the shell's clock gives as less than 0, because the clock stepped back during the run, SHALL print as `0s`.

#### Scenario: a passing run
- GIVEN the script with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments and pass, and an empty selection file
- WHEN it runs
- THEN it exits 0, stderr holds `ci-check: phases`, a line ending in ` tools`, one ending in ` fmt`, one ending in ` clippy`, and a last line matching `<n>s total`, and stdout holds no line starting `ci-check:`

#### Scenario: a failing phase
- GIVEN the same fakes, but the `cargo` fake exits 3 on `clippy`
- WHEN it runs
- THEN it exits 3, stderr's last line is `ci-check: failed in clippy after <n>s, <n>s total`, stderr holds no `ci-check: phases`, and no line names a wasm-build or test phase

#### Scenario: the clock steps back
- GIVEN the same fakes, but `cargo fmt` sets the script's own shell clock (`SECONDS`) back by 100 seconds while it runs, as an exported bash function named `cargo` that takes precedence over the fake on `PATH`
- WHEN it runs
- THEN it exits 0, stderr holds `ci-check: fmt 0s`, and every `<n>` it prints, in each phase's line, the summary and `<n>s total`, is a count of digits with no `-`

#### Scenario: the phases of a full run
- GIVEN the same fakes, every test selected, and a `package.json` in `web/site/`, `capabilities/web/` and `capabilities/panel/`
- WHEN it runs
- THEN the lines after `ci-check: phases` name, in order, `tools`, `fmt`, `clippy`, `wasm`, `tests`, `plugin-panel`, `validate plugin-panel`, `capabilities/panel`, `validate plugin`, `web`, `web/site` and `capabilities/web`, and then `total`

#### Scenario: a failure before the first phase
- GIVEN the same fakes and a `CLM_SELECTION` naming a file the script can't read
- WHEN it runs
- THEN it exits non-zero, `cargo` didn't run, stdout holds no `::group::` line, and stderr's last line is `ci-check: failed in setup after <n>s, <n>s total`

### Requirement: step-markers
`.github/scripts/ci-check.sh` SHALL print on stdout `::group::<phase>` before each phase it times (`phase-times`) and `::endgroup::` after it, and, when `CLM_SELECTION` selects files, inside a phase it runs because of them, `::clm-selected::<path>` for each selected file that made it run that phase, each once: for the Rust tests each `file` line that selected a package, for `web/`'s suite each selected file under `web/`, for `web/site/` its own directory's selected files, for `capabilities/web/` the files that made it run `web/`'s suite and its own directory's selected files, for `plugin-panel/`, `capabilities/panel/` and `claude plugin validate plugin-panel` each selected file under `plugin-panel/` and their own directory, and for `claude plugin validate plugin` each selected file under `plugin/`. The phases every run has, and every phase of a full run, SHALL name no file.

#### Scenario: a web change
- GIVEN the script with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments and pass, a `package.json` in `web/site/` and `capabilities/web/`, and a selection file holding `file web/src/main.ts`
- WHEN it runs
- THEN stdout holds `::group::fmt` followed by `::endgroup::` with no `::clm-selected::` line between, and, for each of `web` and `capabilities/web`, a `::group::` line naming it followed by `::clm-selected::web/src/main.ts` before its `::endgroup::`, and no `::group::` line names `web/site`

#### Scenario: a site change
- GIVEN the same fakes and package files, and a selection file holding `file web/site/guide/index.md`
- WHEN it runs
- THEN stdout holds, for each of `web`, `web/site` and `capabilities/web`, a `::group::` line naming it followed by `::clm-selected::web/site/guide/index.md` before its `::endgroup::`

#### Scenario: a full run
- GIVEN the same fakes and no selection file
- WHEN it runs
- THEN stdout holds a `::group::` and an `::endgroup::` line for each phase, and no `::clm-selected::` line

### Requirement: land-dir
When `CLM_LAND_DIR` is set and non-empty, `.github/scripts/ci-check.sh` SHALL export `CARGO_TARGET_DIR` set to it and `CARGO_INCREMENTAL=0` before its first cargo command, so this repository's lands keep their cargo output in the place `clm land` hands them (`land/land-dir-env`); when it is unset or empty, the script SHALL leave both variables as its caller has them.

#### Scenario: a land's place
- GIVEN the script's test tree with fakes for `cargo` that record `CARGO_TARGET_DIR` and `CARGO_INCREMENTAL`, and `CLM_LAND_DIR=/tmp/x/land` with both cargo variables unset
- WHEN the script runs
- THEN every recorded `cargo` call saw `CARGO_TARGET_DIR=/tmp/x/land` and `CARGO_INCREMENTAL=0`

#### Scenario: no place
- GIVEN the same with `CLM_LAND_DIR` unset and `CARGO_TARGET_DIR=/tmp/y`
- WHEN the script runs
- THEN every recorded `cargo` call saw `CARGO_TARGET_DIR=/tmp/y`

#### Needs
- land/land-dir-env

### Requirement: mutate-script
`.github/scripts/mutate.sh` SHALL be a command for `[verify] mutate` (`verify/mutation-pass`): run from the repository root with `CLM_MUTATE` naming a plan and `CLM_MUTATE_RESULT` a file, it SHALL empty the result file and then write one line for each `requirement <id>` block of the plan, appending each block's line to the result file in one write, ending in a newline, as soon as the block is judged, so a pass ended part-way leaves every verdict it reached (`verify/mutation-pass`). It SHALL judge up to `CLM_MUTATE_JOBS` blocks at once, or one when that is unset or not a positive integer, starting them in the plan's order, after every tool it needs is installed and every fixture the plan's blocks need is judged; each block it runs SHALL have a job number from 1 up to that count that no block running beside it holds, a `CARGO_TARGET_DIR` of its own for that job number for its `cargo llvm-cov` runs, its `cargo mutants` calls and its `cargo nextest` runs, and `CLM_SLOT` set, for every command it runs, to the `CLM_SLOT` it was given plus its job number less one, while the fixtures and `coverage.sh` run with the `CLM_SLOT` it was given, so blocks at once share no build directory and no port or path their tests derive from the slot (`tests-run-concurrently`). With `CLM_MUTATE` or `CLM_MUTATE_RESULT` unset or empty, or `CLM_SLOT` set to what isn't a whole number, it SHALL exit non-zero, naming the variable on stderr (`mutate.sh: CLM_SLOT is not a whole number: <value>` for `CLM_SLOT`), before it runs any tool or empties the result file. On `SIGTERM` or `SIGINT` it SHALL end the blocks it runs and what they started, remove its temporary directory, and exit with status 143. When `cargo nextest --version` fails it SHALL exit non-zero before its first Rust block, naming `cargo-nextest` on stderr, since it installs no nextest. For a block whose `code` lines name Rust files under `crates/`, it SHALL run `cargo mutants` at a version the script pins, installed with `cargo install --locked --version <x.y.z> cargo-mutants` when absent, on the mutants of those files that `--in-diff` on the block's `diff` file finds when it has one, and, when it tries none of those or the tests pass for every one, on up to the block's `limit` further mutants of those files not among them, the limit counting only mutants it may try, taken by their share, highest first, and in `cargo mutants --list` order among equal shares. A line's share is its count in the run that finds the lines the block's tests run (below) divided by the sum of its counts in the runs of every test of the block's test packages, or 1 when that sum is 0 or the quotient is larger; a mutant's share is the highest share among the lines of its span the block's tests run. A line's count in a report is the sum of the counts its `DA:` lines give it there. So the mutants of code only the requirement's tests run come first, and those of code every test of the package runs, which no test citing the requirement is about, come last. It SHALL make the run of every test of a package only when more further mutants may be tried than the block's `limit`, after the run that finds the lines the block's tests run, in the tree it makes, through `cargo llvm-cov nextest --no-rustc-wrapper --no-tests=pass --ignore-run-fail --lcov` over that package with no filterset, reading each `SF:` path relative to that tree, writing its report to a path of the block's job and only then keeping it under the package's name for the rest of the pass, so a block that starts after it was kept runs none of its own for that package and two blocks at once never write one report. It SHALL try a mutant of either kind only when a line from the start to the end of its span, as `cargo mutants --list --json` gives them, is one the block's tests run, since a mutant of code the requirement's tests never run is one no test citing it could notice, and SHALL run the mutants it tries by name (`--re`). It SHALL learn those lines after the fixture and before the block's first mutant, from one run of the block's tests alone in the tree it makes, under the same filterset, through `cargo llvm-cov nextest --no-rustc-wrapper --no-tests=pass --lcov` over the block's test packages, with `cargo-llvm-cov` at the version `coverage.sh` pins, installed as `coverage.sh` installs it when absent, reporting the lines of files under a crate's `tests/` directory too, which `cargo llvm-cov` leaves out by default, and reading each `SF:` path relative to that tree. When the block's tests run no line of any of its files that has a mutant, or run lines of those files but none a mutant changes, and they run no line of any other Rust file of the repository but the files of its `test` lines, so that a test that runs another component's code still reads as before, and a file of the block's `test` lines under a crate's `tests/` directory, a test target's file, which `cargo mutants` never mutates, has a mutant that `cargo mutants --Zmutate-file <file> --json`, given the tree's copy of that file, lists on a line from the start to the end of whose span the block's tests run, it SHALL judge the block by those mutants instead, since its rule may be code in its test's own file, as a scan of the repository's sources is: up to the block's `limit` of them, file by file in the order of the block's `test` lines and each file's in the tool's order, trying none after the first one caught, the block's `diff` playing no part; it SHALL write each into the file, as the mutant's `diff` changes it, in a copy of the tree of its own for the block's job number, never the tree `cargo mutants` copies or the checkout, run the block's tests there with `cargo nextest run` under the block's filterset, and put the file back before the next and before it exits, counting the mutant caught when nextest exits 100, as when a test fails, missed when it exits 0, and neither when it exits 101, as when the mutated file doesn't build; it SHALL judge the block by them only then, never beside mutants of its code files, since a mutant of a helper that builds a test's fixture is caught however little the test asserts, and would hide the code's survivors. Otherwise, when the block's tests run no line of any of its files that has a mutant, as an empty test does, it SHALL try no mutant and count as survived the diff's mutants and the first `limit` further ones, since tests that run none of the code can notice no change to it, unless the block holds a `reads` line, when it SHALL try none and write `not-judged`, since a test that reads the repository's own files may prove a rule that is data in them (`verify/mutation-pass`); when they run lines of those files but none a mutant changes, as when the requirement's rule lives in a data or text file that no mutation tool changes, it SHALL try none and write `not-judged`; it SHALL run `cargo mutants` in a tree it makes once, before its first Rust block, in its temporary directory: a git repository whose `.git` is a directory of its own, sharing the checkout's object store by an absolute path (`git clone --shared`) and holding none of it, checked out at the checkout's `HEAD` commit, with the checkout's branches and remote-tracking branches under the same names, and every tracked file marked unchanged in its index (`git update-index --assume-unchanged`); and it SHALL have `cargo mutants` copy that `.git` into each copy it tests (`--copy-vcs true`), so that a test that asks git about this repository passes there as it does in the checkout, a mutated file leaves git reporting no tracked file changed from `HEAD`, and no copy reads or writes the checkout's index, refs or git directory; the tests it runs for each mutant SHALL be the block's `test` lines alone, each given to nextest as a filterset naming its package, its test target by nextest's binary ID, `binary_id(=<package>::<target>)`, where `ci-check.sh` passes `--test <target>` for a selected file, and its name. For a block whose `code` lines name TypeScript files under `web/src/` or `plugin-panel/`, it SHALL run StrykerJS with its vitest runner at a pinned version, added to `node_modules/` alone with `npm install --no-save --ignore-scripts` so no `package.json` or lockfile changes, mutating the diff's lines first and then up to `limit` more, with the block's vitest `test` lines alone, each of which it SHALL have StrykerJS run whether or not it imports any file StrykerJS mutates, so that a block whose tests read a file of the repository instead of importing its code, as a test of `web/.npmrc` does, reaches StrykerJS's verdict on its mutants, `NoCoverage`, rather than a run with no test that StrykerJS fails, and SHALL give it no range of a file whose name ends in `.test.ts` or `.test.tsx`, a vitest test file, since StrykerJS's instrumentation of a test file moves a `vi.mock` call that vitest hoists out of the module's top level and fails its first run; it SHALL run StrykerJS from that directory with the directory its sandbox is made in (`tempDirName`, a directory under that directory) holding, before StrykerJS starts, a symbolic link for each entry of the repository's top but `.git` and that directory, to that entry, so that the sandbox stands where that directory does among the repository's other files and a path a test resolves from its own file to a file of the repository outside that directory, as `../../specs/web/SPEC.md` from `web/src/`, reaches that file in the sandbox as it does in the checkout, and it SHALL remove that directory as soon as StrykerJS returns, so no later run, such as `coverage.sh`'s vitest, finds the links; it SHALL have StrykerJS copy into its sandbox no file or directory under that directory that the repository's `.gitignore` files mark ignored, the top's and each one under that directory that git itself would read (tracked, or untracked and not ignored), read afresh for each StrykerJS run, whether it exists when StrykerJS starts or appears while StrykerJS copies, so that output another run makes and removes there, as the site's tests do `web/site/.test-dist/`, is never in the sandbox and can't fail StrykerJS's copy; it SHALL give StrykerJS its test runner, the vitest configuration that names those tests, its reporter and its `mutate` ranges in a configuration file it writes under that directory's `node_modules/`, since StrykerJS takes a runner's options from no command line, and that file SHALL set no `thresholds.break`, so StrykerJS's exit status never depends on a score. It SHALL count a mutant StrykerJS reports `NoCoverage`, one the block's tests never run, as neither caught nor survived, unless every mutant StrykerJS reports for the block is `NoCoverage` and the block holds no `reads` line, when it SHALL count them all as survived, as for an empty test. It SHALL write `<id> killed` when the tests failed for a mutant, `<id> survived <n>` when `<n>` mutants built and the tests passed for every one, and `<id> not-judged` when no mutant built, when the block's tests run lines of its files but none a mutant changes and no mutant of its test files, when the block holds a `reads` line and its tests run no line of its files that has a mutant and no mutant of its test files, or StrykerJS reports every mutant `NoCoverage`, when the block's tests are all Playwright specs, or when its `code` lines name neither kind of file, a vitest test file counting as neither. After the blocks it SHALL run `.github/scripts/coverage.sh` and, when `CLM_LAND_DIR` is set, copy `target/lcov.info` to `$CLM_LAND_DIR/lcov.info`. Before the first block that needs a tool, it SHALL judge a fixture it writes itself with that tool, a function with one test that fails when the function is changed and one that passes whatever it returns: for `cargo-mutants` a crate in a temporary directory that the script makes a git repository with its files committed and from which it makes a tree the way it makes the checkout's, and whose tests pass only inside a git repository whose top is the crate and whose blind test also fails when git reports a tracked file changed from `HEAD` (`git status --porcelain --untracked-files=no`), so that a tool that leaves `.git` out of its copy, or a copy in which git sees the mutated file, fails the fixture, and which holds a test file under its `tests/` with a function that is no test and two tests of it, one that fails when that function is changed and one that passes whatever it returns, every mutant the tool lists for it tried as a block's test file's are, with no run to find the lines, and whose two tests also pass only inside a git repository whose top is the copy and, the blind one, fail when git reports a tracked file changed from `HEAD`, so that a copy without `.git` or one in which git sees the mutant, or a `cargo mutants --Zmutate-file` whose listing the script misreads, or a nextest whose exit status means something else, fails the fixture too, once, and for StrykerJS a source file and a test file inside the npm directory of the block, whose test that fails when the function is changed also reads a file of the repository outside that directory by a path relative to its own file, so that a sandbox from which the repository's files can't be reached fails the fixture, and which also fails when it finds a directory that the script makes one level below the fixture's files and that a `.gitignore` the script writes beside them names by a rule with no slash but a trailing one, so that a StrykerJS that copies what git ignores fails the fixture, and beside them a second test file that imports none of the fixture's source files, whose one test reads a file of the repository and passes, judged in a StrykerJS run of its own with that test file alone and the lines of the function the first test notices, so that a StrykerJS that runs only the tests that import a file it mutates, which then runs no test and fails, fails the fixture, once for each such directory before its first block, run with that directory's installed vitest and TypeScript, both removed afterwards; it SHALL go on only when the tool reports the first test's mutants caught and the second's missed, for the crate's function and for its test file's alike, and, for StrykerJS, reports for the run of the test file that imports nothing at least one mutant and every one `NoCoverage`. It SHALL exit non-zero, naming the tool and the block or the fixture on stderr, and, for a block whose own judging failed, appending to the result file in one write the line `<id> failed <why>`, `<why>` that stderr line's text after `mutate.sh: `, and no verdict for that block, but no line for a tool's install, a tree it makes or a fixture, which fail before any block's judging and are no block's, when a tool can't be installed, when a tool's verdict on the fixture is wrong, when a tool fails for a reason other than its mutants' outcomes (`cargo mutants` exiting with a status other than 0, 2 or 3, as when the tests fail in the unmutated tree or a filterset doesn't parse; StrykerJS exiting non-zero, writing no report or running no test; `cargo mutants --Zmutate-file` exiting non-zero or listing what can't be read; `cargo nextest run` exiting for a test file's mutant with a status other than 0, 100 or 101), when the run that finds the lines a block's tests run, or a run of every test of one of its packages, fails or writes no report, or when `coverage.sh` fails; and 0 otherwise, whatever the verdicts. When a block fails so, it SHALL start no further block, let the blocks already running finish and write their lines, run no `coverage.sh`, and then exit non-zero. So a tool that doesn't work records no verdict for the block it failed in, and that block and every block not started stay due (`verify/mutation-pass`). No command `ci-check.sh` runs, nor any workflow step, SHALL be it. In the scenarios below `cargo`, `rustup` and `npx` are fakes on `PATH`, as in `coverage-script`'s, that install and mutate nothing; the fake `cargo llvm-cov` reports every line of the block's files run, and of the files of its `test` lines only when given `--no-default-ignore-filename-regex`, the fake `cargo mutants --Zmutate-file` lists no mutant of the repository's files and one of the fixture's test file, and the fake `cargo nextest run` exits 100 for the fixture's test that notices and 0 for every other run, and the fake `stryker run` reports the fixture's mutants caught for the test that notices, missed for the blind one and `NoCoverage` in the run of the test file that imports nothing, unless a scenario says otherwise.

#### Scenario: a Rust requirement whose test notices
- GIVEN a plan with one block for a requirement of a crate, with a `diff` and one `test` line, and a fake `cargo` whose `mutants` reports one mutant caught
- WHEN `mutate.sh` runs
- THEN the fake was called with `--in-diff` and a filterset holding `package(=<package>)`, `binary_id(=<package>::<target>)` and the test's name, no `binary(` among its words, and the result file holds `<id> killed`

#### Scenario: tests that notice nothing
- GIVEN the same with a fake `cargo llvm-cov` that reports every line of the block's files run, and a fake `mutants` that reports 3 mutants missed for the diff and 2 more missed for the files
- WHEN it runs
- THEN the result file holds `<id> survived 5`

#### Scenario: a tool that fails
- GIVEN a plan with one Rust block, and a fake `cargo` whose `mutants` exits 4, as it does when the tests fail in an unmutated tree
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo mutants` and the requirement, and the result file holds one line for it, `<id> failed` followed by text naming `cargo mutants`, and no verdict

#### Scenario: StrykerJS is given a configuration file
- GIVEN a plan with one block for a requirement of `web/src/`, with a `diff` and one vitest `test` line, and a fake `npx` whose `stryker run` records its arguments and the file they name, and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN `stryker run` was given a configuration file under `web/node_modules/` that names the vitest runner, a vitest configuration including that test alone and the diff's ranges of the block's files; no argument begins `--vitest`; and the result file holds `<id> killed`

#### Scenario: StrykerJS runs no test
- GIVEN the same with a fake `npx` whose `stryker run` exits 1 and writes no report
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names StrykerJS and the requirement, and the result file holds one line for it, `<id> failed` followed by text naming StrykerJS, and no verdict

#### Scenario: StrykerJS mutates no test file
- GIVEN a plan with one block for a requirement of `plugin-panel/`, whose `code` lines name `plugin-panel/src/a.ts` and `plugin-panel/test/a.test.ts`, with a `diff` that changes both and one vitest `test` line naming `plugin-panel/test/a.test.ts`, and a fake `npx` whose `stryker run` records the file it is given and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN every range the recorded configuration's `mutate` names is of `src/a.ts`, and the result file holds `<id> killed`

#### Scenario: the fixture first
- GIVEN a plan with one Rust block, and a fake `cargo` that records each call
- WHEN `mutate.sh` runs
- THEN the first `cargo mutants` call ran in a crate outside the repository, before any call naming the block's files, and no file of the repository changed

#### Scenario: a tool that judges the fixture wrongly
- GIVEN the same with a fake `cargo` whose `mutants` reports every mutant caught, the fixture's blind test included
- WHEN `mutate.sh` runs
- THEN it exits non-zero naming `cargo mutants` and the fixture, no call named the block's files, and the result file is empty

#### Scenario: the fixture's test file judged wrongly
- GIVEN a plan with one Rust block, and a fake `cargo` whose `mutants` judges the fixture's crate rightly, whose `mutants --Zmutate-file` lists one mutant of the fixture's test file, and whose `nextest run` exits 100 for each of the fixture's tests, the blind one included
- WHEN `mutate.sh` runs
- THEN it exits non-zero naming the fixture, no call named the block's files, and the result file is empty

#### Scenario: Playwright only
- GIVEN a block whose `test` lines all name files under `web/e2e/`
- WHEN it runs
- THEN the result file holds `<id> not-judged`, and neither tool, nor its fixture, ran

#### Scenario: not in the land check
- GIVEN `.github/scripts/ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx` that record their arguments, a stand-in `mutate.sh` beside it that records each run, and every test selected; and the two workflow files
- WHEN the script runs and the workflows are parsed
- THEN no recorded call runs `cargo mutants`, `stryker` or the stand-in `mutate.sh`, and no workflow step runs any of them

#### Scenario: a test that asks git about the repository
- GIVEN a plan whose block names a Rust file of a crate and a test of that crate that passes only inside a git repository, and a stand-in `cargo mutants` that records its arguments
- WHEN the script runs for the block
- THEN the recorded call holds `--copy-vcs true`

#### Scenario: the fixture is made as the tree is
- GIVEN a plan with one Rust block, and a stand-in `cargo mutants` that records its arguments and the directory it ran in
- WHEN the script runs
- THEN when the tool is called in the fixture's directory, its `.git` is a directory whose `objects/info/alternates` names another repository's object store, `git status --porcelain --untracked-files=no` there prints nothing after its `src/lib.rs` is changed, the call holds `--copy-vcs true`, and the fixture's test source asks git for the repository's top and for the working tree's status

#### Scenario: the tree the tool copies has a git directory of its own
- GIVEN a plan with one Rust block, run in a linked worktree of a repository that has a branch and a remote-tracking branch, and a stand-in `cargo mutants` that records the directory it ran in
- WHEN the script runs for the block
- THEN that directory is outside the checkout, its `.git` is a directory with no `objects/pack/*.pack`, `git rev-parse HEAD` there prints the checkout's `HEAD`, the branch and the remote-tracking branch resolve there to the commits they name in the checkout, `git status --porcelain --untracked-files=no` there prints nothing after a tracked file there is changed, and the checkout's index file holds the same bytes as before the run

#### Scenario: a mutant the tests never run is not tried
- GIVEN a plan with one Rust block with no `diff` and one `test` line, a fake `cargo` whose `mutants --list --json` lists two mutants of the block's file, one spanning line 3 alone and one line 30 alone, whose `llvm-cov` reports line 3 run and line 30 not, and whose `mutants` reports every mutant it is given missed
- WHEN `mutate.sh` runs
- THEN the `llvm-cov` call's filterset names the block's test, the `cargo mutants` call that tries the block's mutants names the mutant on line 3 and not the one on line 30, and the result file holds `<id> survived 1`

#### Scenario: tests that run none of the code
- GIVEN the same with a fake `llvm-cov` that reports no line of the block's file run
- WHEN `mutate.sh` runs
- THEN no `cargo mutants` call tries a mutant of the block's file, and the result file holds `<id> survived 2`

#### Scenario: tests that run the code but no mutant of it
- GIVEN the same with a fake `llvm-cov` that reports line 10 of the block's file run and lines 3 and 30 not
- WHEN `mutate.sh` runs
- THEN no `cargo mutants` call tries a mutant of the block's file, and the result file holds `<id> not-judged`

#### Scenario: tests that read the repository's files and run none of the code
- GIVEN the plan of the scenario *tests that run none of the code*, with a line `reads specs/**` in its block, and a fake `llvm-cov` that reports no line of the block's file run
- WHEN `mutate.sh` runs
- THEN no `cargo mutants` call tries a mutant of the block's file, and the result file holds `<id> not-judged`

#### Scenario: a rule in the test's own file
- GIVEN a plan with one Rust block with no `diff` whose one `test` line names a file under its crate's `tests/`, a fake `cargo` whose `mutants --list --json` lists one mutant of the block's code file on line 30, whose `llvm-cov` reports line 30 of the code file not run and lines 5 to 9 of the test file run, whose `mutants --Zmutate-file` lists for the test file two mutants, one spanning line 6 and one line 40, and whose `nextest run` records the directory it ran in and the test file there and exits 100
- WHEN `mutate.sh` runs
- THEN the `llvm-cov` call was given `--no-default-ignore-filename-regex`, no `cargo mutants` call tries a mutant of the code file, `nextest run` ran once for the block, in a directory that is neither the checkout nor the tree `cargo mutants` ran in, with the test file there changed as the mutant on line 6 says, the checkout's test file holds the same bytes as before the run, and the result file holds `<id> killed`

#### Scenario: a rule in the test's own file that its tests don't notice
- GIVEN the same with a fake `nextest run` that exits 0
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> survived 1`

#### Scenario: a test file's mutant that doesn't build
- GIVEN the same with a fake `nextest run` that exits 101
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> not-judged`

#### Scenario: a rule in the test's own file of a block that reads files
- GIVEN the same as *a rule in the test's own file*, with a line `reads crates/**` in its block
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> killed`

#### Scenario: an empty test in a file with helpers
- GIVEN the same as *a rule in the test's own file*, with a fake `llvm-cov` that reports lines 2 and 3 of the test file run and no line of the code file
- WHEN `mutate.sh` runs
- THEN `nextest run` never ran for the block, and the result file holds `<id> survived 1`

#### Scenario: a test that runs another component's code
- GIVEN the same as *a rule in the test's own file*, with a fake `llvm-cov` that reports line 4 of a Rust file of another crate run too
- WHEN `mutate.sh` runs
- THEN `nextest run` never ran for the block, and the result file holds `<id> survived 1`

#### Scenario: code mutants first
- GIVEN the same as *a rule in the test's own file*, with a fake `llvm-cov` that reports line 30 of the code file run too, and a fake `mutants` that reports every mutant it is given missed
- WHEN `mutate.sh` runs
- THEN the `cargo mutants` call that tries the block's mutants names the mutant on line 30, `nextest run` never ran for the block, and the result file holds `<id> survived 1`

#### Scenario: nextest fails for a test file's mutant
- GIVEN the same as *a rule in the test's own file*, with a fake `nextest run` that exits 2
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo nextest` and the requirement, and the result file holds one line for it, `<id> failed` followed by text naming `cargo nextest`, and no verdict

#### Scenario: the run that finds the lines fails
- GIVEN the same with a fake `llvm-cov` that exits 1
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo llvm-cov` and the requirement, and the result file holds one line for it, `<id> failed` followed by text naming `cargo llvm-cov`, and no verdict

#### Scenario: the further mutants of the requirement's own code first
- GIVEN a plan with one Rust block with no `diff`, one `test` line and `limit 1`, a fake `cargo` whose `mutants --list --json` lists two mutants of the block's file, one spanning line 3 alone and one line 30 alone, whose `llvm-cov` reports, for the call whose filterset names the block's test, lines 3 and 30 each run once and, for a call with no filterset, line 3 run 10 times and line 30 once, and whose `mutants` reports every mutant it is given missed
- WHEN `mutate.sh` runs
- THEN one `llvm-cov` call had no filterset and named the block's test package, the `cargo mutants` call that tries the block's mutants names the mutant on line 30 and not the one on line 3, and the result file holds `<id> survived 1`

#### Scenario: no more further mutants than the limit
- GIVEN the same with `limit 2`
- WHEN `mutate.sh` runs
- THEN no `llvm-cov` call had no filterset, the `cargo mutants` call that tries the block's mutants names both mutants, and the result file holds `<id> survived 2`

#### Scenario: one run of a package's tests in a pass
- GIVEN a plan with two Rust blocks like the first of *the further mutants of the requirement's own code first*, whose tests are of the same package, and `CLM_MUTATE_JOBS` unset
- WHEN `mutate.sh` runs
- THEN exactly one `llvm-cov` call had no filterset, and the result file holds `survived 1` for each block

#### Scenario: the run of a package's tests fails
- GIVEN the plan and fake of *the further mutants of the requirement's own code first*, with an `llvm-cov` that exits 1 when called with no filterset
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo llvm-cov` and the requirement, and the result file holds one line for it, `<id> failed` followed by text naming `cargo llvm-cov`, and no verdict

#### Scenario: a mutant StrykerJS finds no test running
- GIVEN a plan with one block for a requirement of `web/src/`, with one vitest `test` line, and a fake `npx` whose `stryker run` reports one mutant `Survived` and two `NoCoverage`
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> survived 1`

#### Scenario: StrykerJS finds no test running any mutant
- GIVEN the same with a fake `npx` whose `stryker run` reports two mutants, both `NoCoverage`
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> survived 2`

#### Scenario: StrykerJS finds no test running any mutant of a block that reads files
- GIVEN the same with a line `reads docs/**` in the block
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> not-judged`

#### Scenario: blocks at once
- GIVEN a plan with three Rust blocks, `CLM_MUTATE_JOBS=2`, `CLM_SLOT=5`, and a stand-in `cargo` whose `mutants` and `llvm-cov` record, for each call, its start and end, `CLM_SLOT` and `CARGO_TARGET_DIR`, and wait until a second block's call runs or two seconds pass
- WHEN `mutate.sh` runs
- THEN calls for two different blocks ran at once, calls for three never did, every block's call ran with `CLM_SLOT` 5 or 6, two blocks' calls running at once never shared a slot or a `CARGO_TARGET_DIR`, the fixture's calls ran with `CLM_SLOT` 5 and all ended before the first block's began, and the result file holds three lines

#### Scenario: each line as it is judged
- GIVEN a plan with two Rust blocks, `CLM_MUTATE_JOBS` unset, and a stand-in `cargo` whose `mutants` reports the first block's mutant caught and, for the second block, waits until a file appears
- WHEN `mutate.sh` runs
- THEN the result file holds the first block's `killed` line, ending in a newline, before the file appears

#### Scenario: a failing block lets the others finish
- GIVEN a plan with three Rust blocks, `CLM_MUTATE_JOBS=2`, and a stand-in `cargo` whose `mutants` exits 4 for the first block and, for the second, reports a mutant caught after the first has failed
- WHEN `mutate.sh` runs
- THEN it exits non-zero naming `cargo mutants` and the first block's requirement, the result file holds the second block's `killed` line, a `failed` line naming `cargo mutants` for the first and no line for the third, no call named the third block's files, and `coverage.sh` didn't run

#### Scenario: ended part-way
- GIVEN a plan with two Rust blocks and a stand-in `cargo` whose `mutants` reports the first block's mutant caught and, for the second, sleeps
- WHEN `mutate.sh` gets `SIGTERM` once the first line is written
- THEN it exits, the stand-in no longer runs, the result file holds the first block's line alone, and the temporary directory it made is gone

#### Scenario: a TypeScript test that reads a file of the repository
- GIVEN a plan with one block for a requirement of `web/src/`, with one vitest `test` line, and a fake `npx` whose `stryker run` records, for the `tempDirName` its configuration file names, each entry that directory holds and where it links, and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN that directory held a symbolic link for each entry of the repository's top but `.git` and `web`, to that entry; that directory is gone once the script has run, and the result file holds `<id> killed`

#### Scenario: the StrykerJS fixture reads a file of the repository
- GIVEN the same with a fake `npx` whose `stryker run` records the fixture's test file
- WHEN `mutate.sh` runs
- THEN the fixture's test that fails when its function is changed reads a file outside `web/` by a path relative to its own file

#### Scenario: StrykerJS copies nothing git ignores
- GIVEN a plan with one block for a requirement of `web/src/`, with one vitest `test` line, a checkout whose `web/site/.gitignore` names `.test-dist/` and that has no `web/site/.test-dist/`, and a fake `npx` whose `stryker run` records the configuration file it is given and reports one mutant killed
- WHEN `mutate.sh` runs
- THEN the configuration's `ignorePatterns`, read as `.gitignore` rules from `web/`, ignore `site/.test-dist/drift-src/docs` and not the block's `code` file or its test's file; each pattern carried from a `.gitignore` begins with `/`, after any `!`, since StrykerJS matches a pattern with no slash but a trailing one only at the top of the npm directory; and the result file holds `<id> killed`

#### Scenario: the StrykerJS fixture fails on what git ignores
- GIVEN the same with a fake `npx` whose `stryker run` records the fixture's test file and what the fixture's directory holds
- WHEN `mutate.sh` runs
- THEN the fixture's directory held a `.gitignore` naming, by a rule with no slash but a trailing one, a directory one level below the fixture's files that the directory also held, and the fixture's test that fails when its function is changed also fails when it finds that directory

#### Scenario: the StrykerJS fixture runs a test that imports nothing
- GIVEN a plan with one block for a requirement of `web/src/`, with one vitest `test` line, and a fake `npx` whose `stryker run` records each configuration file it is given and the test files that configuration's vitest configuration names, and reports one mutant killed for the block
- WHEN `mutate.sh` runs
- THEN one of the fixture's runs, before the block's, names a single test file, which imports none of the fixture's source files and reads a file of the repository, with the `mutate` ranges the fixture's run for its test that notices names; and the result file holds `<id> killed`

#### Scenario: a StrykerJS that runs only the tests that import a mutated file
- GIVEN the same with a fake `npx` whose `stryker run` exits 1 and writes no report for a run none of whose test files imports a file its configuration's `mutate` names, as StrykerJS does when vitest keeps only the tests that import one
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names StrykerJS and the fixture, no `stryker run` named the block's files, and the result file is empty

#### Scenario: a bad environment
- GIVEN a plan with one Rust block, and `CLM_MUTATE` unset, then `CLM_MUTATE_RESULT` unset, then `CLM_SLOT=x`, the result file holding a line `x/old killed` where one is named
- WHEN `mutate.sh` runs, each time
- THEN each time it exits non-zero, stderr names the variable, holding `mutate.sh: CLM_SLOT is not a whole number: x` for the last, no fake ran, and the result file still holds `x/old killed`

#### Scenario: a result file from before
- GIVEN a plan with one Rust block, a fake `cargo` whose `mutants` reports one mutant caught, and a result file holding a line `x/old killed`
- WHEN `mutate.sh` runs
- THEN the result file holds `<id> killed` alone

#### Scenario: no nextest
- GIVEN a plan with one Rust block, and a fake `cargo` whose `nextest --version` exits 1
- WHEN `mutate.sh` runs
- THEN it exits non-zero, stderr names `cargo-nextest`, no `cargo mutants` call ran, and the result file is empty

#### Scenario: interrupted part-way
- GIVEN the plan and stand-in of *ended part-way*
- WHEN `mutate.sh` gets `SIGINT` once the first line is written, and in another run `SIGTERM`
- THEN each time it exits with status 143, the stand-in no longer runs, the result file holds the first block's line alone, and the temporary directory it made is gone

#### Serves
- architecture/proven-by-tests
- architecture/land-checks-stay-fast

#### Needs
- verify/mutation-pass

### Requirement: npm-installs-bounded
`.github/scripts/npm-install.sh <argument>...` SHALL run `npm <argument>...` in its working directory and exit with that run's status, unless the run is still going after the limit: 300 seconds, or the whole number of seconds `NPM_INSTALL_TIMEOUT` names. It SHALL then end that `npm` process, print a line to stderr naming the arguments and the limit, and run the same command once more; when that second run is still going after the limit it SHALL end it too and exit 124. It SHALL NOT run again a command that ended by itself, whatever its status, and SHALL leave no process it started running when it exits. `.github/scripts/ci-check.sh`, `.github/scripts/coverage.sh` and `.github/scripts/mutate.sh` SHALL run every `npm ci` and `npm install` they run through it, so a stalled install delays a check, a coverage pass or a mutation pass by the limit at most twice for each install, and never without end.

#### Scenario: a stalled install is ended and run again
- GIVEN a fake `npm` on `PATH` that records its arguments, sleeps for 600 seconds the first time it runs and exits 0 at once after that, and `NPM_INSTALL_TIMEOUT=5`
- WHEN `.github/scripts/npm-install.sh ci --ignore-scripts` runs
- THEN it exits 0 within 120 seconds, the fake recorded `ci --ignore-scripts` twice, and stderr holds a line naming `ci --ignore-scripts` and `5`

#### Scenario: an install that stalls twice fails
- GIVEN a fake `npm` that records its arguments and sleeps for 600 seconds every time it runs, and `NPM_INSTALL_TIMEOUT=5`
- WHEN `.github/scripts/npm-install.sh ci --ignore-scripts` runs
- THEN it exits 124 within 120 seconds and the fake recorded `ci --ignore-scripts` twice, not three times

#### Scenario: a failing install is not run again
- GIVEN a fake `npm` that records its arguments and exits 7 at once
- WHEN `.github/scripts/npm-install.sh install --no-save --ignore-scripts x@1.0.0` runs
- THEN it exits 7 within 120 seconds, with no process it started still running, and the fake recorded its arguments once

#### Scenario: the coverage pass gets past a stalled install
- GIVEN `coverage.sh` with its fakes on `PATH`, the `npm` fake sleeping for 600 seconds on its first run alone, and `NPM_INSTALL_TIMEOUT=5`
- WHEN it runs
- THEN it exits 0 within 120 seconds and writes `target/lcov.info`, and the `npm` fake recorded `ci --ignore-scripts` three times: twice in `web/` and once in `plugin-panel/`

#### Scenario: the coverage pass fails on an install that never answers
- GIVEN the same, in a tree with no `target/lcov.info`, but the `npm` fake sleeps for 600 seconds on every run of `install`
- WHEN it runs
- THEN it exits non-zero within 120 seconds and writes no `target/lcov.info`

#### Scenario: the check gets past a stalled install
- GIVEN `ci-check.sh` with fakes on `PATH` for `cargo`, `npm` and `npx`, the `npm` fake sleeping for 600 seconds on its first run of `ci` alone, no selection, so every suite runs, and `NPM_INSTALL_TIMEOUT=5`
- WHEN it runs
- THEN it exits 0 within 120 seconds and stderr holds `ci-check: phases`

#### Scenario: the check fails on an install that never answers
- GIVEN the same, but the `npm` fake sleeps for 600 seconds on every run of `ci`
- WHEN it runs
- THEN it exits non-zero within 120 seconds and stderr's last line starts `ci-check: failed in `

#### Scenario: the mutation pass gets past a stalled install
- GIVEN `mutate.sh` with its fakes on `PATH`, a plan with one block whose `code` line names a file under `web/src/`, the `npm` fake sleeping for 600 seconds on its first run alone, and `NPM_INSTALL_TIMEOUT=5`
- WHEN it runs
- THEN it exits 0 within 120 seconds and writes that block's verdict to the result file

#### Serves
- capability/land-safely
- capability/know-health
- architecture/proven-by-tests

### Requirement: tests-time-bounded
Every `cargo nextest run` in this repository's tree (`ci-check.sh`'s, `coverage.sh`'s through `cargo llvm-cov nextest`, `mutate.sh`'s in each copy of the tree its `cargo mutants` tests, the release's `prepare` job's, a session's own) SHALL end a test still running 15 minutes after it started and count it failed, so that the run fails in bounded time, naming the test, instead of waiting on it for ever: this repository's `.config/nextest.toml`, which nextest reads from the workspace root wherever it runs, sets under `[profile.default]` `slow-timeout = { period = "60s", terminate-after = 15 }`. The file SHALL set no other profile, and no script or workflow of this repository SHALL pass nextest a profile (`--profile`, `-P`) or a configuration file (`--config-file`, `--tool-config-file`), or set `NEXTEST_PROFILE` or `NEXTEST_CONFIG`.

#### Scenario: the bound
- GIVEN `.config/nextest.toml`; `ci-check.sh`, `coverage.sh`, `release-prepare.sh` and `mutate.sh`, each with fakes on `PATH` that record their arguments and `NEXTEST_PROFILE` and `NEXTEST_CONFIG`; and the two workflow files
- WHEN the file is parsed as TOML, each script runs, with every test selected and, for `mutate.sh`, a plan with one Rust block, and the workflows are parsed
- THEN its `profile.default.slow-timeout` is `{ period = "60s", terminate-after = 15 }` and it names no other profile, and no recorded call that runs nextest (`nextest run`, `llvm-cov nextest`, `mutants`), nor any workflow step, passes `--profile`, `-P`, `--config-file` or `--tool-config-file` or sets `NEXTEST_PROFILE` or `NEXTEST_CONFIG`
