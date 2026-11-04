---
title: "The guard blocks pushes to main, and clm run always loads it"
date: 2026-10-22
status: accepted
---

Refines `gate-without-branch-protection`, which left direct pushes to `main` "forbidden by convention". The repo is private on GitHub Free, so branch protection and rulesets aren't available (`gh api` returns 403 for both). Claude shares the human's account, and `clm run` pre-approves `Bash(git *)` under `dontAsk`, so a session could push to `main` and skip the PR gate.

The plugin's guard (`guard-matches-writes`) now also blocks a `git push` whose destination is `main`: by refspec, with `--all` or `--mirror`, or implicitly from a checkout on `main`. The checkout is the hook event's directory, moved by `git -C` and by earlier `cd <dir>` segments it can resolve, so `cd .clm/worktrees/<item> && git push` from a session on `main` pushes the worktree's branch and passes. A `cd` it can't resolve leaves the event's directory, so it never guards less than without the `cd`. It also blocks `gh api` writes to `git/refs/heads/main`. `clm run` adds the guard to every session it launches through `--settings`, run by its own binary, so unattended sessions have it even when the plugin isn't loaded.

We didn't narrow `clm run`'s allowlist instead. Prefix rules are easy to step around (`git push origin x:main`), they'd cover only unattended sessions, and under `dontAsk` a narrow list denies git commands the flow needs.

Like the other guards, this is a backstop for accidents, not a boundary. A deliberately obfuscated command, or the contents API writing a file to `main`, still gets through. If the repo moves to a plan with branch protection, protection becomes the boundary and this stays as the early warning.
