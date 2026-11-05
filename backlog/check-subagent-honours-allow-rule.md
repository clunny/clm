---
title: "Check that a worktree-isolated background subagent in auto mode honours a project allow rule"
kind: human-task
components: [init, plugin]
origin: 20261105-unattended-steps-denied-in
blocks: [init/init-seeds-permissions, init/init-check-permissions, plugin/unattended-permissions-check, clm/session-commands]
---

`20261105-unattended-steps-denied-in` rests on one fact Claude Code's docs imply but don't state: a narrow allow rule in the project's settings resolves before the auto-mode classifier for a background subagent launched with `isolation: "worktree"`, as builds and drafts are. An unattended session can't run this check, since it means editing `.claude/settings.local.json` and watching the classifier decide.

1. In a clone of this repository, push a throwaway branch: `git switch -c scratch/allow-rule origin/main`, `git commit --allow-empty -m scratch`, `git push -u origin scratch/allow-rule`, then `git commit --amend --allow-empty -m scratch2`, so the next push must be forced.
2. Start Claude Code there in auto mode. Ask it to launch a background `general-purpose` subagent with `isolation: "worktree"` that runs `git fetch origin`, `git switch scratch/allow-rule`, `git reset --hard <the amended sha>` and `git push --force-with-lease origin scratch/allow-rule`, and reports whether the push was denied. Expect a denial.
3. Add `"Bash(git push --force-with-lease origin scratch/allow-rule)"` to `permissions.allow` in `.claude/settings.local.json`, start a new session in auto mode, and repeat step 2. The push should now run without a denial.
4. Remove the rule, and delete the branch with `git push origin --delete scratch/allow-rule`.

When step 3's push ran, delete this item: the builds it blocks can go ahead. When it was denied, the design's premise is wrong: replace this item with a `question` saying so, blocking the same requirements, so the change is redesigned before anything relies on the rules.
