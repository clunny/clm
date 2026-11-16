---
title: "Git commands other than a listing can fail while another run adds a worktree"
kind: idea
components: [vcs]
---

`20261115-git-commands-fail-while` makes `clm-git`'s `worktrees` list again after a failure (`vcs/worktrees-tries-again`), as `fetch` already fetches again, so both survive git's `fatal: failed to read .git/worktrees/<name>/commondir` while another run of the clone adds or removes a worktree. Other git commands that walk every worktree, such as `git switch`, `git branch -d` and `git worktree add`, still die on it, and callers that need only the main worktree (`clm-land`'s `runner::places_of`, `clm-loop`'s `escalate`, `clm-orchestrator`'s `repo`) still list them all rather than reading it from `common_dir`, as `clm-verify`'s `tree::places_of` now does. No such failure has been seen outside a listing and a fetch. Expected, when one is: the command that met it holds against the race, without running again a command that may have changed something before it died.
