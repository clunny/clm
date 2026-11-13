---
title: "The guard blocks pushes to main, and clm run always loads it"
date: 2026-10-22
status: accepted
---

Refines `gate-without-branch-protection`, which left direct pushes to `main` "forbidden by convention". The repo is private on GitHub Free, so branch protection and rulesets aren't available (`gh api` returns 403 for both). Claude shares the human's account, and `clm run` pre-approves `Bash(git *)` under `dontAsk`, so a session could push to `main` and skip the PR gate.

The plugin's guard (`guard-matches-writes`) now also blocks a `git push` whose destination is `main`: by refspec, with `--all` or `--mirror`, or implicitly from a checkout on `main`. The checkout is the hook event's directory, moved by `git -C` and by earlier `cd <dir>` segments it can resolve, so `cd .clm/worktrees/<item> && git push` from a session on `main` pushes the worktree's branch and passes. A `cd` it can't resolve leaves the event's directory, so it never guards less than without the `cd`. It also blocks `gh api` writes to `git/refs/heads/main`. `clm run` adds the guard to every session it launches through `--settings`, run by its own binary, so unattended sessions have it even when the plugin isn't loaded.

We didn't narrow `clm run`'s allowlist instead. Prefix rules are easy to step around (`git push origin x:main`), they'd cover only unattended sessions, and under `dontAsk` a narrow list denies git commands the flow needs.

Like the other guards, this is a backstop for accidents, not a boundary. A deliberately obfuscated command, or the contents API writing a file to `main`, still gets through. If the repo moves to a plan with branch protection, protection becomes the boundary and this stays as the early warning.

## The rule in full

`ARCHITECTURE.md` stated this as the entry `guard-blocks-push-to-main` until 2026-11-13, when it was merged into `land-locally` (`human-layer-entries-stay-short`). The text still holds:

The guard of each agent harness whose adapter declares one SHALL block every `git push` from a session whose destination is `main`, by refspec, `--all`, `--mirror` or implicitly from a checkout on `main`, unless the push runs from a git repository that isn't under CLM (one holding no `specs/SPEC.md` at its top or in any subdirectory), where no `clm land` owns `main`; a push whose repository the guard can't tell counts as under CLM. Every unattended build SHALL load the guard. A `clm` command that pushes or deletes a branch for a session, whose own `git push` the guard never sees, SHALL refuse `main` itself.

**Scenario: implicit push**
- GIVEN a session on `main` in a repository under CLM
- WHEN it runs `git push`
- THEN the guard blocks it

**Scenario: a repository not under CLM**
- GIVEN a session that runs `git push -u origin main` from a scratch repository with no `specs/SPEC.md`
- WHEN the guard judges it
- THEN the push goes through

**Untested**
An agent of the generic tier runs no guard, so nothing stops its direct `git push` to `main` but the host's branch protection, where the host has it, and the instruction `AGENTS.md` carries; `clm ledger verify` finds a `main` commit no `landed` record names after the fact (`agent-harness-adapters`).
