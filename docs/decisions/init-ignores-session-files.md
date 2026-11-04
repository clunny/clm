---
title: "clm init ignores .clm/ and .claude/worktrees/ in .gitignore, as it does clm.local.toml"
date: 2026-10-27
status: accepted
---

`clm init` adds `.clm/` and `.claude/worktrees/` to the CLM repository's `.gitignore`, next to `clm.local.toml` (`local-settings`), each when no line already names it, and a rerun adds whichever is missing. This repository's own `.gitignore` gets the same two lines, through `config`, which owns the file here.

**Why.** CLM's skills write into both directories inside the session's project: `propose`, `init` and `implement` write scratch files such as the page body under `.clm/`, and cut their session branches in `.claude/worktrees/<id>` (`session-worktrees`), where Claude Code also puts subagents' worktrees. `propose` tells the session that git ignores `.clm/`, and removes its worktree with a plain `git worktree remove`. Nothing made either true: `clm run` and `clm draft` add `.clm/` to `.git/info/exclude`, but only when they run, and this repository ignored both directories only through one clone's `.git/info/exclude`. Seen on 2026-10-27, a `propose` session's page body showed as untracked, `git worktree remove` refused, and the session deleted the file by hand. In an adopter's checkout `git status` shows `?? .claude/`, and `git add -A` records a worktree as an embedded repository.

**Why `.gitignore`, not `.git/info/exclude`.** The ignore has to hold in every clone, from the first session, before `clm run` or `clm draft` has run there. `.git/info/exclude` is per clone and isn't committed, so a fresh clone has none of it; `.gitignore` is committed once and reaches everyone. The two paths are CLM's on every machine, like `clm.local.toml`, so a committed line is right.

**Why `.claude/worktrees/` and not `.claude/`.** `.claude/settings.json` and a project's skills and agents under `.claude/` are committed; only the worktrees are scratch.

**Why one line per entry.** As for `clm.local.toml`, `clm init` doesn't ask git whether a broader pattern (`.claude/`, `.*`) already covers a path: a plain line per entry is what a reader looks for and what a rerun can test without running git, and a redundant line costs nothing.

Alternatives considered:
- *`clm check` reports a missing line.* It would fail every adopter's checks on upgrade until they edit the file by hand, for something `clm init` can do for them on its next run; and `clm check` would have to read `.gitignore` patterns or run git.
- *The skills put their files where git already ignores them*, such as inside `.git/`. A worktree inside the git directory is outside the session's project, so a `cd` there doesn't persist between Bash calls and the `Stop` hook's autosync misses it (`session-worktrees`); and Claude Code chooses `.claude/worktrees/` for subagents itself.
- *`.git/info/exclude`, as `clm run` does.* Per clone, so it misses every session before the first `clm run`, as above.
