---
status: applied
resolves: [a-stopped-build-s-worktree-is-removed-before-the-build-can-be]
---

## Intent

Accept `backlog/a-stopped-build-s-worktree-is-removed-before-the-build-can-be` and delete it: CLM already keeps a stopped, unlanded build's worktree, as written and as built, until the orchestrating session chooses a fresh build. This change only deletes the item; it designs nothing.

## Why the behaviour is accepted

The item asks that `clm wrap-up` and `clm run` keep a stopped build's worktree "until the build lands, is retried fresh, or the session ends". That is what the rules already say, and what the code does:

- `clm wrap-up` (`orchestrator.checkpoint/checkpoint`) removes a build's worktree only when its branch is *landed* (its tip an ancestor of `origin/main` off the first-parent chain), and SHALL NOT touch files outside the worktrees it removes. A pushed, clean, unlanded `impl/*` branch is reported as `open <branch>`, and its worktree stays. `crates/clm-checkpoint/src/lib.rs` judges a worktree by `landed_local` alone, never by whether its work is pushed; no rule of checkpoint treats pushed work as removable.
- `clm run --finish <component>` (`orchestrator.run`'s Contract and Design) removes a worktree only when the branch has landed; otherwise "the claim, the local branch and the worktree stay, so a retry resumes them".
- `clm run <component>` (`orchestrator.run/run`) touches only a worktree that has `impl/<component>` checked out, never another component's. It removes that one when it resumes the branch for a new build, which is the "retried fresh" end the item itself names: the orchestrating session runs `clm run <component>` only once it has chosen a new subagent over the stopped one, so the old subagent will not be resumed and its worktree holds nothing the new build lacks (the branch, pushed or held, stays). A dirty worktree refuses the run instead.

So neither command removes the worktree of a stopped build while the session may still resume it. The removals the report saw came either from the retry of that same component, by design as above, or from outside CLM: none of `clm wrap-up`'s or `clm run`'s code paths removes an unlanded worktree of another component. Claude Code itself cleans up an `isolation: worktree` subagent's worktree that it judges unchanged, which fits the one survivor being the build with an unpushed commit; CLM doesn't control that cleanup, and a rule in CLM can't keep a worktree Claude Code removes.

Whether the orchestrating session should resume a stopped build rather than retry it is `backlog/a-stopped-build-is-always-restarted-from-scratch-even-when`, being designed in `20261107-a-stopped-build-is` (`plugin`). That judgement is made before `clm run <component>`, so the worktree is still there when it is made, as far as CLM is concerned; a resume that finds the worktree gone already counts as a stop and goes to the retry (`resume-limited-builds`).

`redesign-before-workaround` and `yagni`: the design is right as it stands, so adding a warning to `clm run`'s resume, or a new keep rule to `clm wrap-up`, would add a rule for a removal CLM doesn't make.

## Edge cases and workarounds

None.

## Open questions

None.
