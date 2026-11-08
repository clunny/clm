---
description: Review CLM's settings or change one. Use whenever the person wants to see or change how CLM behaves in the project, in plain words (how many builds run at once, whose approval counts, what runs before a land) or by a key's name.
---

# Configure CLM

Show the person CLM's settings and change the ones they choose, in the file that fits. Never push `main` by any means but `clm land`. Run `clm approve` only after the human says yes in this conversation, and only through the guard's confirmation.

1. Run `clm config list` for every key's value and where it came from, and `clm guide settings` for what each key does, what each choice trades off and where it may be set. When `clm config list` fails, show the error, which names the file and the key, and offer to fix that first.
2. With no argument, review every setting in the settings guide's order: for each, show its value and where it came from and the trade-offs briefly, and ask whether to change it. Leave `upgraded`, `declined_migrations` and `skip_migrations` to `/clm:upgrade`.
3. With an argument (`/clm:configure set concurrent builds`), find the key it means from the settings guide, asking when more than one fits, and go straight to that key.
4. For a change, say which files may set the key and recommend one: `clm.toml` for the project, the clone's `clm.local.toml` for this clone, the person's own `config.toml` for every project of theirs. Never set a key in a file the guide forbids. Say when an environment variable sets the key, since a file's value doesn't count while it does.
5. Edit the file in place, keeping its comments and other keys, then run `clm config list` again. On a failure, fix the edit or undo it.
6. Commit nothing for `clm.local.toml` or the person's own `config.toml`, which take effect at once.
7. Gather every `clm.toml` edit of the session on one branch `config/<YYYYMMDD>-<slug>` cut from `origin/main` in a worktree of its own, `.claude/worktrees/<YYYYMMDD>-<slug>`, never switching the checkout you started in (`session-worktrees`): run `git worktree add .claude/worktrees/<YYYYMMDD>-<slug> -b config/<YYYYMMDD>-<slug> origin/main` and change into it. In a session whose checkout already lies under `.claude/worktrees/`, cut the branch in that checkout with `git switch -c config/<YYYYMMDD>-<slug> origin/main` instead. Commit the edit there.
8. Land it by the first line of `clm gate --base origin/main`, as `propose` lands a branch (`small-change-no-claim`). On `code` or `spec`, push with `clm branch push`, open the pull request with `clm pr open` and run `clm land`. On `spec review`, which an edit to `approval` makes, show the human the edit and ask for their approval; on a yes run `clm approve` through the guard's confirmation, then `clm land`.
9. Once it has landed, delete the pushed branch with `clm branch delete`, since `clm land` deletes only claim branches. Remove the worktree with `git worktree remove` from the directory you started in; in a session already under `.claude/worktrees/`, switch off the branch with `git switch --detach origin/main` and remove nothing. Then run `clm wrap-up` and pass each `left <path>: ...` line it prints to the human as it stands, a worktree it couldn't remove, which doesn't make the wrap-up incomplete, and leave the directory, and any command the line names, to them.
