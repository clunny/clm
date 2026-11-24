---
component: plugin.hooks.guard
base: { contract: "401f37f6e91aeb01", design: "fe68d708e0f41b6d" }
---

## MODIFIED Design
Crate `clm-guard`, at `crates/clm-guard/` (`guard-split`). Its module `shell` is `plugin.hooks.guard.shell`: the guard walks the command once and judges each piece the walk yields, with that piece's checkout. It loads the `defects` setting with `clm-config`'s `load`, at the CLM root `clm-hooks`' `clm_root` finds from the event's `cwd`: the root search the hooks and `clm defect` use, so they agree on the repository. `clm hook guard`'s call loads it under the environment lookup it was given; `guard`, `guard_within` and `guard_hook` under the process's. The `git` and `gh` the guard asks inherit the process's environment either way. It tells whether a push is under CLM with `clm_root`, the same way. The call's parser carries the sentence the root's parser gives `clm hook guard` as its description, so the help reads the same whichever prints it. The scenarios that need a fake `gh` or `git` on the `PATH`, and the defect ask, are tested in `crates/clm-guard/tests/guard.rs`: each calls `clm-guard`'s call of `clm hook guard` and sets `PATH` on its own process for the length of the call (`commands-live-in-their-crates`). The scenarios of a query or lookup that runs out of time call `guard_within` with budgets of about a second, in `crates/clm-guard/tests/budgets.rs`, a test file of its own whose one test function puts the hanging fakes first on its process's `PATH`: `nextest` runs each test in a process of its own, and with one function a plain `cargo test` shares that environment with no other test. A test file cites this component's requirements or the shell's, never both, so each counts toward one component's size.

A `cargo run` is how this repository's `CLAUDE.md` tells sessions to run CLM, so it is the documented way, not obfuscation, but the guard doesn't work out which package it runs or which tokens reach the binary (`guard-asks-any-cargo-run`, `approve-guard`). Asking too often is the safe side, so another binary's `approve` and a value that happens to be the word, as in `--title approve`, are asked about too. A segment's `git` call is found the same way as a plain `clm` call, so prose that quotes a command, such as a pull request body, isn't mistaken for one. A `gh` call is found anywhere in the segment, so a release or a merge into `main` behind any prefix is caught; prose that names `gh` after another command, as in `echo gh release create`, is judged too, and costs only a rerun of that command.

**Pushes** (`push-guard`, `host-routes-guard`). The command hasn't been allowed yet, so the guard passes git nothing from it beyond the options the requirement lists. `@{push}` follows `push.default=upstream` on a branch cut from `origin/main`. The push and lookup budgets (`hook-budgets`) together stay under Claude Code's 60-second hook timeout, which would let the command through. A `remote.<name>.push` refspec counts whichever remote it names, since asking which remote a push uses would need more of git's rules than a backstop is worth. A `--config-env` setting takes its value from an environment variable the guard can't read, since the segment, an earlier `export` or the session may set it, so one whose name could move where a push of the current branch or of a named branch goes blocks such a push instead of being guessed at; it costs only a rerun with an explicit `HEAD:<branch>`, and a `--config-env` of `core.worktree` or `core.bare` counts as naming another work tree, as the `-c` form does. A segment's assignment of `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL` or `GIT_CONFIG_SYSTEM` is taken the same way: `GIT_CONFIG_COUNT` with its `GIT_CONFIG_KEY_<n>` and `GIT_CONFIG_VALUE_<n>`, and `GIT_CONFIG_PARAMETERS`, set settings as `-c` does, and `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` name other config files; reading them would mean parsing `GIT_CONFIG_PARAMETERS`' quoting, filtering every key and reading files the command names, for values that may be expansions, so a push of the current branch or of a named branch with one is blocked instead, and the push counts as naming another work tree. A key or value variable with no `GIT_CONFIG_COUNT` sets nothing, and `GIT_CONFIG_NOSYSTEM` can only drop settings, so neither counts. The guard asks for each of the four by name with the shell's `assigned`, and decides this case, as the `--config-env` one, before any git query, the `--config-env` case first. A `-c` or `--config-env` `include.path` or `includeIf.<condition>.path` names a config file that can set any setting; reading it would mean reading a file the command names, and the `--config-env` form's path is a variable's, so a push of the current branch or of a named branch with one is blocked instead, as for `--config-env`, and the push counts as naming another work tree. The repository's own `include.path` needs nothing more, since the settings query, like any `git config` read, follows it. A refspec with no `:` that names a local branch is mapped by git through `remote.<name>.push`, or under `push.default=upstream` to the branch's `branch.<b>.merge`, so a named branch is judged as a push of the current branch is, with the same settings query; and `push.default=matching` or a `remote.<name>.mirror` pushes every branch the remote holds, `main` among them, with no refspec. One `git config --get-regexp` asks for every setting these cases read, so a push with a named refspec pays one query more than before and no other. Each case counts any line of its setting, not only the last one git uses, and blocks `matching` or a mirror whether or not the local `main` is ahead, the safe side: a push of a named branch with a `remote.<name>.push` whose source is another branch is blocked too, since matching git's refspec rules is more than a backstop is worth, and the remedy, an explicit `HEAD:<branch>`, is never mapped. A refspec held in a variable (`git push origin $B`, or a `for` loop's variable) is judged as written, and no setting maps it, since a session's own scripts write such pushes for their branches and treating one as possibly `main` would block them. The branch is always `main`, the only base CLM's flow uses (`no-stacked-prs`). GraphQL's ref IDs can't be read, so `updateRef` and `updateRefs` are blocked for any ref. CLM never merges a pull request (`land-locally`), so none of the host routes to `main` is part of its flow. `clm land` pushes `main` from inside the binary, not through a guarded `git push`, so it passes; what keeps a session from landing an unapproved branch that needs review is `clm land`'s gate, which is deterministic (`land-locally`). The guard is a backstop for accidents, not a shell parser: `eval`, a command held in a variable, an `export`ed `GIT_DIR` or `GIT_CONFIG_COUNT`, a `HOME` or `XDG_CONFIG_HOME` that points git at another global config, a script file, a refspec `xargs` reads from stdin, a refspec held in a variable that a configured push refspec or the upstream maps to `main`, a `cd` into a directory on a hung file system (resolving it isn't a git query, so it has no budget), a command's output piped into a shell (`echo '...' | bash`), a GraphQL query read from a file, and the contents API writing a file to `main` get through.

**Repositories not under CLM** (`push-guard-scoped-to-clm`). The guard protects the `main` that `clm land` owns, so a push from a scratch repository a session makes to try something, or from another project where the plugin is installed for the user, is not judged. The test is made only for a push a rule would block, so other pushes pay nothing for it. `clm_root` alone can't tell a directory in no git repository from one in a repository without `specs/SPEC.md`, so the guard also looks for the `.git` above; the `git ls-files` query then finds a CLM repository in a subdirectory of a larger one, which shares its `main`. Whatever the guard can't place counts as under CLM, so a push from a checkout under CLM is always judged. These get through, as a backstop for accidents accepts: a push from a repository not under CLM whose remote is a CLM repository's (telling would mean reading the remote); a push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (a `git clone --no-checkout`, or a worktree at a commit before `specs/` existed); one whose work tree is moved by an exported variable or a `core.worktree` set in its config rather than on the command; and the file-system walk, like a `cd`, has no budget on a hung file system. The host's routes are judged whatever the checkout, since the repository they write to is named by a path or resolved by `gh`, not by the checkout alone.

**Release workflows** (`release-guard`). The workflow argument of `gh workflow run` is read as `gh` reads its options, so neither a value that happens to be `release` nor an option before the workflow decides wrongly. A method set twice counts if either is a write, the safe side.

**Re-run lookups** (`release-run-lookup`). With no `-R`/`--repo`, the guard asks `gh repo view` which repository a re-run targets, since `gh api` takes `{owner}/{repo}` from the checkout but its host from `GH_HOST` or the default host, not from that repository.

**Approval** (`approval-policy`). The `ask` permission decision is `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": ...}}`. Claude Code shows the human the `ask` decision's reason before the command runs, in any permission mode; it doesn't promise to show the command beside it, so a reason that asks the human to judge text quotes that text. Unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve. An agent harness that can't confirm turns an ask into a refusal, which cancels the whole command as a block does, so it carries the same closing line (`block-refuses-whole-command`).

## MODIFIED Requirements

### Requirement: push-guard <!-- base: d1b155d49dc727e6 -->
`clm hook guard` SHALL exit 2, writing a reason that names `guard-blocks-push-to-main` to stderr, for a `Bash` command with a segment that runs `git push` under CLM (`push-guard-scoped-to-clm`) and:
- has an `--all` or `--mirror` token; or
- has a refspec whose destination reaches `main`: it is `main`, `heads/main` or `refs/heads/main`, or empty (`:`, which pushes every matching branch), or holds a `*` and its text before the first `*` is the start of `main`, `heads/main` or `refs/heads/main`; or
- has no refspec, or only `HEAD` or `@` (git's name for `HEAD`), and, in the checkout it pushes from, the current branch is `main`; or
- has no refspec or a named refspec, and a `remote.<name>.push` value, from the segment's `-c` options or from the settings query, has a destination that reaches `main` as an explicit refspec's would; or
- has a named refspec other than `HEAD` and `@`, naming the branch `<b>` (the refspec without a leading `+` and then without a leading `refs/heads/` or `heads/`), and the settings query prints a `push.default` value of `upstream` or `tracking` and a `branch.<b>.merge` value that is `main`, `heads/main` or `refs/heads/main`, since git then pushes `<b>` to its upstream; or
- has no refspec, and the settings query prints a `push.default` value of `matching`, or a `remote.<name>.mirror` value other than `false`, `no`, `off` and `0` without regard to ASCII case, since git then pushes `main` beside every other branch; or
- has no refspec or a named refspec, and, before the subcommand, a `--config-env` option, alone with its value as the next token or with `=` and its value glued on, whose name, the part of the value before its last `=`, starts, without regard to ASCII case, with `push.`, `remote.`, `branch.`, `include.` or `includeif.`, since the guard can't read the environment variable that holds the setting, or a `-c` option whose name, the part before its first `=`, starts, without regard to ASCII case, with `include.` or `includeif.`, since the guard doesn't read the file it includes; or
- has no refspec or a named refspec, and an assignment in the segment's environment of `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL` or `GIT_CONFIG_SYSTEM`, since each sets configuration the guard doesn't read; or
- has no refspec, and `git rev-parse --symbolic-full-name @{push}` in that checkout prints `refs/remotes/<remote>/main`, `<remote>` holding no `/`. Only `push.default` and a configured `remote.<name>.push` move where a bare `HEAD` goes, and the second is checked above, so `@{push}` isn't asked for a push of `HEAD` or `@`.

The settings query is `git config --get-regexp '^(push\.default|remote\..*\.(push|mirror)|branch\..*\.merge)$'`, asked at most once for a push with no refspec or a named refspec; every line it prints counts, not only the last one of a name, which is the one git uses. Each of these queries SHALL run in that checkout, with those of the segment's `-c` options whose name, the part before its first `=`, starts, without regard to ASCII case, with `push.`, `remote.` or `branch.` before the subcommand, its `--git-dir` and `--work-tree` options, each alone or with `=` and the value glued on, and its `GIT_DIR` and `GIT_WORK_TREE` assignments in the environment; a query that fails decides nothing. The queries of one command SHALL share the push budget (`hook-budgets`), counted from the start of its first push query: a query still running when the budget runs out is killed, a query not yet started then SHALL NOT be started, and either SHALL block the push, with a reason that names `guard-blocks-push-to-main`, says git didn't answer in time so the guard couldn't tell whether the push reaches `main`, and suggests pushing with an explicit `HEAD:<branch>`. Other `-c` options, `include.` and `includeif.` ones among them, every `--config-env` option and the segment's assignments of those four variables SHALL NOT be passed, since the command hasn't been allowed yet. The checkout is the segment's (`guard-checkout`), then moved by the segment's `-C` directories, each alone or glued on (`-Cw`), read from the tokens the walk yields, in which a `-C` value with an unquoted glob already stands expanded as bash expands it (`plugin.hooks.guard.shell/guard-checkout`): each match is a token of its own, so with several the first is the directory and the subcommand is read among the rest, and a glued value (`-Cm*`) stands as bash globs the whole word, which only names starting `-C` match (`guard-checkout`). For these cases the reason SHALL say which test decided (the branch is `main`, the branch pushes to `main`, a configured push refspec sends to `main`, the named branch's upstream is `main` under `push.default=upstream`, `push.default=matching` or a mirroring remote pushes `main` with every branch, or a setting the guard can't read, naming the first such `--config-env` or `-c` option as written, or the first of those four variables, in the order listed, that the segment assigns, could send it to `main`) and suggest pushing with an explicit `HEAD:<branch>`. When the guard keeps a `cd` it couldn't follow for the segment (`guard-checkout`), the reason for these cases SHALL also say that it couldn't follow that `cd`, quoting it as kept (such as `cd w`, `cd $W` or `cd`), and that it judged the push from the checkout it fell back to, instead of saying that is the checkout the push runs from. The reason for a query that ran out of time doesn't change.

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

### Requirement: push-guard-scoped-to-clm <!-- base: 72395435bd12f0a6 -->
`push-guard`'s cases SHALL hold only for a push under CLM: when a case would block a push that isn't under CLM, the guard SHALL allow it instead, whatever its refspecs and options (`guard-blocks-push-to-main`). A push isn't under CLM when all of these hold: the push's checkout (the segment's, moved by its `-C` directories as `push-guard` reads them), canonicalized (symlinks and `..` resolved), is an existing directory; going up from that directory, a directory holding `.git` (a directory or a file) is reached, and none up to and including the first such holds `specs/SPEC.md` as a file, the CLM root search `clm hook session-start` and `clm defect` use (`reuse-first`); the segment has no `--git-dir` or `--work-tree` option, no `-c` option whose name, the part before its first `=`, is `core.worktree` or `core.bare` or starts with `include.` or `includeif.`, without regard to ASCII case, no `--config-env` option before the subcommand, alone or with `=` and its value glued on, whose name, the part of its value before its last `=`, passes the same test, since an included file can set either setting, and no assignment of `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`, `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL` or `GIT_CONFIG_SYSTEM`, since the last four can set `core.worktree` or `core.bare`; and no `cd` the guard couldn't follow is kept for the push, as `push-guard`'s reasons would name it: a kept `cd` counts unless the push's own absolute `-C`, or an absolute `env -C` or `sudo -D` (`guard-checkout`), set the checkout; and, last, `git ls-files -z -- ':(top,glob)**/specs/SPEC.md'`, run in the checkout as `push-guard`'s queries are, within the same budget, succeeds and prints nothing, so a CLM repository in a subdirectory of the git repository keeps its `main` guarded from anywhere in that git repository. The test SHALL be made only for a push a `push-guard` case would block, the `--all`, `--mirror` and explicit refspec cases included, so a push of another branch runs no query for it; a query that fails or runs out of the budget leaves the push under CLM. Every other push, such as one from a directory in no git repository, from a checkout the guard fell back to, from a `-C` path that isn't a directory, or naming another git directory or work tree, counts as under CLM, since the guard can't tell which repository it reaches. A scratch repository made with `git init` inside a checkout under CLM is a repository of its own, so a push from it isn't under CLM.

#### Scenario: a repository not under CLM
- GIVEN an event whose `cwd` is a scratch repository on branch `main`, made with `git init`, with a remote `origin` and no `specs/SPEC.md` in it or in any directory above it up to the one holding its `.git`
- WHEN `clm hook guard` runs on `git push -u origin main`, `git push`, `git push origin HEAD:main` and `git push --all origin`
- THEN each exits 0

#### Scenario: a scratch repository inside a checkout under CLM
- GIVEN an event whose `cwd` is a repository under CLM on `impl/x/b`, and `s` a repository of its own under it, made with `git init`, on `main`
- WHEN `clm hook guard` runs on `cd s && git push -u origin main` and on `git -C s push`
- THEN each exits 0, while `git push origin main` from the `cwd` exits 2

#### Scenario: a CLM repository in a subdirectory
- GIVEN an event whose `cwd` is the top of a git repository on `main` that holds no `specs/SPEC.md` there but has `tool/specs/SPEC.md` committed
- WHEN `clm hook guard` runs on `git push origin main` and on `git push`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: a push whose repository the guard can't tell
- GIVEN an event whose `cwd` is a scratch repository on branch `main` with no `specs/SPEC.md`
- WHEN `clm hook guard` runs on `cd $W && git push origin main`, `GIT_DIR=.git git push origin main`, `git --work-tree=. push origin main` and `git -C missing push origin main`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: a work tree set in any case
- GIVEN an event whose `cwd` is a scratch repository on branch `main` with no `specs/SPEC.md`
- WHEN `clm hook guard` runs on `git -c Core.Worktree=. push origin main` and on `git -c CORE.BARE=false push origin main`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: a work tree set from the environment
- GIVEN an event whose `cwd` is a scratch repository on branch `main` with no `specs/SPEC.md`, and a remote `origin`
- WHEN `clm hook guard` runs on `W=. git --config-env core.worktree=W push origin main`, on `B=false git --config-env=Core.Bare=B push origin main` and on `U=upstream git --config-env push.default=U push`
- THEN the first two each exit 2 naming `guard-blocks-push-to-main`, and the last exits 0

#### Scenario: a work tree set by a variable
- GIVEN an event whose `cwd` is a scratch repository on branch `main` with no `specs/SPEC.md`
- WHEN `clm hook guard` runs on `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.worktree GIT_CONFIG_VALUE_0=. git push origin main` and on `GIT_CONFIG_PARAMETERS="'core.bare'='false'" git push origin main`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: a work tree set by an included file
- GIVEN an event whose `cwd` is a scratch repository on branch `main` with no `specs/SPEC.md`, and `<p>`, the absolute path of a file that sets `core.worktree`
- WHEN `clm hook guard` runs on `git -c include.path=<p> push origin main` and on `P=<p> git --config-env includeIf.onbranch:main.path=P push origin main`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Needs
- plugin.hooks.guard.shell/guard-checkout

