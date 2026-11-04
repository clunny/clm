---
title: "The browser and pane capabilities are tested by two npm packages of the capability node's own"
date: 2026-10-30
status: accepted
---

Settles `backlog/capability-browser-boundary`, which the builder check of `20261030-capability-node` filed: `follow-from-browser`, `review-capability-cost`'s review page and `watch-beside-chat` are reached through the web app and the Claude Code pane, and every way of running a directory of `capability`'s from `web/`'s Playwright suite or `plugin-panel/`'s vitest suite needed a workaround (module resolution from outside each package, a fixed tsconfig `include`, mocks copied because `web` is optional, the whole Playwright suite on most lands). The human chose, on 2026-10-30, **packages of their own**: `capabilities/web/` and `capabilities/panel/`, each with its own `package.json`, lockfile and test runner, owned by `capability`. A second Playwright install is accepted.

**What each tests against** (`human-layer-proof`: an app's boundary is a driven browser; only what lies beyond the product is stubbed).
- `capabilities/web/`: a phone-sized Chromium page on the built app and the real Worker, both run locally by Miniflare, the runtime `wrangler dev` uses. GitHub is beyond the boundary: Miniflare's outbound-fetch hook answers the Worker from a fake GitHub backed by a real git repository, so an approval or a capability decision ends as a ledger commit the test reads, and `clm land` on a clone lands the approved branch. No Worker code changes for it: the GitHub URLs stay hard-coded, and the hook intercepts them. Miniflare runs in the test process, so the hook and the test share the fake's state.
- `capabilities/panel/`: the panel's plugin directory loaded by vitest as the engine loads it, through `hooks/hooks.json`'s `modules`, with `'claude-code'` aliased to a fake engine of the package's own, whose processes are real, so the pane runs the real `clm` against a repository where a build lands.

**No import edges.** Both packages reach what they test the way a person's tools do: the web package through `web/`'s build and the Worker's, the panel package through the plugin's manifest. Neither imports another node's file by a literal path, so `check/undeclared-imports` sees no edge, and `capability` keeps depending on the root alone. Depending on `plugin.panel` would have made every panel change select every CLI capability test; depending on `web` isn't allowed (`optional-nodes`).

**How they run.** `ci-check.sh` runs `capabilities/web/` after `web/`'s suite and `capabilities/panel/` after `plugin-panel/`'s, and either package when the selection names a file in it (`ci/web-checks`, `ci/selected-tests`). That covers the gap the graph leaves: `capability` can't depend on `web`, so a change to the web app or the panel never selects their capability tests through the dependant walk. The script maps files to its runner's syntax, as `graph-test-selection` leaves to it.

**Edge cases accepted, written down rather than worked around.**
- **No fake of Claude Code's plugin engine exists.** The package keeps its own, covering only what `register.tsx` calls, and it can drift from the engine. That the engine loads the module is `ci/panel-validate`'s check. `claude plugin test`, whose kit is the engine's own, was not chosen: it needs `claude` on the landing machine, a missing one would have to skip, which `check/skipped-tests` refuses, and the kit is early access.
- **The fake GitHub is ours.** No standard fake answers GitHub's REST and GraphQL from a git repository. It answers only the calls the two scenarios reach and fails a test on any other, so a new call the Worker makes shows up as a failing capability test, fixed in the package.
- **Three pins kept alike by hand.** `@playwright/test` in `capabilities/web/` follows the version `web/package-lock.json` resolves, `miniflare` the one its `wrangler` resolves, and the `worker-build` version `web/`'s `build:worker`. A mismatch costs a second download or a failed build, never a wrong pass.
- **Miniflare's bindings are a copy of `web/wrangler.toml`'s**, since Miniflare doesn't read the file. A binding the copy lacks fails a capability test that reaches it; one the scenarios never reach goes unnoticed, which costs nothing they prove.
- **Effort comes from fixture transcripts.** `review-capability-cost`'s history is made with `clm effort record --transcripts` and `clm land`, as a person's sessions would make it; the transcripts are Claude Code's, beyond the boundary.
- **Most code lands run the browser test.** `capability` depends on the root, which most code lands affect, so its files are selected and `capabilities/web/` builds the app and the Worker and runs a browser. The Worker is a dev build that reuses the wasm build the script has just made, and the package holds two specs.
- **The packages need what `web/` installs.** The app is built with `web/`'s own dependencies, so the script runs `web/`'s `npm ci` before `capabilities/web/`; the package runs nothing of `web/`'s but its build.

Rejected:
- **Running `capabilities/` from the existing suites**: the five workarounds above (`redesign-before-workaround`).
- **Proving the three at the CLI's boundary** (`clm capabilities`, `clm approve` with a ledger record), leaving the browser and pane to `web`'s and `plugin.panel`'s own tests, or **rewording the capabilities** to what the CLI proves: either leaves the hand-off between the app and the Worker, or the pane and `clm`, unproven at the boundary a person uses.
- **Stubbing the Worker's API in the browser**, as `web/`'s own suite does: it would test the app's half alone, and copy `web/`'s mocks.
