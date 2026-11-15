---
title: "A stalled npm install hangs verify with no end"
kind: bug
components: [ci]
---

Step: `clm verify --due`, run by the build loop, in the coverage pass that `.github/scripts/coverage.sh` runs (`ci/coverage-script`).

What happened: `npm ci --ignore-scripts` in the verify tree's `plugin-panel/` sat for over 11 minutes with 1s of CPU and no output. Its one connection to the npm registry had unacknowledged bytes in its send queue: a silently stalled connection. Nothing timed out, so verify would have waited indefinitely; the human noticed the idle machine and the process was killed by hand, after which `mutate.sh` failed with status 143 and verify recorded nothing. The registry answered normally a moment later.

Expected: the script's npm steps (`npm ci`, and the `npm install` of `@vitest/coverage-v8` beside it) run under a time limit, e.g. `timeout`, or npm's own fetch-timeout/fetch-retries settings, so a stalled fetch fails the step (and is retried) instead of hanging verify and the loop that waits on it. The same may apply to `[build] setup`'s `npm ci` in build worktrees.
