---
title: "clm sync carries an unrelated tracked edit through the rebase, and fails when the settings don't load"
date: 2026-10-26
status: accepted
---

`sync-ignores-untracked` narrowed the refusal for untracked files, but any modified tracked file still stopped `clm sync`. A human's edit to `.claude/settings.json`, left before an unattended `propose` session started, stopped the session's `clm sync` on a `spec/*` branch, and the only way on was a hand `git stash`, which on this machine shares one stash list across every worktree and session.

A tracked edit is a risk to sync in three ways: the rebase writes over it, sync's own commits sweep it in, or a step that judges `HEAD` reads it in `HEAD`'s place. So `clm sync` refuses only over those: any staged change (sync's `Re-apply`, `Archive` and `Activate` commits commit the whole index, and `--autostash` would hand it back unstaged), a modified file under `specs/`, `changes/` or `backlog/` (the trees sync writes), a modified file `origin/main` changed since the merge base (the rebase writes it, so putting the edit back could conflict), and on `impl/*` any modified file (the archive step reads markers and the split-item step measures the node from the working tree, standing in for `HEAD`). Any other edit rides through `git rebase --autostash`, which keeps it in the rebase's own per-worktree state rather than the shared stash list, and puts it back when the rebase finishes; since main didn't touch the path, putting it back never conflicts.

In the same step, the settings: `clm sync` read `clm.toml` only for the split-item step, and when it didn't load (an installed `clm` older than a key `main` added) it printed `size not checked` and still said `sync ok`. `clm check` and `clm land` load the same settings and fail on them, so `sync ok` promised a branch they would reject. `clm sync` now loads the settings on every branch after the rebase and exits 1 with the error when they don't load. A session running an outdated `clm` then learns it at its first sync, not at land.

Alternatives considered:
- Refusing only under the spec paths `autosync` snapshots (`specs/`, `changes/`, `backlog/`, `docs/`, `ARCHITECTURE.md`, `CLAUDE.md`). Those aren't what sync can lose: an edit to `docs/` main never touched is safe through `--autostash`, and an edit to `src/` main did change is not.
- Stashing with `git stash push` and popping around the rebase. The stash list is shared by every worktree of the repository, so another session could pop sync's entry, and a rebase stopped on a conflict would leave the entry on that shared list.
- Carrying every edit through on `impl/*` too. The archive and split-item steps would judge an uncommitted test citation or code size as `HEAD`'s, and could archive a change the branch hasn't completed. Builds run in their own worktrees, so a stray edit there is rare.
- Keeping `size not checked` as a skipped step. It is one line in a long report and the outcome line contradicts it; the session that hit it ran a whole build without the size check and never knew.
