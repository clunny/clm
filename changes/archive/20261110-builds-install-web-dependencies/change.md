---
status: done
resolves:
  - builds-install-web-dependencies-by-hand-because-no-build-setup
---

## Intent

This repository declares a `[build] setup` that installs its npm packages' locked dependencies, so `clm setup` prepares a build's fresh worktree for its JavaScript suites and no build installs them by hand (`backlog/builds-install-web-dependencies-by-hand-because-no-build-setup`).

## Problem

`20261109-a-build-s-tests` gave projects `[build] setup` and `clm setup`, and decided this repository needs none: its Rust crates resolve from the worktree's own manifest, and its npm packages sit in subdirectories (`web/`, `plugin-panel/`, ...) whose lookup never reaches the main checkout's `node_modules`, so their tests fail rather than run against the wrong code. That holds for `tests-run-in-their-own-tree`'s guarantee, but it left every build that touches a JavaScript suite (`plugin.panel`, `web.client-live`, `web`, `web.site`, `capability`'s browser packages) with a worktree its tests can't run in. With no setup, `clm run`'s prompt says the repository declares none and only covers a run that resolves dependencies from outside the worktree, so each build noticed the missing `node_modules` and ran `npm ci` itself, an improvised step the declared setup exists to replace.

The design is right; this repository's settings are what's missing.

## Approaches

- **Set `[build] setup` here, naming each npm package's directory (chosen).** A loop running `npm ci --ignore-scripts` in `plugin-panel/`, `capabilities/panel/`, `web/`, `web/site/` and `capabilities/web/`, as the check script does, so a build of any component gets them. Measured on this machine: about 11 seconds for all five from npm's cache, which every build, Rust-only ones included, now pays once. `config/build-setup-here` states it, as `verify-here` and `health-lcov-here` state this repository's other settings, and a scenario fails when a tracked `package.json` lies outside the list, so a new package can't be missed.
- **Find the packages with `git ls-files`.** No list to keep, but it puts `git` in a command whose output a test can no longer state exactly, and Claude Code's worktree isolation refuses compound `git` commands a session runs by hand. The test on the list keeps it as current as finding would.
- **Install only what the build's component needs.** Would need the setup to know which component is being built; `clm setup` takes no component, and the whole run is short.
- **Tell builds in the prompt to install what a directory needs.** That is the improvised step, written down; the item's own fallback, rejected since a declared setup covers it.

## Outcome

- `config/build-setup-here`: this repository's `clm.toml` sets `[build] setup` to install its five npm packages, skipping a listed directory with no `package.json`, every tracked `package.json` lies in a listed directory, and the test is marked `clm:reads **/package.json`.
- `config`'s Design lists the setting among this repository's, and says how a change that adds an npm package lists it: it modifies `build-setup-here`, which the package's requirement needs, so each build edits only its own `paths` (architecture audit, `path-ownership`).

## Constraints

- `settings-before-use`: `[build]` is already parsed by the installed `clm` (`config/build-setup-setting` is current, and `clm config get build` reads it), so the requirement's build writes the value directly; no `install-clm-for-build` item is needed.
- `unattended-commands`: every install runs with `--ignore-scripts`; each package's `.npmrc` sets it too.
- The setup doesn't install Playwright's Chromium: it lives in a per-machine cache, not the worktree, and `ci-check.sh` installs it (`ci/browser-install`).

## Open questions

- **No allow rule for `clm setup`.** `.claude/settings.json` allows exactly the rules `clm init` seeds (`clm/session-commands`), and `20261109-a-build-s-tests` seeded none for `clm setup`, leaving it to the permission mode the human runs builds under. Once this setting lands, every build here runs `clm setup` first, and a denial stops it with `stopped: setup failed`. The change adds no rule, as that change chose; if builds start stopping on it, the rule belongs in `init/init-seeds-permissions` with a note migration, a change of its own.
