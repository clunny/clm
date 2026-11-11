---
title: "clm wrap-up reports an outside glob unmatched when it names only files under dot-directories"
kind: bug
components: [orchestrator.checkpoint, check]
---

In a project hosted on Azure DevOps, after its first spec branch landed, `clm wrap-up` ended `wrap-up incomplete` with `specs/SPEC.md: check/unowned-files: outside glob .azure-pipelines/** matches no file outside every component`, while `clm check` passed on the same commit, in the main checkout and in `.clm/land-tree`. Removing the glob is no way out, since `clm check` and every land then fail on the files it covered, so the session could never reach a clean checkpoint.

Cause: `clm-checkpoint` checks `origin/main` in a tree `clm_git::extract_tree` writes to a temporary directory, which is not a git work tree. There `tracked_files` fails, and `clm-check`'s `unowned` falls back to walking the directory, skipping every directory whose name starts with `.`. Files under `.azure-pipelines/` (or `.gitlab/`, `.circleci/`, `.github/` when no component owns them) are never listed, so an `outside` glob covering only them matches nothing.

Expected: wrap-up's check lists the same files `clm check` does on the same commit, for example from `git ls-tree -r` of the commit it extracted, and a wrap-up failure names the tree it checked.
