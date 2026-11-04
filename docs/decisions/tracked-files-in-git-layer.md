---
title: "The tracked files a node's paths match are listed once, in the git layer"
date: 2026-10-27
status: accepted
---

`clm survey --nodes` (`init`) and `clm size` (`check`) both ask which tracked files a node's `paths` match, and each ran its own `git ls-files -z -- ':(glob)<path>'`. The copies had already drifted: `size`'s passed `--deduplicate`, so a file in conflict counted once, and kept names that aren't UTF-8 exact; `init`'s did neither. Two answers to "which files does this node own" would let a node map that `clm survey --nodes` accepts be sized differently by `clm size`.

The question is now answered in one place, `git`'s `tracked_files(root)` and `tracked_matching(root, globs)` (`git/tracked-files`), and both nodes call it. `clm survey`'s whole-repository listing goes through `tracked_files` too, so it reads a conflicted file once.

Rejected: making the helper public from `clm-check`, as the backlog item first suggested. `init` doesn't depend on `check` today (`depends_on: [model, config, git]`), and taking on the whole of `check` for one `ls-files` call widens `init`'s context and dependency graph for nothing. `git` is the plumbing layer every node that shells out already depends on (`git-plumbing-layer`), and plumbing several nodes copy belongs in a shared node (`slim-nodes`). Also rejected: leaving the copies and testing that they agree, which keeps two implementations to maintain and only finds drift after it happens.

`check`'s coverage scan lists untracked files too (`--others --exclude-standard`) and `sync` queries single paths and merge stages; those are different questions and keep their own calls.
