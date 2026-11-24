---
status: applied
resolves: [guard-checkout-expands-a-git-c-glob-from-the-segment-s-checkout]
---

## Intent

`plugin.hooks.guard.shell/guard-checkout` expands a glob in a `git -C` value "from the segment's checkout", which the same requirement moves by the segment's own `env -C` or `sudo -D`. Bash expands every word of a command line in the shell's directory, before `env` or `sudo` changes directory, so for `env -C s git -C m* push` bash globs `m*` where the shell stands, while the guard globs it in `s` and can judge another repository than the one git pushes from. The requirement now expands a `git -C` glob from the checkout so far, before the segment's own `env -C` or `sudo -D`, as it already expands an `env -C` or `sudo -D` value and as `guard-quoted-substitutions` judges a substitution; the Design says the same where it describes the `-C` expansion, and a scenario proves it.

## The problem

With `cwd` on `impl/x/b`, a plain directory `m` there, and `s` holding `m` (a worktree on `main`) and `m2`, bash runs `env -C s git -C m* push` as `git -C m push` from `s`, a push from `s/m`, on `main`. The guard as specified globs `m*` in `s`, yields `git -C m m2 push`, finds `m2` as git's subcommand and allows the command. The guard is less guarded than bash's reading, the direction `guard-checkout` never accepts.

## Approaches

- **Glob from the checkout so far** (chosen): the walk already holds that checkout, since it expands an `env -C` or `sudo -D` value from it; the `git -C` expansion takes the same directory. No new mechanism.
- **Accept the difference**: rejected, since it lets a push to `main` through, which `guard-blocks-push-to-main` exists to stop.

## Outcome

`guard-checkout` modified: the `git -C` glob clause says "from the checkout so far, before the segment's own `env -C` or `sudo -D`", and the scenario "a git -C glob after env -C" is added. The Design's sentence on the `-C` expansion names the same directory. The build changes `crates/clm-guard/src/shell/walk.rs`'s `git -C` expansion to glob from the checkout before the segment's own `env -C`/`sudo -D`, and re-cites the requirement's tests at its new version.

## Constraints

Edge cases and workarounds: none. A `git -C` after `cd` segments still globs from the checkout those `cd`s leave, which is where the shell stands.

## What the requirements no longer state

- `guard-checkout`: the sentence "In a segment whose command is `git` … SHALL be expanded in the same way, from the segment's checkout, …" no longer expands the glob from the checkout the segment's own `env -C` or `sudo -D` moves it to; it expands it from the checkout before them.
