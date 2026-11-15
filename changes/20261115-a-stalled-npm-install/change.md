---
status: applied
resolves: [a-stalled-npm-install-hangs-verify-with-no-end]
---

## Intent

An npm install that stalls must fail or recover within minutes, not hold a verify or a land for ever. Every `npm ci` and `npm install` that this repository's three check scripts run goes through one small script that ends the install at a time limit and tries it once more. The fix is outside the plan's freeze, so a question item holds its build back until the human says whether to make the exception.

## Problem

On 2026-11-15 `clm verify --due`, run by the build loop, reached the coverage pass (`.github/scripts/coverage.sh`, run by `mutate.sh`). `npm ci --ignore-scripts` in the verify tree's `plugin-panel/` then sat for over 11 minutes with one second of CPU and no output: its one connection to the npm registry had stalled silently. Nothing timed out. The human noticed the idle machine and killed the process by hand, `mutate.sh` failed with status 143 and verify recorded nothing. The registry answered normally a moment later.

No requirement bounds an npm step. The same commands run in `mutate.sh` (StrykerJS's install) and five times in `ci-check.sh`, which is `[land] check`: a stall there holds the clone's check lock, and every land of the clone behind it (`backlog/a-hung-land-check-holds-every-land-of-the-clone` describes that for a hanging test).

## Approaches

- **Taken: one script, `.github/scripts/npm-install.sh`, that runs an npm command under a time limit and retries a run it ended, once.** The three scripts call it wherever they ran `npm ci` or `npm install`. The limit is 300 seconds for each attempt, which a cold install of the largest package here stays well under, and `NPM_INSTALL_TIMEOUT` names another, which the tests use to prove the limit in a second. A stall is a fault of the network, not of the tree, and both commands are safe to run again (`npm ci` replaces `node_modules/`; `npm install --no-save` adds to it), so one retry turns the fault seen on 2026-11-15 into a passing run a few minutes late instead of a verify that records nothing or a land that loses its check.
- **Not taken: npm's own `fetch-timeout` and `fetch-retries`.** Their defaults (five minutes, two retries) were in force during the stall and didn't end it in 11 minutes. They time one request's socket, not the install, so they give no bound a test can prove with a fake `npm`.
- **Not taken: coreutils `timeout`.** `ci-check.sh` is written to run on macOS's bash 3.2 too, and macOS has no `timeout`. The script ends the install itself, with the shell's own job handling.
- **Not taken: a time limit on the whole `[verify] check`, `[verify] mutate` or `[land] check` command.** It would bound every hang, not only npm's, but a whole run's time varies with the machine's load by a factor of several, so its limit must be loose, and it can't retry the one step that stalled. It is the idea in `backlog/a-hung-land-check-holds-every-land-of-the-clone`, which stays filed.
- **Not taken: accepting the hang.** The plan's default answer to a defect is "accept", but this one has no bound: the loop that builds the plan's phases waits on verify, and nothing but a person watching ends it.

## Outcome

- `ci/npm-installs-bounded` (new): `.github/scripts/npm-install.sh` ends an npm run at the limit, runs it once more, and fails with status 124 when the second is ended too; a run that fails by itself is not retried. `ci-check.sh`, `coverage.sh` and `mutate.sh` run every `npm ci` and `npm install` through it.
- `ci`'s Design gains a paragraph saying why, with the approaches not taken.
- `backlog/the-fix-for-a-stalled-npm-install-is-outside-the-plan-s-freeze` (`kind: question`) blocks the requirement until the human answers.
- No capability test sees a difference: none observes how long an install may take, and the scripts' commands and results are otherwise unchanged.

## Constraints

- The existing requirements that name `npm ci --ignore-scripts` and `npm install --no-save --ignore-scripts` (`web-checks`, `selected-tests`, `coverage-script`, `mutate-script`) stay true as written: the same npm commands run with the same arguments, in the same directories.
- The script adds no dependency: bash and what the scripts already use.

## Edge cases

- **An install that is slow, not stalled**, and passes 300 seconds, is ended and retried, and fails the script when the second attempt is as slow. npm's cache makes the second attempt faster. A machine or network that slow sets `NPM_INSTALL_TIMEOUT`.
- **`[build] setup`'s `npm ci`** (`config/build-setup-here`) is left as it is. A stall there holds one build's `clm setup`, which the session runs in the foreground under its tool's own time limit (`bounded-test-runs`), and holds no lock and no other session.
- **The release and site workflows' `npm ci`** are left as they are: GitHub ends a job at its own limit, and a person dispatches and watches them.
- **`npm test` and `npx`** aren't installs and aren't bounded here: a hanging test is `backlog/a-hung-land-check-holds-every-land-of-the-clone`.
- **npm's child processes.** With `--ignore-scripts` an install runs no package script, so ending the `npm` process ends the install.
