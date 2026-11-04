---
title: "PRs that change only doc-only paths run just `clm check` in CI"
date: 2026-10-23
status: superseded
superseded_by: land-locally
---

Most PRs in this repository are specs, plans, claims and backlog items. fmt, clippy, the wasm builds and the tests can't fail on a diff that changes no code, so running them on those PRs, and again on their merge commits, only costs time. `.github/scripts/ci-check.sh` diffs the tree against `CLM_CHECK_BASE` (`HEAD^1` in the `check` and `merge` jobs) and, when every changed path is doc-only, runs `clm check --no-detectors` alone. The detectors read only code and manifests, so the light path doesn't install their tools.

The list names what is safe to skip, not what is code: `specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `README.md` and `CLAUDE.md`. A path not on it gets every check, so a new directory fails safe. The list holds only while no test reads those paths from the repository itself, apart from `clm check`'s run of it, and a test enforces that.

`merge-tested-tree` still holds. `main`'s code was checked when it landed, and a doc-only PR leaves the code unchanged, so the code in its merge commit was already checked.

One exception is accepted (2026-10-23): `web/` reads two doc-only inputs when it's built, the guide from `docs/guide/*.md` (`guide-from-docs`) and the invited logins from `specs/web/SPEC.md`'s Contract (`invite-only-web`). A doc-only PR can break either, for example with an unlinked guide page or a malformed list, and its light run won't notice. The next code PR's `web/` tests fail on it, and so does the release's deploy, loudly and before anything is served. Rejected: taking those paths off the doc-only list, and running only `web/`'s build and tests on PRs that touch them. Both cost CI time and complicate `doc-only.sh` for a failure that is rare and caught before it ships.

Rejected:
- **A list of code paths**: a new code directory would skip the checks until someone added it.
- **Skipping the `check` job with an `if:`**: `merge` needs `check`, so it would be skipped too.
- **Using the gate tier**: plan and task PRs are `code` to the gate but usually touch only `changes/`.

**Amended 2026-10-29** (`20261029-refounding`). `web/` no longer reads the guide (`guide-from-docs`, amended); the invited logins are the one doc-only input left in the exception.
