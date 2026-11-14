---
title: "The web app keeps a derived, disposable cache"
date: 2026-10-22
status: accepted
---

The `web` Worker caches what it derives from git and GitHub:
- In the storage of each repo's `Replica` Durable Object: the repo's derived state, its spec files, marker index, ledgers, refs, pull requests and events, and the views computed from them, kept up to date by its webhooks (`web-repo-replica`). KV holds no snapshot and no marker index.
- In the Workers Cache API: the views for 30 seconds, and the live state the spec-branch reviews read for 60.

`git-is-truth` allows this: a service may keep such a cache as long as it can be deleted and rebuilt from git with nothing lost, and no check, gate or approval reads it. An approval is never kept in the cache: it goes to git, as an `approved` record in the person's web ledger (`per-writer-ledgers`).

Alternatives considered:
- A browser-only app that reads GitHub on every page load, which keeps `git-is-truth` as it was. Rejected for slow loads (coverage means reading every test file) and no live updates.
- A site generated in CI for each repo. No shared place for a team's repos, no approving in the UI, and stale between merges.
