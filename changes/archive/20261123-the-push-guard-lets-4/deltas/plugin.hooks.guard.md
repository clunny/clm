---
component: plugin.hooks.guard
base: { contract: "401f37f6e91aeb01", design: "4c27a608019445d3" }
---

## MODIFIED Requirements

### Requirement: push-guard <!-- base: d829891a3069e0f6 -->
`clm hook guard` SHALL exit 2, writing a reason that names `guard-blocks-push-to-main` to stderr, for a `Bash` command with a segment that runs `git push` under CLM (`push-guard-scoped-to-clm`) and:
- has an `--all` or `--mirror` token; or
- has a refspec whose destination reaches `main`: it is `main`, `heads/main` or `refs/heads/main`, or empty (`:`, which pushes every matching branch), or holds a `*` and its text before the first `*` is the start of `main`, `heads/main` or `refs/heads/main`; or
- has no refspec, or a refspec that is `HEAD` or `@` (git's name for `HEAD`), with or without a leading `+`, whatever other refspecs it has, and, in the checkout it pushes from, the current branch is `main`, since git pushes the current branch for each such refspec; or
- has no refspec or a named refspec, and a `remote.<name>.push` value, from the segment's `-c` options or from the settings query, has a destination that reaches `main` as an explicit refspec's would; or
- has a named refspec other than `HEAD` and `@`, with or without a leading `+`, naming the branch `<b>` (the refspec without a leading `+` and then without a leading `refs/heads/` or `heads/`), and the settings query prints a `push.default` value of `upstream` or `tracking` and a `branch.<b>.merge` value that is `main`, `heads/main` or `refs/heads/main`, since git then pushes `<b>` to its upstream; or
- has no refspec, and the settings query prints a `push.default` value of `matching`, or a `remote.<name>.mirror` value other than `false`, `no`, `off` and `0` without regard to ASCII case, since git then pushes `main` beside every other branch; or
- has no refspec or a named refspec, and, before the subcommand, a `--config-env` option, alone with its value as the next token or with `=` and its value glued on, whose name, the part of the value before its last `=`, starts, without regard to ASCII case, with `push.`, `remote.`, `branch.`, `include.` or `includeif.`, since the guard can't read the environment variable that holds the setting, or a `-c` option whose name, the part before its first `=`, starts, without regard to ASCII case, with `include.` or `includeif.`, since the guard doesn't read the file it includes; or
- has no refspec or a named refspec, and an assignment in the segment's environment of `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL` or `GIT_CONFIG_SYSTEM`, since each sets configuration the guard doesn't read; or
- has no refspec, and `git rev-parse --symbolic-full-name @{push}` in that checkout prints `refs/remotes/<remote>/main`, `<remote>` holding no `/`. Only `push.default` and a configured `remote.<name>.push` move where a bare `HEAD` goes, and the second is checked above, so `@{push}` isn't asked for a push with a refspec.

The settings query is `git config -z --get-regexp '^(push\.default|remote\..*\.(push|mirror)|branch\..*\.merge)$'`, asked at most once for a push with no refspec or a named refspec; each entry it prints, its key up to a newline and its value up to a NUL, or its key alone up to a NUL for a setting with no value, which counts as an empty value, counts, not only the last one of a key, which is the one git uses. Each of these queries SHALL run in that checkout, with those of the segment's `-c` options whose name, the part before its first `=`, starts, without regard to ASCII case, with `push.`, `remote.` or `branch.` before the subcommand, its `--git-dir` and `--work-tree` options, each alone or with `=` and the value glued on, and its `GIT_DIR` and `GIT_WORK_TREE` assignments in the environment; a query that fails decides nothing. The queries of one command SHALL share the push budget (`hook-budgets`), counted from the start of its first push query: a query still running when the budget runs out is killed, a query not yet started then SHALL NOT be started, and either SHALL block the push, with a reason that names `guard-blocks-push-to-main`, says git didn't answer in time so the guard couldn't tell whether the push reaches `main`, and suggests pushing with an explicit `HEAD:<branch>`. Other `-c` options, `include.` and `includeif.` ones among them, every `--config-env` option and the segment's assignments of those four variables SHALL NOT be passed, since the command hasn't been allowed yet. The checkout is the segment's (`guard-checkout`), then moved by the segment's `-C` directories, each alone or glued on (`-Cw`), read from the tokens the walk yields, in which a `-C` value with an unquoted glob already stands expanded as bash expands it (`plugin.hooks.guard.shell/guard-checkout`): each match is a token of its own, so with several the first is the directory and the subcommand is read among the rest, and a glued value (`-Cm*`) stands as bash globs the whole word, which only names starting `-C` match (`guard-checkout`). For these cases the reason SHALL say which test decided (the branch is `main`, the branch pushes to `main`, a configured push refspec sends to `main`, the named branch's upstream is `main` under `push.default=upstream`, `push.default=matching` or a mirroring remote pushes `main` with every branch, or a setting the guard can't read, naming the first such `--config-env` or `-c` option as written, or the first of those four variables, in the order listed, that the segment assigns, could send it to `main`) and suggest pushing with an explicit `HEAD:<branch>`. When the guard keeps a `cd` it couldn't follow for the segment (`guard-checkout`), the reason for these cases SHALL also say that it couldn't follow that `cd`, quoting it as kept (such as `cd w`, `cd $W` or `cd`), and that it judged the push from the checkout it fell back to, instead of saying that is the checkout the push runs from. The reason for a query that ran out of time doesn't change.

A segment runs git when its command (`guard-command-position`) is `git` or ends in `/git`. Its subcommand is the first token after that which isn't an option or the value of `-C`, `-c`, `--config-env`, `--git-dir`, `--work-tree` or `--namespace`. Its refspecs are the positional arguments after the first (the remote), skipping the values of `-o`, `--push-option`, `--repo`, `--receive-pack` and `--exec`. A refspec's destination is the part after its last `:`, or the whole refspec when it has none, without a leading `+`. A named refspec is one holding no `:`, `*`, `$` or backtick: `HEAD`, `@` or a branch's name, whose destination git may map through `remote.<name>.push` or the branch's upstream. A refspec SHALL be judged as written: one that holds a shell expansion, such as `$B`, is no named refspec and reaches `main` only when its destination as written does.

Pushing to or deleting any other branch SHALL stay allowed.

#### Scenario: ways to reach main
- GIVEN the commands `git push origin HEAD:main`, `git push --force origin main`, `git push origin +main`, `git push origin impl/x/b:refs/heads/main`, `git push origin HEAD:heads/main`, `git push origin :`, `git push --force origin refs/heads/*:refs/heads/*`, `git -C ../w push origin --delete main`, `git --config-env a=B push origin main` and `git push --mirror origin`, from a repository under CLM
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: implicit push from main
- GIVEN an event whose `cwd` is a repository under CLM (it holds `specs/SPEC.md`) on branch `main`, or a directory `sub` of one, and the commands `git push` and `git push origin HEAD`
- WHEN `clm hook guard` runs on each
- THEN each exits 2, with a reason suggesting `HEAD:<branch>`

#### Scenario: a push of @
- GIVEN an event whose `cwd` is a repository under CLM on branch `main`
- WHEN `clm hook guard` runs on `git push origin @`
- THEN it exits 2 naming `guard-blocks-push-to-main`, with a reason suggesting `HEAD:<branch>`

#### Scenario: HEAD beside other refspecs
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main` and `impl/x/b`, on branch `main`, with a branch `impl/x/b` whose upstream is `origin/impl/x/b`
- WHEN `clm hook guard` runs on `git push origin HEAD impl/x/b`, on `git push origin impl/x/b @`, on `git push origin +HEAD` and on `git push origin impl/x/b HEAD:impl/x/c`
- THEN the first three each exit 2 naming `guard-blocks-push-to-main`, with a reason that says the branch is `main` and suggests `HEAD:<branch>`, and the last exits 0

#### Scenario: a -C glob
- GIVEN an event whose `cwd` is a repository under CLM on `impl/x/b`, holding `m`, a worktree on `main`, and the plain directories `x1` and `x2`
- WHEN `clm hook guard` runs on `git -C m* push`, on `git -C 'm*' push`, on `git -C x* push` and on `git -Cm* push`
- THEN the first exits 2 naming `guard-blocks-push-to-main`, and the other three each exit 0

#### Scenario: a branch that pushes to main
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main`, on branch `impl/x/b` whose upstream is `origin/main`
- WHEN `clm hook guard` runs on `git -c push.default=upstream push`, on `git -c remote.origin.push=HEAD:main push`, and on `git push` with `push.default` set to `upstream` in the repository's config, and again with `remote.origin.push` set to `HEAD:main` there instead
- THEN each exits 2

#### Scenario: settings in any case
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main`, on branch `impl/x/b` whose upstream is `origin/main`
- WHEN `clm hook guard` runs on `git -c PUSH.DEFAULT=upstream push` and on `git -c REMOTE.origin.PUSH=HEAD:main push`
- THEN each exits 2

#### Scenario: a branch named without its destination
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main` and `impl/x/b`, on branch `impl/x/b` whose upstream is `origin/impl/x/b`, with a branch `impl/x/c` whose upstream is `origin/main`
- WHEN `clm hook guard` runs on `git -c remote.origin.push=refs/heads/impl/x/b:refs/heads/main push origin impl/x/b`, on `git push origin impl/x/b` with `remote.origin.push` set to `refs/heads/impl/x/b:refs/heads/main` in the repository's config, on `git -c push.default=upstream push origin impl/x/c`, on `git -c push.default=upstream push origin impl/x/b` and on `git -c push.default=upstream push origin impl/x/c:impl/x/c`
- THEN the first three each exit 2 naming `guard-blocks-push-to-main`, with a reason that suggests `HEAD:<branch>`, and the last two each exit 0

#### Scenario: matching and mirroring
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main` and `impl/x/b`, on branch `impl/x/b` whose upstream is `origin/impl/x/b`
- WHEN `clm hook guard` runs on `git -c push.default=matching push`, on `git -c remote.origin.mirror=true push`, on `git push` with `push.default` set to `matching` in the repository's config, on `git -c push.default=matching push origin HEAD` and on `git -c remote.origin.mirror=false push`
- THEN the first three each exit 2 naming `guard-blocks-push-to-main`, with a reason that suggests `HEAD:<branch>`, and the last two each exit 0

#### Scenario: settings from an included file
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main` and `impl/x/b`, on branch `impl/x/b` whose upstream is `origin/impl/x/b`, and `<p>`, the absolute path of a file that sets `remote.origin.push` to `HEAD:main`
- WHEN `clm hook guard` runs on `git -c include.path=<p> push`, on `P=<p> git --config-env include.path=P push`, on `git -c includeIf.onbranch:impl/**.path=<p> push origin impl/x/b` and on `git -c include.path=<p> push origin HEAD:impl/x/b`
- THEN the first three each exit 2 naming `guard-blocks-push-to-main`, with a reason that names the option as written, says the guard can't read its setting, and suggests `HEAD:<branch>`, and the last exits 0

#### Scenario: settings from the environment
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main` and `impl/x/b`, on branch `impl/x/b` whose upstream is `origin/impl/x/b`
- WHEN `clm hook guard` runs on `B=HEAD:main git --config-env remote.origin.push=B push`, on `U=upstream git --config-env=PUSH.DEFAULT=U push`, on `M=refs/heads/main git --config-env branch.impl/x/b.merge=M push origin HEAD`, on `git --config-env remote.origin.push=B push origin impl/x/b` and on `W=. git --config-env core.worktree=W push`
- THEN the first four each exit 2 naming `guard-blocks-push-to-main`, with a reason that names the `--config-env` option, says the guard can't read its setting, and suggests `HEAD:<branch>`, and the last exits 0

#### Scenario: configuration from variables
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main` and `impl/x/b`, on branch `impl/x/b` whose upstream is `origin/impl/x/b`
- WHEN `clm hook guard` runs on `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=remote.origin.push GIT_CONFIG_VALUE_0=HEAD:main git push`, on `GIT_CONFIG_PARAMETERS="'push.default'='upstream'" git push origin HEAD`, on `env GIT_CONFIG_GLOBAL=g git push` and on `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=remote.origin.push GIT_CONFIG_VALUE_0=HEAD:main git push origin HEAD:impl/x/b`
- THEN the first three each exit 2 naming `guard-blocks-push-to-main`, with a reason that names the variable (`GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS` and `GIT_CONFIG_GLOBAL` in turn) and suggests `HEAD:<branch>`, and the last exits 0

#### Scenario: another checkout's git directory
- GIVEN an event whose `cwd` is on `impl/x/b`, and `m` a repository on `main`
- WHEN `clm hook guard` runs on `GIT_DIR=m/.git git push` and on `git --git-dir=m/.git push`
- THEN each exits 2

#### Scenario: pushing other branches
- GIVEN the commands `git push -u origin impl/x/b`, `git push --force-with-lease origin impl/x/b`, `git push origin --delete impl/x/b` and `git push origin main:impl/x/b`, `git push` with a `cwd` on branch `impl/x/b` whose upstream is `origin/impl/x/b`, `git push` with a `cwd` on branch `impl/x/c` with no upstream, `git -c push.default=upstream push -u origin HEAD` with a `cwd` on branch `impl/x/b` whose upstream is `origin/main`, and `git -C ../w push` with a `cwd` on `main` and `../w` on `impl/x/b`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: a refspec in a variable
- GIVEN an event whose `cwd` is a repository under CLM on branch `main`
- WHEN `clm hook guard` runs on `git push origin $B`, `git push origin HEAD:$B`, `for b in x; do git push origin $b; done` and `git push origin $B:main`
- THEN the first three each exit 0, and the last exits 2 naming `guard-blocks-push-to-main`

#### Scenario: an implicit push it can't decide
- GIVEN an event whose `cwd` is a directory in no git repository
- WHEN `clm hook guard` runs on `git push`
- THEN it exits 0

#### Scenario: cd into a directory the command creates
- GIVEN an event whose `cwd` is a repository under CLM on `main`, and no directory `w` under it
- WHEN `clm hook guard` runs on `git worktree add -q w -b spec/x && cd w && git commit -m m && git push -u origin HEAD`
- THEN it exits 2 with a reason that quotes `cd w`, says the guard couldn't follow it, and suggests `HEAD:<branch>`, while `git worktree add -q w -b spec/x && cd w && git push -u origin spec/x` exits 0

#### Scenario: a push git doesn't answer
- GIVEN a fake `git`, first on the `PATH` the guard runs with, that writes its process ID to a file and then `exec`s a sleep of 60 seconds
- WHEN `guard_within` runs with a push budget of 1 second on `git push` and on `git push origin HEAD:impl/x/b`
- THEN the first returns `Block` within 5 seconds, with a reason that says git didn't answer in time and suggests `HEAD:<branch>`, and no process with the recorded ID is left once the guard has exited; the second returns `Allow` without running the fake `git`

#### Scenario: prose about pushing
- GIVEN the commands `gh pr create --title t --body "never run git push origin main"` and `git commit -m "explain git push origin main"`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Needs
- plugin.hooks.guard.shell/guard-checkout
