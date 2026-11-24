---
status: proposed
resolves: [the-push-guard-lets-a-push-through-whose-settings-come-by-git-config-count]
---

## Intent

`push-guard` reads a push's `-c` settings and, with `20261123-the-push-guard-lets`, blocks a push of the current branch that a `--config-env` setting could redirect; `push-guard-scoped-to-clm` counts either form's `core.worktree` or `core.bare` as naming another work tree. A segment's assignment of `GIT_CONFIG_COUNT` with `GIT_CONFIG_KEY_<n>`/`GIT_CONFIG_VALUE_<n>`, or of `GIT_CONFIG_PARAMETERS`, sets the same settings, and `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` name config files that can; the guard reads none of them. It now takes the safe side as it does for `--config-env`: a push of the current branch (no refspec, or only `HEAD` or `@`) from a segment that assigns a variable whose name starts with `GIT_CONFIG` is blocked, with a reason that names the variable and suggests `HEAD:<branch>`, and such an assignment makes the push count as under CLM.

## The problem

`GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=remote.origin.push GIT_CONFIG_VALUE_0=HEAD:main git push` from a branch is allowed while its `-c` form is blocked, and `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.worktree GIT_CONFIG_VALUE_0=/clm/checkout git push origin main` from a scratch repository is taken as outside CLM, so a push to the CLM checkout's `main` gets through.

## Approaches

- **Block a push of the current branch from a segment that assigns a `GIT_CONFIG*` variable, and count it as under CLM** (chosen): the rule `--config-env` already has, with no parsing; one prefix covers every variable git reads configuration through, and a session that meant another branch reruns with an explicit `HEAD:<branch>`, the remedy every push reason suggests. It blocks a few cases more than strictly needed (a key outside `push.`, `remote.` and `branch.`, or `GIT_CONFIG_NOSYSTEM`), accepted to keep one rule.
- **Read the literal assignments and pass the `push.`, `remote.` and `branch.` pairs to the queries as `-c` options**: rejected; it needs a parser for `GIT_CONFIG_PARAMETERS`' quoting, reads config files a command names for `GIT_CONFIG_GLOBAL`, can't read a value that is an expansion, and adds a second path for settings a backstop doesn't need (`yagni`).
- **Name them in the Design among what gets through**: rejected; it lets a push to `main` through by documented git variables while the `-c` and `--config-env` forms are blocked, which `guard-blocks-push-to-main` exists to stop.

## Outcome

`plugin.hooks.guard`: `push-guard` gains a case for a segment's assignment of a variable whose name starts with `GIT_CONFIG`, says such assignments aren't passed to the queries, names the variable among the reasons, and adds the scenario "configuration from variables". `push-guard-scoped-to-clm` counts such an assignment beside `GIT_DIR`, `GIT_WORK_TREE` and `GIT_COMMON_DIR` and adds the scenario "a work tree set by a variable". The Design says why the variables aren't read, and names an `export`ed `GIT_CONFIG_*` variable and a `HOME` or `XDG_CONFIG_HOME` pointing at another global config among what gets through. The build changes `crates/clm-guard/src/push.rs`, reading the segment's assignments from the shell's `Command::assigns`, which already lists every `NAME=value` a segment's prefix holds, and re-cites the two requirements' tests at their new versions.

## Constraints

Edge cases and workarounds: none. An assignment in an earlier segment (`export GIT_CONFIG_COUNT=1; git push`) isn't the push segment's and gets through, as an `export`ed `GIT_DIR` does. Variable names are matched case-sensitively, as git reads them.
