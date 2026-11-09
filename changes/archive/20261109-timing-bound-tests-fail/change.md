---
status: done
resolves: []
---

## Intent

Corrects the components of `backlog/timing-bound-tests-fail-when-several-builds-land-checks-run-at` to `vcs`, `plugin.hooks.guard`, `orchestrator.runner` and `land`, the components whose tests fail under a loaded host, so the next triage drafts it as a code-only fix in each.

The design stands: `check-runs-one-at-a-time`, as amended, lets builds' `--check-only` runs overlap on purpose (`worktree-check-counts`), and `tests-run-concurrently` and `land-checks-stay-fast` require this repository's tests to pass beside other runs and to be deterministic. Serialising the checks again would undo that amendment, so the fault is in the four tests' wall-clock bounds, and each lies in its own component with its requirement's text unchanged. The item names only `land`, so it is left for the next draft rather than fixed here; its body records this judgement.
