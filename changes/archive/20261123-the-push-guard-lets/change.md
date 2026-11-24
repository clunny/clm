---
status: done
resolves: [the-push-guard-lets-a-push-through-whose-settings-come-by-git-s]
---

## Intent

`push-guard` passes a push's `-c` settings under `push.`, `remote.` and `branch.` to its queries, and `push-guard-scoped-to-clm` counts a `-c core.worktree` or `core.bare` as naming another work tree, but neither reads git's `--config-env <name>=<envvar>`, which sets the same settings from an environment variable. The guard can't read that variable, so it now takes the safe side: a push of the current branch (no refspec, or only `HEAD` or `@`) with a `--config-env` whose name starts with `push.`, `remote.` or `branch.` is blocked, with a reason that names the option and suggests `HEAD:<branch>`, and a `--config-env` of `core.worktree` or `core.bare` makes the push count as under CLM, as the `-c` form does.

## The problem

`B=HEAD:main git --config-env remote.origin.push=B push` from a branch, and `U=upstream git --config-env push.default=U push` on a branch tracking `main`, are allowed while their `-c` forms are blocked; and `W=/clm/checkout git --config-env core.worktree=W push origin main` from a scratch repository is taken as outside CLM, so a push to the CLM checkout's `main` gets through. The variable may be set by the segment, by an earlier `export` or by the session, so the guard can't know its value.

## Approaches

- **Block a push of the current branch that a `--config-env` setting could redirect** (chosen): no value to read, one rule, and a session that meant another branch reruns with an explicit `HEAD:<branch>`, the remedy every push reason already suggests. It blocks one case more than strictly needed (a `push.` or `branch.` setting with a push of `HEAD`, which only `remote.<name>.push` moves), accepted to keep one rule.
- **Pass the setting to the queries, with the value taken from the segment's own assignment**: rejected; it reads only a literal assignment in the same segment, misses an `export`ed or inherited variable (the guard's environment is not the shell's), and adds a second path for settings a backstop doesn't need.
- **Accept the gap**: rejected; it lets a push to `main` through by a documented git option, which `guard-blocks-push-to-main` exists to stop.

## Outcome

`plugin.hooks.guard`: `push-guard` gains a case for a `--config-env` under `push.`, `remote.` or `branch.` (separate or glued with `=`, the name being the part of its value before the last `=`, as git splits it, matched without regard to ASCII case), says no `--config-env` is passed to the queries, names the new test among the reasons, and adds the scenario "settings from the environment". `push-guard-scoped-to-clm` counts a `--config-env` of `core.worktree` or `core.bare` like the `-c` form and adds the scenario "a work tree set from the environment". The Design says why the setting isn't guessed at. The build changes `crates/clm-guard/src/push.rs`'s reading of the segment's options and re-cites the two requirements' tests at their new versions.

## Constraints

Edge cases and workarounds: none. The subcommand is already found past `--config-env`'s value (`push-guard` as it stands), so the new case reads options the guard already walks. A `--config-env` after the subcommand is a `git push` option git rejects, so only those before it count.

Settled by the builder check: the new case needs no git query, so it decides with no checkout too, before any query and spending none of the budget; the reason quotes the first such option as written (`--config-env=PUSH.DEFAULT=U`); a `--config-env` value with no `=`, which git refuses, matches no name.
