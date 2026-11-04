---
title: "The web app keeps a derived, disposable cache"
date: 2026-10-22
status: accepted
---

The `web` Worker caches what it derives from git and GitHub:
- In Cloudflare KV: per-commit snapshots of the spec graph, coverage, changes, plans and backlog, plus each repo's marker index. A push webhook keeps these fresh.
- In the Workers Cache API, for up to 60 seconds: PR, CI, claim and event state.

(Amended 2026-11-02, `20261102-web-repo-replica`: KV no longer holds snapshots or the marker index. Each repo's derived state, its spec files, marker index, ledgers, refs, pull requests and events, and the views computed from them, lives in the storage of the repo's `Replica` Durable Object, kept up to date by its webhooks (`web-repo-replica`). The same rule holds for it: it can be deleted and rebuilt from git with nothing lost, and no check, gate or approval reads it. The Cache API keeps the views for 30 seconds, and the live state the spec-branch reviews read for 60.)

`git-is-truth` is amended to allow this: a service may keep such a cache as long as it can be deleted and rebuilt with nothing lost, and no check, gate or approval reads it. Approvals still go to GitHub as the `spec-approved` label.

Alternatives considered:
- A browser-only app that reads GitHub on every page load, which keeps `git-is-truth` as it was. Rejected for slow loads (coverage means reading every test file) and no live updates.
- A site generated in CI for each repo. No shared place for a team's repos, no approving in the UI, and stale between merges.
