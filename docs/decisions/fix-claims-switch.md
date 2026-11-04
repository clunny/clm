---
title: "A fix claim switches the checkout to its branch, or refuses before claiming"
date: 2026-10-27
status: accepted
---

`clm claim <node> --fix backlog/<slug>` pushed `impl/<node>` and printed its name, and the session was then told to switch to it. When a stale local `impl/<node>`, left by a build that had already landed, was checked out in another worktree, the switch couldn't happen and nothing said so: the session stayed on its spec branch. A stale local branch checked out nowhere is a quieter trap: `git switch impl/<node>` checks out the old tip, not the new claim.

So the fix claim does the switch itself (`code-fix-resolves-bug`): the session that claims a fix builds it in the checkout it claimed from, and the claim is the one place that knows the claim commit. Before pushing, it checks what would make the switch fail, and refuses with the reason and the remedy while nothing has been claimed: uncommitted changes to tracked files, `impl/<node>` checked out in another worktree (named by path), or a local `impl/<node>` with commits `origin/main` lacks. Otherwise it pushes, resets the local branch to the claim commit with `git switch -C` and tracks `origin/impl/<node>`. A switch that still fails after the push exits 2 saying the claim stands but the checkout couldn't switch. A plain `clm claim <node>` is unchanged: `clm run` builds in its own worktree.

Alternatives considered:
- Checking out detached at the claim commit when another worktree holds the branch. It never fails, but `clm land impl/<node>` and the session's pushes work on the branch, so the build would end with commits on no branch.
- Moving the other worktree's branch (`--ignore-other-worktrees`, or `update-ref`) when its tip is in `main`. It frees the name, but leaves that worktree's files out of step with its branch, behind the back of whatever session owns it.
- Keeping the claim as it is and spelling out `git switch -C impl/<node> origin/impl/<node>` in `propose` and `clm draft`'s prompt. Prose can't make the failure loud (`deterministic-enforcement`), and two texts would have to say it.
- Switching on every node claim. `clm run` claims through the same library and builds elsewhere, and a human running `clm claim <node>` may want a worktree of their own; only the fix flow always builds where it claimed.
