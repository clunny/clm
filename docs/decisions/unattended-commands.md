---
title: "Unattended sessions may run a narrow set of npm commands, with install scripts off"
date: 2026-10-22
status: accepted
---

Beyond `clm run`'s own commands, this repo's `.claude/settings.json` lets unattended sessions run `cargo`, `npm ci`, `npm install --ignore-scripts`, `npm run`, `npm test` and `npx playwright test`. Not `npm install` without `--ignore-scripts`, not any other `npx` command, and not `rustup`. `web/.npmrc` sets `ignore-scripts=true`, so no dependency's install script runs whichever command installs it.

**Why.** Sessions run under `dontAsk` with no one watching. The `web` task of `20261022-web-ui` needs npm to build and test `web/`. A package's install scripts are the usual way a compromised dependency runs code on the machine that installs it, and `npx <pkg>` downloads and runs any package by name. With both closed, the third-party code a session runs is what its own build and tests load, as with `cargo`'s build scripts, and new dependencies show in the PR's `package.json` and lockfile diff.

**Adding dependencies.** The human chose to let sessions add dependencies unattended, with `npm install --ignore-scripts`, rather than add each one by hand. The PR review is where a new dependency gets looked at.

**Browsers.** Playwright's Chromium and the system libraries it needs (`npx playwright install --with-deps chromium`, which uses sudo) are installed outside sessions: once per machine by the human, and by CI's browser-test step.

**Alternatives.** `Bash(npm *)` and `Bash(npx *)` are simpler but run install scripts and arbitrary packages unattended. Allowing only `npm ci` would make every new dependency a step for the human.
