---
title: "clm sync refuses only over work its own steps could lose or commit"
date: 2026-10-24
status: accepted
---

`clm sync` refused to run whenever `git status --porcelain` printed anything, calling it "uncommitted changes" without naming a path. That stopped safe syncs: another session's `.claude/worktrees/` directory (Claude Code's `EnterWorktree` puts a locked worktree inside the checkout, neither tracked nor ignored) and Playwright's `test-results/` output both blocked an `impl/*` branch whose tracked files were clean, and the session couldn't tell generated output from a forgotten new file.

The refusal guards two things. A modified, staged or deleted tracked file is what the rebase can lose. An untracked file under `specs/` or `changes/` is what sync's own commits would sweep in: the regenerate step commits `specs/` with `git add -A -- specs` (`regenerate-applied-specs`), and the archive step renames `changes/<id>/` into the archive and commits with `git add -A -- changes` (`archive-on-sync`). So uncommitted work is exactly those: tracked changes, and untracked, unignored files under `specs/` or `changes/`. `clm sync` prints `uncommitted <path>` for each and exits 1. Every other untracked file is left alone and never stops a sync: nothing sync does stages or removes it, and if the rebase would have to write over one, git refuses with the path and sync reports that as a git failure. Ignored files were never counted.

Alternatives considered:
- Narrowing the two commits to per-file staging and never refusing over untracked files. An untracked file under `changes/<id>/` would then ride the directory rename into the archive but stay unstaged, and a forgotten delta would silently stay out of the branch.
- Ignoring all untracked files. A stray file under `specs/` or `changes/` would be committed by sync without anyone choosing it.
- Having `clm init` exclude `.claude/worktrees/`. It fixes one directory, not the rule, and the excluded directory is Claude Code's, not CLM's.
