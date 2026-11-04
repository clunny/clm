---
title: "Runs on main keep CI's Rust cache warm, dispatched by each merge"
date: 2026-10-23
status: superseded
superseded_by: land-locally
---

GitHub scopes Actions caches to the ref that saved them: a pull request's run can restore its own ref's caches and the default branch's, never another PR's. CI never runs on `main` here, because the `merge` job lands PRs with the workflow token, and its pushes start no workflow (`merge-tested-tree`). So no PR ever found a cache, and every job compiled the workspace from scratch.

A separate workflow, `cache.yml`, runs `ci-check.sh` on `main` and saves the cache there. The `merge` job dispatches it after each landing, since `workflow_dispatch` is the one event the workflow token can start. A daily schedule covers a dispatch that fails, and a concurrency group with `cancel-in-progress` keeps only the newest run. Every job uses one `shared-key`, so `check`, `gate` and `merge` share the cache, and only runs on `main` save, since a PR's saves help nothing but that PR's later runs.

Rejected:
- **CI on pushes to `main`**: needs a PAT or an App token for the merge push to start it.
- **A schedule alone**: the cache lags `main` by up to a day and misses after every `Cargo.lock` change.
- **Saving from PR runs too**: each PR then writes a cache entry only it can read, which spends the repository's 10 GB and time spent saving for little gain.
