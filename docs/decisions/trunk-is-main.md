---
title: "The trunk is a branch named main"
date: 2026-11-07
status: accepted
---

A repository whose trunk is `master` can't land: `clm land` refuses with "no `main` to land on", and nothing says so earlier (GitHub issue #283). The issue asked for a setting naming the base branch, filled by `clm init` and read by every command and skill in place of `main`.

Decided: CLM supports only a trunk named `main` (`trunk-named-main`), and `clm init` says so up front, with the rename command, in a repository without one (`init/init-names-missing-main`).

Why:
- A configurable trunk is a lasting cost on CLM, not a one-off: `main` and `origin/main` appear in roughly 800 places across the specs, the code, the skills and the generated prompts, and every future requirement, skill and test would have to use the setting instead. A missed literal is a bug that shows only in a repository on another branch name, which this repository's own checks never exercise.
- The benefit is one step an adopter takes once: `git branch -m master main`, plus moving the remote's default branch, CI triggers and branch protection keyed to `master`.
- What #283 reported as broken, a missing `main` found only at the first land, is fixed by the `clm init` warning alone.

Rejected:
- A `trunk` setting in `clm.toml`, defaulting to `main`, with a `clm base` command for skills and prompts: correct, but the cost above, for every change after it.
- Detecting the trunk from `origin/HEAD`: absent with no remote (`host = "none"`), and it can be unset or differ per clone, so two clones could disagree.

Revisit if adopters keep asking for it and renaming is often impossible for them, such as where an outside system they can't change pins the branch name.
