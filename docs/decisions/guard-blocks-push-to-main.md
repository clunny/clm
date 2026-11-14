---
title: "The guard blocks pushes to main, and every unattended build loads it"
date: 2026-10-22
status: accepted
---

Only `clm land` pushes `main` (`land-locally`), and without a guard that is a convention. A host's branch protection can't be counted on: when this was decided the repository was private on GitHub Free, where branch protection and rulesets aren't available (`gh api` returned 403 for both), and a repository on a bare git server has none. Claude shares the human's account, and an unattended build's `git` commands are pre-approved, so a session could push to `main` and skip the land's checks.

So the plugin's guard (`guard-matches-writes`) blocks a `git push` whose destination is `main`: by refspec, with `--all` or `--mirror`, or implicitly from a checkout on `main`. The checkout is the hook event's directory, moved by `git -C` and by earlier `cd <dir>` segments it can resolve, so `cd .clm/worktrees/<item> && git push` from a session on `main` pushes the worktree's branch and passes. A `cd` it can't resolve leaves the event's directory, so it never guards less than without the `cd`. It also blocks `gh api` writes to `git/refs/heads/main`. Unattended builds and drafts run as subagents of the human's session, so the plugin's hook guards them as it guards the session (`builds-in-subagents`).

We didn't narrow the unattended builds' allowlist instead. Prefix rules are easy to step around (`git push origin x:main`), they'd cover only unattended work, and a narrow list denies git commands the flow needs.

Like the other guards, this is a backstop for accidents, not a boundary. A deliberately obfuscated command, or the contents API writing a file to `main`, still gets through. If the repo moves to a plan with branch protection, protection becomes the boundary and this stays as the early warning.

## The rule in full

`ARCHITECTURE.md`'s entry `land-locally` states this in short; the rule in full:

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
