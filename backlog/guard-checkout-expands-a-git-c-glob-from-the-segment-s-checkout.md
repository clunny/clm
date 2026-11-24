---
title: "guard-checkout expands a git -C glob from the segment's checkout, after env -C, where bash expands it from the shell's directory"
kind: bug
components: [plugin.hooks.guard.shell]
---

`plugin.hooks.guard.shell/guard-checkout` says a `git -C` value's glob is expanded "from the segment's checkout", which the same requirement defines as moved by the segment's own `env -C` or `sudo -D`. Bash expands every glob of a command line in the shell's own directory, before `env` or `sudo` changes directory. So for `env -C sub git -C m* push`, bash globs `m*` in the shell's directory, while the guard, as specified, globs it in `sub` and can judge another repository than the one git uses. The build of 2026-11-23 follows the requirement as written. A spec change should say "from the checkout so far" (before the segment's own `env -C`/`sudo -D`), as it already does for `env -C` and `sudo -D` values. Found by /code-review on impl/plugin.hooks.guard.shell.
