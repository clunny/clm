---
status: proposed
resolves:
  - clm-wrap-up-reports-an-outside-glob-unmatched-when-it-names-only
---

## Intent

`clm wrap-up` judges `origin/main` with the same file listing `clm check` uses in a checkout of that commit, so an `outside` glob that covers only files under a directory whose name starts with `.` (`.azure-pipelines/`, `.gitlab/`, `.circleci/`, `.github/`) no longer reads as unmatched, and a session on such a project can reach `wrap-up ok`. Each violation wrap-up finds on that tree names it, starting `origin/main: `, so it isn't mistaken for one in the checkout. `clm-check` gains `run_listed(root, files)`, which takes the listing from its caller; the walk `run` falls back to outside a git work tree goes.

## Problem

Wrap-up checks `origin/main` in the directory `clm_git::extract_tree` writes, which isn't a git work tree. There `git ls-files` fails, so `paths-disjoint` reads no file at all and `unowned-files` walks the directory as the coverage scan does, skipping every directory whose name starts with `.` and `target/`. The rule as written (`check/unowned-files`) asks for that walk. On a project whose CI files live under `.azure-pipelines/` and are covered by an `outside` glob, `clm check` passes while wrap-up reports `outside glob .azure-pipelines/** matches no file outside every component`; removing the glob fails `clm check` and every land instead, so no clean checkpoint is reachable. The line names `specs/SPEC.md` with nothing saying it was `origin/main`'s, so it reads as the checkout's.

## Approaches

- **The caller passes the commit's listing (chosen).** Checkpoint lists the extracted commit's files with `git ls-tree -r --name-only` (the same set `git ls-files --cached` gives in a checkout of it, gitlinks and symlinks included) and calls `run_listed`. Both rules that read the tracked files (`paths-disjoint`, `unowned-files`) then see exactly what `clm check` sees; `paths-disjoint`, which reports nothing on the extracted tree today, starts working there too. `run` keeps its signature for its other callers (`clm check`, `clm rebase`'s judge, `clm health`), all of which run in a git work tree.
- **Walk hidden directories too, outside a work tree.** The extracted tree holds exactly the commit's files, so a full walk is close, but it still misses gitlinks, which `checkout-index` doesn't write as files, still leaves `paths-disjoint` blind, and keeps two ways of listing the same thing. Rejected (`reuse-first`: git already lists a commit's files).
- **Check `origin/main` in a temporary git worktree, as `clm rebase`'s judge does.** Exact, but adds a worktree to make and remove on every wrap-up to fix a listing. Rejected as heavier than the fault.

## Outcome

- `check/run-listed` (added): `run_listed(root, files)` runs every rule `run` runs, reading `files` as the index's files for `paths-disjoint` and `unowned-files`.
- `check/unowned-files` (modified): outside a git work tree with no listing, `run` reads no file and reports nothing; the walk fallback and its code go (`no-dead-code`).
- `check`'s Contract names `run_listed`; its Design says which callers use which.
- `orchestrator.checkpoint/checkpoint-reports-unsafe-work` (modified): judges `origin/main`'s tree with `run_listed` on the commit's `ls-tree` listing, and prefixes each check violation with `origin/main: `. Needs `check/run-listed`.

## Constraints

- Edge case known: the marker scan (`check.coverage`'s `scan`) still walks an extracted tree as before, skipping hidden directories, so a `req:` marker in a file under a hidden directory isn't seen by wrap-up's `complete` lines or marker rules. No test of this repository or a typical project cites from there; left as it is (`yagni`).
- `clm-checkpoint` lists through `clm-git`'s `run`/`git` calls (`git-plumbing-layer`); no new `vcs` call is needed.
- No workaround is needed.

## Open questions
