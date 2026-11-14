---
title: "Decisions written while the repository was private: repair their links and note what changed"
kind: idea
---

## Intent

CLM's development moved to a public repository with a fresh history. Some decisions under `docs/decisions/` still rest on the old situation, and readers can't tell which reasons still hold. There are two kinds of stale reference.

## Broken links

`effort-records`, `human-layers`, `refounding-edits-specs-directly` and `yes-no-review` point at `changes/archive/20261029-refounding`, which isn't in this repository. Repair or remove each link. Don't name that archive in a backlog item's `origin`: `clm check` fails on it.

## "The repository is private"

These decisions give the repository being private as a reason:

- `binary-from-releases`: downloads use `gh release download` because the repository was private. A public release doesn't need `gh`, so the install requirement could change.
- `guard-blocks-push-to-main`: branch protection wasn't available on a private repository on GitHub Free. It's available now. Leaving pushes to `main` unrestricted is still the choice, because `clm land` and the release workflow's version bump both push there directly.
- `land-locally`: cites metered Actions minutes on a private repository.
- `init-cli-and-skill`, `guide-from-docs` and `static-linux-binaries`: rejected options partly because the repository was private.
- `web-repo-replica`: matched the same search and needs reading to see whether it applies.

This list came from a pattern search, so search again before relying on it.

## The fix

A decision records why a choice was made, so don't rewrite its reasoning as if it had never been true. Add a note to each affected decision saying the repository is now public and whether the reason still holds, and flag any choice worth reconsidering. Reversing a choice, such as dropping the `gh` requirement, is its own change.
