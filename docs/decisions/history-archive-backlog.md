---
title: "History in the archive, intent in the backlog, decisions one per file"
date: 2026-10-21
status: accepted
---

`specs/` and `ARCHITECTURE.md` hold only the present (`spec-is-current`). Done and abandoned changes move to `changes/archive/<id>/`, which is never edited again (OpenSpec's archive convention). Git history covers line-level history. Decisions are one file each in `docs/decisions/`, so authors don't conflict on a shared log, and a reversal is a new decision that the old one points to via `superseded_by`. Ideas, deferred work, open questions and bugs are one file each in `backlog/`. Code PRs may add items (`fold-small-defer-large`: small discoveries are folded into the change, and only significant out-of-scope work is deferred), but editing or removing them needs a spec PR. Everything stays in git (`git-is-truth`). An issue tracker may mirror `backlog/` but never owns it. None of these are part of context bundles by default (`spine-scoped-context`).
