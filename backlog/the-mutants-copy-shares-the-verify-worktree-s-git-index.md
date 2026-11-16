---
title: "The mutants' copy shares the verify worktree's git index"
kind: bug
components: [ci]
---

`ci/mutate-script` has `mutate.sh` run `cargo mutants --copy-vcs true`, so a test that asks git about this repository passes in cargo-mutants' copy of the tree. The verify tree is a linked worktree, whose `.git` is a `gitdir:` file, so the copy points at the same worktree git directory, index and refs as the verify tree itself. The code review of the build that added the flag (impl/ci, 2026-11-16) raised these consequences, none of which a test or the fixture covers:

- A mutated file looks modified to git in the copy, so a cited test that checks the working tree against the index or `HEAD` (such as `git status --porcelain`) fails for every mutant, and the requirement is recorded `killed` when its test proves nothing.
- Copies tested at the same time, and the verify tree's own git commands, share one index: a git command that writes it can hit `index.lock`, failing a mutant or the baseline, or write the copy's stat data into the verify tree's index.
- The fixture is a `git init` directory, not a `gitdir:` file, so it passes for a shape the real run never copies: a worktree made with relative paths (`worktree.useRelativePaths`, git 2.48+) writes a `gitdir:` that doesn't resolve from the copy in the temporary directory, and every block then fails its baseline.
- Run from a main clone rather than a worktree, the whole object store is copied into each build directory.

A fix likely gives the copy a git repository of its own (for example a `git worktree add` or a shared clone per pass) instead of the verify tree's `.git`, which changes what `mutate-script` states, so it needs a spec change.
