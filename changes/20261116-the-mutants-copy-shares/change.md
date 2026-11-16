---
status: proposed
resolves: [the-mutants-copy-shares-the-verify-worktree-s-git-index]
---

## Intent

Plan work under the freeze (`backlog/a-passing-check-proves-less-than-the-spec-claims.md`): the mutation pass's copies of the tree must answer git as the checkout does without sharing its git directory. `ci/mutate-script` now has `mutate.sh` point `cargo mutants` at a tree with a git repository of its own, a clone sharing the checkout's objects, at its `HEAD`, holding its branches and remote-tracking branches, every tracked file marked unchanged, and makes the `cargo-mutants` fixture the same way, so the fixture proves the shape the pass really copies.

## Problem

`20261116-the-mutants-copy-is-a-git-repository` had `cargo mutants` copy the checkout's `.git` (`--copy-vcs true`). The verify tree is a linked worktree, whose `.git` is a `gitdir:` file, so every copy links back to the verify tree's own git directory. The code review of its build found four consequences no test or fixture covers:

- A mutated file looks modified to git in the copy, so a cited test that compares the working tree with the index or `HEAD` fails for every mutant, and the requirement is recorded `killed` when its test proves nothing: the false pass this plan exists to remove, inverted.
- The copies and the verify tree share one index: a git command that writes it can meet `index.lock` and fail a mutant or the baseline, or write a copy's stat data into the verify tree's index.
- The fixture is a `git init` directory, whose `.git` is a directory, so it passes for a shape the pass never copies: a worktree made with relative paths (`worktree.useRelativePaths`, git 2.48+) writes a `gitdir:` that doesn't resolve from the copy, and every block then fails its baseline while the fixture passed.
- Run from a main clone, the whole object store is copied into each of the tool's build directories.

## Approaches

- **A clone of the checkout per pass, sharing its objects, every tracked file marked unchanged (taken).** Before its first Rust block the script makes, in its temporary directory, a repository whose `.git` is a directory of its own: a clone sharing the checkout's object store (`git clone --shared`, whose `objects/info/alternates` names the store by an absolute path, so it resolves from any copy), at the checkout's `HEAD` commit, holding the checkout's branches and remote-tracking branches, so a test that reads `main` or `origin/main` there reads what it reads in the checkout. Every tracked file is marked unchanged in its index (`git update-index --assume-unchanged`), so git there reports the working tree as `HEAD` whatever the tool writes into a copy. `cargo mutants` runs in that tree with `--copy-vcs true`: each copy gets its own index and refs and a few files of git metadata, never the object store, and nothing a copy runs touches the checkout's git directory. The fixture's crate is made a committed repository and then cloned the same way, and its blind test also asks whether its working tree matches `HEAD`: a mutated file that git still sees fails that test, so the fixture's verdict is wrong and the pass stops, with the real tool, at every pass.
- **A `git worktree add` per pass.** Not taken: its `.git` is again a `gitdir:` file, so the copies share that worktree's index, and with relative paths it doesn't resolve from the copy.
- **A git repository per copy.** Not possible from the script: `cargo mutants` makes the copies, one per build job, and offers no hook to run before each.
- **Mutate in place (`--in-place`).** Not taken, as before: a stopped run leaves mutants in the verify tree.
- **Accept it.** Not taken: the first consequence is a passing check proving less than it claims, and the fixture proving a shape the pass doesn't run is the same.

## Outcome

`ci/mutate-script` states the tree `cargo mutants` runs in and the fixture's shape, with a scenario that the tree has a git directory of its own and one that the fixture is made the same way. `ci`'s build changes `.github/scripts/mutate.sh` and its tests in `crates/clm/tests/ci_check.rs`. The verify tree's own index and refs are never written by the pass.

## Edge cases and workarounds

- Marking every tracked file unchanged is a workaround for the tool writing mutants into a tree git can see: it hides each mutated file from `git status` and `git diff` in the copy. A test that relies on git noticing its own edits to a tracked file of this repository would be fooled by it; such a test would also write into this repository, which `tests-run-concurrently` forbids.
- The tree is the checkout's `HEAD`: a hand run judges committed code only, not edits left uncommitted in the checkout. The verify tree is always at a commit, so the pass is unchanged.
- The clone holds no other git configuration of the checkout (its `user.name`, hooks); no test of this repository reads them from this repository.

## What it removes

Nothing.

## Constraints

- No file, key or format an adopter holds changes: the script is this repository's own.
- `clm land` gains no check; the pass runs after the land (`mutation-after-land`).
