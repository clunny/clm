---
description: Bring a project up to date with the clm it runs, taking the migrations clm upgrade lists and landing them. Use when clm says a migration is pending or the human asks to upgrade the project.
---

# Upgrade the project

Bring the project up to date with the `clm` it runs, one migration at a time (`upgrades-carry-migrations`). Never push `main` by any means but `clm land`. Run `clm approve` only after the human says yes in this conversation, and only through the guard's confirmation.

1. Run `clm upgrade`. When it prints `up to date`, say so and stop.
2. Cut the branch `upgrade/<YYYYMMDD>` from `origin/main` in a worktree of its own (`session-worktrees`): run `git worktree add .claude/worktrees/upgrade-<YYYYMMDD> -b upgrade/<YYYYMMDD> origin/main`, change into it and work only there. Never switch the checkout you started in onto another branch.
3. Run `clm upgrade --apply`. It takes the mechanical migrations and prints a `note <id>` line for each advised edit the project owns.
4. For each `note <id>`, read the note with `clm upgrade --note <id>`, show the human the edit it proposes beside the file's current text, and make the edit on their yes. On their no, run `clm upgrade --decline <id>`. An edit to `ARCHITECTURE.md` or `CAPABILITIES.md` is one the human then approves again with `clm approve`.
5. When `clm upgrade` still lists a `required` migration after `--apply`, or a migration fails, report it with `clm defect --component upgrade`, and tell the human that `skip_migrations` in this clone's `clm.local.toml` lets this clone run meanwhile. Never set it without their yes.
6. Run `clm check`.
7. Land in two branches, since mechanical migrations are code and a note's edit is spec, which `clm gate` refuses together as `mixed`. Commit the changes outside spec paths on `upgrade/<YYYYMMDD>`, when there are any, push with `clm branch push`, open the pull request with `clm pr open` and land it with `clm land`. Then cut `upgrade/<YYYYMMDD>-notes` from the new `origin/main`, commit the rest there, with `clm.toml`'s `upgraded` and `declined_migrations` lines, and land it the same way.
8. Land each branch by the first line of `clm gate --base origin/main`, as `propose` lands a branch: `code` or `spec` lands on checks with `clm land`; when it prints `spec review`, ask for the human's `clm approve` first and land on their yes. Go by what `clm land` prints; `gate/up-to-date` or a conflict means `clm rebase` until its last line is `rebase ok`.
