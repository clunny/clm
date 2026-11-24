---
title: Guard
status: active
owners: ["@clunny"]
depends_on: [plugin.hooks, plugin.hooks.guard.shell, config, vcs, agent-harness]
paths: ["crates/clm-guard/Cargo.toml", "crates/clm-guard/src/*.rs", "crates/clm-guard/tests/guard.rs", "crates/clm-guard/tests/git_layer.rs", "crates/clm-guard/tests/budgets.rs"]
successor: ""
serves: [capability/guard-sessions, capability/work-from-any-agent-harness, architecture/approval-policy, architecture/land-locally, architecture/agent-harness-adapters]
---

## Contract
The rules behind `clm hook guard` (`hook-logic-in-clm`): which Bash commands the guard asks the human about or blocks, and why. It asks the human to confirm `clm approve`, `clm capabilities decide` and `clm architecture decide` (`approval-policy`), and a `clm defect` whose report leaves the project; it blocks pushes to `main` from a repository under CLM and the host's routes to `main` (`guard-blocks-push-to-main`), release runs (`release-by-dispatch`), a `clm land` whose output is piped (`guard-refuses-piped-land`), and a `clm defect` report it can't show the human. How a command is read before the rules judge it is `plugin.hooks.guard.shell`.

Crate `clm-guard` offers:
- `guard(event)`, which judges a Claude Code `PreToolUse` event and returns `agent-harness`'s `Decision` (`Allow`, `Ask(reason)` or `Block(reason)`), loading settings under the process's environment;
- `guard_within(event, budgets)`, the same judgement with the time its git queries and host lookups may spend, given as `Budgets` (a `push` and a `lookup` duration), `Budgets::HOOK` being the hook's (`hook-budgets`);
- `guard_hook(agent_harness, event)`, which returns, as `agent-harness`'s `Rendered`, what `clm hook guard --agent-harness <name>` prints and exits with for that event (`guard-per-agent-harness`);
- `ask_json(reason)`, the `ask` permission decision's JSON that Claude Code reads;
- `clm hook guard` as a call (`commands-in-crate`, `commands-live-in-their-crates`): `guard_command(args, event, env, out, err)`, which takes the arguments the command line gives after `clm hook guard`, the text on standard input, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status; and `GuardArgs`, those arguments as the command-line parser reads them (`--agent-harness <name>`), which the root's parser embeds. The call takes no repository root: the guard places each command by its event's `cwd`.

## Design
Crate `clm-guard`, at `crates/clm-guard/` (`guard-split`). Its module `shell` is `plugin.hooks.guard.shell`: the guard walks the command once and judges each piece the walk yields, with that piece's checkout. It loads the `defects` setting with `clm-config`'s `load`, at the CLM root `clm-hooks`' `clm_root` finds from the event's `cwd`: the root search the hooks and `clm defect` use, so they agree on the repository. `clm hook guard`'s call loads it under the environment lookup it was given; `guard`, `guard_within` and `guard_hook` under the process's. The `git` and `gh` the guard asks inherit the process's environment either way. It tells whether a push is under CLM with `clm_root`, the same way. The call's parser carries the sentence the root's parser gives `clm hook guard` as its description, so the help reads the same whichever prints it. The scenarios that need a fake `gh` or `git` on the `PATH`, and the defect ask, are tested in `crates/clm-guard/tests/guard.rs`: each calls `clm-guard`'s call of `clm hook guard` and sets `PATH` on its own process for the length of the call (`commands-live-in-their-crates`). The scenarios of a query or lookup that runs out of time call `guard_within` with budgets of about a second, in `crates/clm-guard/tests/budgets.rs`, a test file of its own whose one test function puts the hanging fakes first on its process's `PATH`: `nextest` runs each test in a process of its own, and with one function a plain `cargo test` shares that environment with no other test. A test file cites this component's requirements or the shell's, never both, so each counts toward one component's size.

A `cargo run` is how this repository's `CLAUDE.md` tells sessions to run CLM, so it is the documented way, not obfuscation, but the guard doesn't work out which package it runs or which tokens reach the binary (`guard-asks-any-cargo-run`, `approve-guard`). Asking too often is the safe side, so another binary's `approve` and a value that happens to be the word, as in `--title approve`, are asked about too. A segment's `git` call is found the same way as a plain `clm` call, so prose that quotes a command, such as a pull request body, isn't mistaken for one. A `gh` call is found anywhere in the segment, so a release or a merge into `main` behind any prefix is caught; prose that names `gh` after another command, as in `echo gh release create`, is judged too, and costs only a rerun of that command.

**Pushes** (`push-guard`, `host-routes-guard`). The command hasn't been allowed yet, so the guard passes git nothing from it beyond the options the requirement lists. `@{push}` follows `push.default=upstream` on a branch cut from `origin/main`. The push and lookup budgets (`hook-budgets`) together stay under Claude Code's 60-second hook timeout, which would let the command through. A `remote.<name>.push` refspec counts whichever remote it names, since asking which remote a push uses would need more of git's rules than a backstop is worth. A `--config-env` setting takes its value from an environment variable the guard can't read, since the segment, an earlier `export` or the session may set it, so one whose name could move where a push of the current branch or of a named branch goes blocks such a push instead of being guessed at; it costs only a rerun with an explicit `HEAD:<branch>`, and a `--config-env` of `core.worktree` or `core.bare` counts as naming another work tree, as the `-c` form does. A segment's assignment of `GIT_CONFIG_COUNT`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_GLOBAL` or `GIT_CONFIG_SYSTEM` is taken the same way: `GIT_CONFIG_COUNT` with its `GIT_CONFIG_KEY_<n>` and `GIT_CONFIG_VALUE_<n>`, and `GIT_CONFIG_PARAMETERS`, set settings as `-c` does, and `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` name other config files; reading them would mean parsing `GIT_CONFIG_PARAMETERS`' quoting, filtering every key and reading files the command names, for values that may be expansions, so a push of the current branch or of a named branch with one is blocked instead, and the push counts as naming another work tree. A key or value variable with no `GIT_CONFIG_COUNT` sets nothing, and `GIT_CONFIG_NOSYSTEM` can only drop settings, so neither counts. The guard asks for each of the four by name with the shell's `assigned`, and decides this case, as the `--config-env` one, before any git query, the `--config-env` case first. A `-c` or `--config-env` `include.path` or `includeIf.<condition>.path` names a config file that can set any setting; reading it would mean reading a file the command names, and the `--config-env` form's path is a variable's, so a push of the current branch or of a named branch with one is blocked instead, as for `--config-env`, and the push counts as naming another work tree. The repository's own `include.path` needs nothing more, since the settings query, like any `git config` read, follows it. A refspec with no `:` that names a local branch is mapped by git through `remote.<name>.push`, or under `push.default=upstream` to the branch's `branch.<b>.merge`, so a named branch is judged as a push of the current branch is, with the same settings query; and `push.default=matching` or a `remote.<name>.mirror` pushes every branch the remote holds, `main` among them, with no refspec. One `git config --get-regexp` asks for every setting these cases read, so a push with a named refspec pays one query more than before and no other. Each case counts any line of its setting, not only the last one git uses, and blocks `matching` or a mirror whether or not the local `main` is ahead, the safe side: a push of a named branch with a `remote.<name>.push` whose source is another branch is blocked too, since matching git's refspec rules is more than a backstop is worth, and the remedy, an explicit `HEAD:<branch>`, is never mapped. A refspec held in a variable (`git push origin $B`, or a `for` loop's variable) is judged as written, and no setting maps it, since a session's own scripts write such pushes for their branches and treating one as possibly `main` would block them. The branch is always `main`, the only base CLM's flow uses (`no-stacked-prs`). GraphQL's ref IDs can't be read, so `updateRef` and `updateRefs` are blocked for any ref. CLM never merges a pull request (`land-locally`), so none of the host routes to `main` is part of its flow. `clm land` pushes `main` from inside the binary, not through a guarded `git push`, so it passes; what keeps a session from landing an unapproved branch that needs review is `clm land`'s gate, which is deterministic (`land-locally`). The guard is a backstop for accidents, not a shell parser: `eval`, a command held in a variable, an `export`ed `GIT_DIR` or `GIT_CONFIG_COUNT`, a `HOME` or `XDG_CONFIG_HOME` that points git at another global config, a script file, a refspec `xargs` reads from stdin, a refspec held in a variable that a configured push refspec or the upstream maps to `main`, a `cd` into a directory on a hung file system (resolving it isn't a git query, so it has no budget), a command's output piped into a shell (`echo '...' | bash`), a GraphQL query read from a file, and the contents API writing a file to `main` get through.

**Repositories not under CLM** (`push-guard-scoped-to-clm`). The guard protects the `main` that `clm land` owns, so a push from a scratch repository a session makes to try something, or from another project where the plugin is installed for the user, is not judged. The test is made only for a push a rule would block, so other pushes pay nothing for it. `clm_root` alone can't tell a directory in no git repository from one in a repository without `specs/SPEC.md`, so the guard also looks for the `.git` above; the `git ls-files` query then finds a CLM repository in a subdirectory of a larger one, which shares its `main`. Whatever the guard can't place counts as under CLM, so a push from a checkout under CLM is always judged. These get through, as a backstop for accidents accepts: a push from a repository not under CLM whose remote is a CLM repository's (telling would mean reading the remote); a push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (a `git clone --no-checkout`, or a worktree at a commit before `specs/` existed); one whose work tree is moved by an exported variable or a `core.worktree` set in its config rather than on the command; and the file-system walk, like a `cd`, has no budget on a hung file system. The host's routes are judged whatever the checkout, since the repository they write to is named by a path or resolved by `gh`, not by the checkout alone.

**Release workflows** (`release-guard`). The workflow argument of `gh workflow run` is read as `gh` reads its options, so neither a value that happens to be `release` nor an option before the workflow decides wrongly. A method set twice counts if either is a write, the safe side.

**Re-run lookups** (`release-run-lookup`). With no `-R`/`--repo`, the guard asks `gh repo view` which repository a re-run targets, since `gh api` takes `{owner}/{repo}` from the checkout but its host from `GH_HOST` or the default host, not from that repository.

**Approval** (`approval-policy`). The `ask` permission decision is `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": ...}}`. Claude Code shows the human the `ask` decision's reason before the command runs, in any permission mode; it doesn't promise to show the command beside it, so a reason that asks the human to judge text quotes that text. Unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve. An agent harness that can't confirm turns an ask into a refusal, which cancels the whole command as a block does, so it carries the same closing line (`block-refuses-whole-command`).

## Requirements

### Requirement: push-guard
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

### Requirement: release-guard
`clm hook guard` SHALL exit 2, writing a reason that names `humans-cut-releases` to stderr, for a `Bash` command with a segment that:
- runs `gh workflow run` whose workflow argument names the release workflow or the site workflow: a file name `release.yml`, `release.yaml`, `site.yml` or `site.yaml`, a path ending in one of them after a `/`, or `release` or `site`, each in any case; or
- runs `gh release` with `create`, `edit`, `upload`, `delete` or `delete-asset`; or
- runs `gh api` with a token containing `/dispatches` or `/releases`, and a write; or
- runs `gh run rerun` with a run ID, or with a job ID given by `-j` or `--job` (`-j <id>`, `-j<id>`, `--job <id>` or `--job=<id>`), when a run it re-runs is a release run or can't be looked up (`release-run-lookup`). A site run counts as a release run: re-running it would deploy the site again. A segment with both is judged by each. Re-running a release run would publish, merge or deploy again. A `gh run rerun` with neither stays allowed: `gh` then asks for a run interactively, which a session's Bash can't answer.

A segment runs `gh` when one of its tokens (as `plugin.hooks.guard.shell/guard-command-position` gives them, each `env -S` value split in place) is `gh` or ends in `/gh`, anywhere in the segment, so a prefix command or a path before it (`sudo gh`, `/usr/local/bin/gh`) still runs `gh`; each such token SHALL be read as a `gh` call whose arguments are the tokens after it, and the segment SHALL be judged by every one of these calls, so a prefix's value that is `gh` or ends in `/gh` (`sudo -u gh gh`, `env -C /src/gh gh`) doesn't hide the call.

A `gh` call's *command words* SHALL be read as `gh` finds its command: going through its arguments in order, `--help` and `-h` stand alone; any other token that is `--` and a name with no `=`, or `-` and one character, takes the next token as its value, whatever it is; any other token that starts with `-` stands alone, with its value glued on when it has one (`--repo=o/r`, `-Ro/r`); `--` ends the command words; and the command words are the tokens that are none of these. The first command word is the command (`workflow`, `release`, `run`, `pr` or `api`) and, for each but `api`, the second its subcommand, so an option before the subcommand, with its value, hides neither. `api` has no subcommand: its other arguments, `graphql` among them, are read as tokens wherever they stand.

The workflow argument is the first operand after `workflow run`, read as `gh` reads its options: going through the tokens after the command word `run` in order, `-R`, `--repo`, `-r`, `--ref`, `-f`, `--raw-field`, `-F` and `--field`, each alone, take the next token as their value, whatever it is; `--` ends the options, and the token after it is the operand; any other token that starts with `-` and is more than `-` alone is an option on its own, with its value glued on when it has one (`--ref=main`, `-rmain`); the first token that is none of these is the workflow argument, and it alone is judged.

A `gh api` call makes a *write* when it has a token starting with `-f`, `-F`, `--field`, `--raw-field` or `--input`, or a `-X` or `--method` whose value, the next token or glued on (`-XPOST`, `--method=post`), isn't `GET` in any case; any one such method makes it a write, however many it sets.

A `gh run rerun`'s run ID is its first token after the command word `rerun` that doesn't start with `-` and isn't the value of `-R`, `--repo`, `-j` or `--job`. A run or job ID is a word of digits only, or a word holding `$` or a backtick: a shell expansion whose value the guard can't know, so its lookup fails without a `gh` call. Any other word is no ID.

A run is a release run when its `path`, cut at any `@`, has the file name `release.yml`, `release.yaml`, `site.yml` or `site.yaml` in any case, or its `name` is `release` or `site` in any case. Reading releases and workflow runs, and re-running a run of any other workflow, SHALL stay allowed.

In the scenarios below, a fake `gh` for `o/r` answers `repo view --json url` with `https://github.com/o/r`, unless the scenario says otherwise.

#### Scenario: dispatching a release or the site
- GIVEN the commands `gh workflow run release.yml -f version=0.2.0`, `gh workflow run Release.YAML`, `gh workflow run RELEASE`, `gh workflow run site.yml`, `gh workflow run .github/workflows/Site.YAML` and `gh workflow run SITE`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `humans-cut-releases`

#### Scenario: dispatching another workflow
- GIVEN the command `gh workflow run ci.yml --ref release`
- WHEN `clm hook guard` runs on it
- THEN it exits 0

#### Scenario: the workflow after a flag's value
- GIVEN the commands `gh workflow run --ref main release.yml`, `gh workflow run -f version=1 Release`, `gh workflow run -f -r release.yml`, `gh workflow run --json release.yml` and `gh workflow run -- site.yml`, and the commands `gh workflow run --ref release ci.yml`, `gh workflow run -r site ci.yml` and `gh workflow run --ref=release ci.yml`
- WHEN `clm hook guard` runs on each
- THEN each of the first five exits 2 naming `humans-cut-releases`, and each of the last three exits 0

#### Scenario: creating a release directly
- GIVEN the command `gh release create v0.2.0 --generate-notes`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: dispatching through the API
- GIVEN the command `gh api repos/o/r/actions/workflows/release.yml/dispatches -f ref=main -f inputs[version]=0.2.0`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: methods as gh reads them
- GIVEN the commands `gh api -XPOST repos/o/r/releases`, `gh api --method=post repos/o/r/releases` and `gh api -X GET -X POST repos/o/r/releases`, and the command `gh api -X get repos/o/r/releases`
- WHEN `clm hook guard` runs on each
- THEN each of the first three exits 2 naming `humans-cut-releases`, and the last exits 0

#### Scenario: reading releases
- GIVEN the commands `gh release list`, `gh release view v0.1.0`, `gh release download v0.1.0 -p 'clm-*'` and `gh run list --workflow release.yml`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: re-running a release run
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/runs/77` reports `name` `Release` and `path` `.github/workflows/release.yml`
- WHEN `clm hook guard` runs on `gh run rerun 77 --failed`
- THEN it exits 2 naming `humans-cut-releases`

#### Scenario: a release workflow by another file or name
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/runs/77` reports `name` `Publish` and `path` `.github/workflows/release.yaml`, and whose `actions/runs/78` reports `name` `release` and `path` `.github/workflows/publish.yml`, and whose `actions/runs/79` reports `name` `Publish` and `path` `.github/workflows/release.yml@refs/heads/main`
- WHEN `clm hook guard` runs on `gh run rerun 77`, `gh run rerun 78` and `gh run rerun 79`
- THEN each exits 2

#### Scenario: re-running a CI run
- GIVEN a fake `gh` for `o/r` that, like `gh` 2.45, fails on `run view --json` with the field `path`, and whose `api repos/o/r/actions/runs/42` reports `name` `CI` and `path` `.github/workflows/ci.yml`
- WHEN `clm hook guard` runs on `gh run rerun 42 --failed`
- THEN it exits 0

#### Scenario: re-running a release run's job
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/jobs/900` reports `run_id` `77`, and whose `actions/runs/77` reports `name` `Release` and `path` `.github/workflows/release.yml`
- WHEN `clm hook guard` runs on `gh run rerun --job 900`, `gh run rerun -j 900`, `gh run rerun -j900`, `gh run rerun --job=900` and `gh run rerun 42 --job 900`, where run `42` is a CI run
- THEN each exits 2

#### Scenario: re-running a CI run's job
- GIVEN a fake `gh` for `o/r` whose `api repos/o/r/actions/jobs/901` reports `run_id` `42`, and whose `actions/runs/42` reports `name` `CI` and `path` `.github/workflows/ci.yml`
- WHEN `clm hook guard` runs on `gh run rerun --job 901`
- THEN it exits 0

#### Scenario: an ID the guard can't know
- GIVEN a fake `gh` for `o/r` that records each call and reports a CI run for any lookup
- WHEN `clm hook guard` runs on `gh run rerun $RUN`, `gh run rerun --job $JOB` and ``gh run rerun `cat id` ``, and on `gh run rerun abc`
- THEN each of the first three exits 2 naming `humans-cut-releases`, the last exits 0, and none of them calls `gh`

#### Scenario: a gh call behind a prefix
- GIVEN the commands `sudo gh release create v1`, `/usr/local/bin/gh release create v1`, `GH_TOKEN=x nohup gh release create v1`, `sudo -u gh gh release create v1` and `env -C /src/gh gh release create v1`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `humans-cut-releases`

#### Scenario: options before the subcommand
- GIVEN the commands `gh workflow --ref x run release.yml`, `gh workflow -f a=b run release.yml` and `gh release --notes x create v1`, and the command `gh pr --body x merge 3`
- WHEN `clm hook guard` runs on each
- THEN each of the first three exits 2 naming `humans-cut-releases`, and the last exits 2 naming `guard-blocks-push-to-main`

#### Serves
- capability/guard-sessions

### Requirement: defect-approval
`clm hook guard` SHALL judge a `Bash` command with a segment that runs `clm defect`, or a `cargo run` that `approve-guard` treats as running it, whose arguments (for a `cargo run`, its tokens after the subcommand) hold neither `--help` nor `-h`, when the `defects` setting for the event's `cwd` is `ask` or can't be loaded and no other guard rule blocks the command (`defect-ask-shows-report`). A segment's report *can be shown* when its standard input (`guard-standard-input`) is a heredoc the shell sends as written. When every such segment's report can be shown, the guard SHALL print `hookSpecificOutput` with `permissionDecision: "ask"` and exit 0, with a reason that quotes, for each such segment in order, its own text as written, without its body, and its report, the body whole, says the report leaves the project as a public issue, and says that, when no one can approve, the defect goes under `## CLM defects` in the pull request body instead; no reason the guard gives SHALL call a branch's pull request a page. When a segment's report can't be shown, it SHALL exit 2, writing to stderr a reason that names `defect-ask-shows-report`, says the guard can't show the report to the human, and tells the session to send the report as the body of a `<<'EOF'` heredoc on the `clm defect` line. When another guard rule also asks, it SHALL print one `ask` decision whose reason gives both rules' reasons; a refusal wins over any ask. A heredoc with no line ending it is no heredoc (`guard-heredocs`), so its report can't be shown. No guard rule SHALL match the heredoc body of such a segment, but for the substitutions `plugin.hooks.guard.shell/guard-heredocs` judges in an unquoted one. With `backlog` or `off` it SHALL treat the segment like any other. When the segment itself assigns `CLM_DEFECTS`, before its command or after `env` (`guard-command-position`), the last value it assigns SHALL stand for the setting, since `clm defect` reads its own environment: exactly `backlog` or `off` as the setting's, and any other value as `ask`.

#### Needs
- plugin.hooks.guard.shell/guard-standard-input

#### Scenario: asking
- GIVEN an event whose `cwd` has no settings, and a command whose first line is `clm defect --title "x" --component gate <<'EOF'`, followed by a line `The gate misfired.` and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason naming `## CLM defects` and `pull request body`, holding `clm defect --title "x" --component gate` and `The gate misfired.`, and not holding the word `page`, and exits 0

#### Scenario: asking through cargo run
- GIVEN an event whose `cwd` has no settings, and a command whose first line is `cargo run -q -p clm -- defect --title "x" <<'EOF'`, followed by a line `The gate misfired.` and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason naming `## CLM defects` and holding `The gate misfired.`, and exits 0

#### Scenario: an unquoted heredoc with nothing to expand
- GIVEN an event whose `cwd` has no settings, and the command `clm defect --title x <<EOF` followed by a line `plain words` and `EOF`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason holding `plain words`, and exits 0

#### Scenario: a report it can't show
- GIVEN an event whose `cwd` has no settings, and each of the commands `clm defect --title x < report.md`, `cat report.md | clm defect --title x`, `clm defect --title x <<< "$R"`, `clm defect --title x`, and `clm defect --title x <<EOF` followed by a line `cost $HOME` and `EOF`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `defect-ask-shows-report` and `<<'EOF'`

#### Scenario: help
- GIVEN an event whose `cwd` has no settings, and the command `clm defect --help`
- WHEN `clm hook guard` runs on it
- THEN it prints nothing and exits 0

#### Scenario: backlog
- GIVEN an event whose `cwd` is a repository with `defects = "backlog"`, and the commands `clm defect --title "x" <<'EOF'` followed by a report, and `clm defect --title x < report.md`
- WHEN `clm hook guard` runs on each
- THEN each prints nothing and exits 0

#### Scenario: the command's own setting
- GIVEN an event whose `cwd` is a repository with `defects = "backlog"`, and an event whose `cwd` has no settings, and a `clm defect --title "x" <<'EOF'` report
- WHEN `clm hook guard` runs, from the first, on the report run as `CLM_DEFECTS=ask clm defect …` and as `env CLM_DEFECTS=always clm defect …`, and, from the second, on it run as `CLM_DEFECTS=off clm defect …`
- THEN the first two each print the `ask` decision with a reason naming `## CLM defects` and exit 0, and the last prints nothing and exits 0

#### Scenario: quoting a blocked command
- GIVEN an event whose `cwd` has no settings, and a `clm defect --title "x" <<'EOF'` command whose report has a line `clm approve spec/x` and a line `git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision and exits 0

#### Scenario: block wins
- GIVEN an event whose `cwd` has no settings, and a command with a segment running `clm defect` with a `<<'EOF'` report and a segment `git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it exits 2 naming `guard-blocks-push-to-main`

#### Scenario: two asks
- GIVEN an event whose `cwd` has no settings, and a command with a segment running `clm defect` with a `<<'EOF'` report and a segment `clm approve spec/x`
- WHEN `clm hook guard` runs on it
- THEN it prints one `ask` decision with a reason naming both `## CLM defects` and `humans-review-intent`, and exits 0

#### Serves
- capability/report-defects
- capability/guard-sessions
- architecture/approval-policy
- architecture/plain-terms

### Requirement: approve-guard
`clm hook guard` SHALL read a `PreToolUse` event from stdin and, for a `Bash` command, judge each segment of it separately. It SHALL answer with the `ask` permission decision on stdout and exit 0, with a reason that names `humans-review-intent` and the command, when no other guard rule blocks the command and a segment runs `clm approve`, `clm capabilities decide` or `clm architecture decide`, outside the heredoc bodies that `guard-heredocs` says match no rule. It SHALL NOT ask about reading a capability's or an architectural constraint's cost (`clm capabilities`, `clm capabilities <slug>`, `clm architecture`, `clm architecture <slug>`), about `clm review`, which is no command, about `clm land`, or about a `gh` call that adds or removes a label. For every other event, including input it can't parse, it SHALL exit 0 and print nothing. An event with no `cwd` SHALL be judged, by every rule, as if its `cwd` were the guard's own working directory. When the guard can't read its own working directory either (it was removed), the walk SHALL start with no checkout (`guard-checkout`), so only an absolute `cd`, `env -C` or `sudo -D` sets one, and a `git push`'s own absolute `-C` SHALL be followed the same way, its queries running from that directory; a push whose segment is left with none, and has no absolute `-C`, counts as under CLM (`push-guard-scoped-to-clm`) and runs no push query, so only its `--all`, `--mirror` and explicit refspec cases decide, and a re-run whose segment is left with none SHALL count every lookup of `release-run-lookup` as failed, with no `gh` call.

A segment runs `clm` when its command (`guard-command-position`) is `clm` or ends in `/clm`, and `clm`'s arguments are the tokens after it. A `clm` call runs `clm capabilities decide` when its first argument is `capabilities` and its first later argument that doesn't start with `-` and isn't the value of `--root` or `--format` is `decide`, and runs `clm architecture decide` the same way with `architecture`. A segment is a `cargo run` when its command is `cargo` or ends in `/cargo` and its subcommand is `run` or `r`; the subcommand is the first token after `cargo` that doesn't start with `-` or `+` and isn't the value of `--color`, `--config`, `-C` or `-Z`. The guard SHALL NOT work out which package or binary a `cargo run` runs, or which of its tokens reach the binary (`guard-asks-any-cargo-run`): it SHALL treat a `cargo run` as running `clm approve` when one of its tokens after the subcommand is exactly `approve`, as running `clm capabilities decide` when one of them is exactly `capabilities` and a later one exactly `decide`, as running `clm architecture decide` when one of them is exactly `architecture` and a later one exactly `decide`, and as running `clm defect` when one of them is exactly `defect`. A cargo alias that runs `run` under another name isn't a `cargo run`. Every guard rule that looks for a `clm` call SHALL use these definitions.

#### Scenario: an event with no cwd
- GIVEN the guard running in a repository under CLM on `main`, a fake `gh` for `o/r` that records the directory of each call and reports a CI run for any run, and events with no `cwd`
- WHEN `clm hook guard` runs on `git push` and on `gh run rerun 42`
- THEN the first exits 2 naming `guard-blocks-push-to-main`, and the second exits 0, its `repo view` and its run lookup each run in the guard's own working directory

#### Scenario: a guard with no directory
- GIVEN the guard running in a directory that has been removed, a fake `gh` that records each call, and events with no `cwd`
- WHEN `clm hook guard` runs on `git push origin main`, on `git push`, on `gh run rerun 42`, on `cd <m> && git push` and on `git -C <m> push`, where `<m>` is the absolute path of a repository under CLM on `main`
- THEN the first exits 2 naming `guard-blocks-push-to-main`, the second exits 0, the third exits 2 naming `humans-cut-releases` with no `gh` call, and the fourth and fifth each exit 2 naming `guard-blocks-push-to-main`

#### Scenario: approving
- GIVEN the command `clm approve spec/x`
- WHEN `clm hook guard` runs on it
- THEN it prints the `ask` decision with a reason naming `humans-review-intent` and `clm approve`, and exits 0

#### Scenario: approving through cargo run
- GIVEN the commands `cargo run -q -p clm -- approve spec/x`, `cargo +stable run --package=clm -- approve spec/x`, `~/.cargo/bin/cargo r --bin clm -- approve x`, `CARGO_TERM_COLOR=never cargo run -pclm --release -- approve spec/x`, `cargo --color never run -p clm -- approve spec/x`, `cargo run -p clm --features f approve spec/x`, `cargo run -p=clm -- approve spec/x`, `cargo run -p clm@0.7.0 -- approve spec/x`, `cargo run --manifest-path crates/clm/Cargo.toml -- approve spec/x`, `cargo run -- approve spec/x`, `cargo run -p clm --artifact-dir d approve spec/x` and `cargo run -p clm-web -- approve spec/x`
- WHEN `clm hook guard` runs on each
- THEN each prints the `ask` decision with a reason naming `humans-review-intent`, and exits 0

#### Scenario: deciding
- GIVEN the commands `clm capabilities decide x --value high --decision keep`, `cargo run -q -p clm -- capabilities decide x --value low --decision cut`, `clm architecture decide x --value high --decision keep` and `cargo run -q -p clm -- architecture decide x --value low --decision cut`
- WHEN `clm hook guard` runs on each
- THEN each prints the `ask` decision with a reason naming `humans-review-intent` and the command, and exits 0

#### Scenario: deciding after options
- GIVEN the commands `clm capabilities --root /w decide x --value high --decision keep` and `clm architecture --root=/w --format json decide x --value high --decision keep`, and the commands `clm capabilities --root decide` and `clm capabilities x --root decide`
- WHEN `clm hook guard` runs on each
- THEN each of the first two prints the `ask` decision with a reason naming `humans-review-intent`, and exits 0, and each of the last two exits 0 and prints nothing

#### Scenario: cargo running something else
- GIVEN the commands `cargo run -q -p clm -- land spec/x`, `cargo run -q -p clm -- review x`, `cargo run -q -p clm -- capabilities x`, `cargo test -p clm -- approve`, `cargo build -p clm approve`, `cargo run -p clm`, `clm capabilities`, `clm capabilities x`, `clm architecture` and `clm architecture x`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: review is no command
- GIVEN the commands `clm review x` and `clm review`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: landing
- GIVEN the command `clm land spec/x`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Scenario: a label is nobody's business
- GIVEN the command `gh pr edit 30 --add-label spec-approved`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Scenario: approval combined with a push to main
- GIVEN the command `clm approve spec/x && git push origin main`
- WHEN `clm hook guard` runs on it
- THEN it exits 2, since the push block wins over `ask`

#### Scenario: quoted in a body
- GIVEN a `cat > body.md <<'EOF'` command whose body has a line `clm approve spec/x`
- WHEN `clm hook guard` runs on it
- THEN it exits 0 and prints nothing

#### Needs
- plugin.hooks.guard.shell/guard-checkout

#### Serves
- capability/guard-sessions
- capability/review-capability-cost
- architecture/approval-policy
- architecture/humans-review-intent
- architecture/land-locally

### Requirement: piped-land-guard
`clm hook guard` SHALL block (exit 2) a `Bash` command in which a segment that runs `clm land`, with any arguments, `--check-only` among them, pipes its output to the next command: the separator that ends the segment, as `guard-quotes` reads it, is `|` or `|&`. A segment runs `clm land` when its `clm` call (`approve-guard`) is `clm` with `land` as its first argument, or a `cargo run` whose `land` token is `land`, and no token after that `land` is exactly `--help` or `-h`: that prints the help and lands nothing, so it has no verdict for a pipe to hide. A `cargo run`'s `land` token is the first token after the first `--` that follows the subcommand, since cargo hands the binary the tokens after that `--` and `clm` reads its first argument as its subcommand; in a `cargo run` with no such `--`, it is the first token after the subcommand that is exactly `land`, since the guard doesn't work out which of those tokens are cargo's (`guard-asks-any-cargo-run`). The rule SHALL judge every piece the guard judges, a `-c` script's and a heredoc body a shell runs included, and SHALL NOT judge a heredoc body that `guard-heredocs` says matches no rule. The block's reason SHALL name `guard-refuses-piped-land` and the plain command to run instead, the segment's tokens joined by spaces (its redirections left out), and SHALL say why: a pipe reports the last command's result rather than the land's and cuts its output, while the land's last line is its verdict and Claude Code keeps a long output whole. It SHALL NOT block a `clm land` that is not piped, whatever redirections it has, nor one whose only pipe is the one it reads its standard input from (`a | clm land | b` pipes its output, and is blocked), nor a pipe from any other `clm` command, a `land` that is an argument of another subcommand included.

#### Scenario: check piped into tail
- GIVEN the commands `clm land --check-only | tail -5`, `clm land --check-only 2>&1 | tail -20`, `clm land impl/x |& tail`, `cargo run -q -p clm -- land --check-only | tail -5` and `cargo run -p clm land spec/x | tail`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `guard-refuses-piped-land`, `last line` and the plain command (`clm land --check-only` for the first two, `clm land impl/x` for the third)

#### Scenario: piped inside a script
- GIVEN the commands `bash -c 'clm land --check-only | tail'` and `cd /tmp && clm land spec/x | tee land.log`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `guard-refuses-piped-land`

#### Scenario: plain land
- GIVEN the commands `clm land --check-only`, `clm land spec/x`, `clm land --check-only 2>&1` and `clm land --check-only && git status`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: other pipes
- GIVEN the commands `clm status | head`, `echo y | clm land spec/x`, `cargo run -q -p clm -- status | head` and a `cat > body.md <<'EOF'` command whose body has a line `clm land --check-only | tail`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: land as another subcommand's argument
- GIVEN the commands `cargo run -q -p clm -- hashes land | grep x`, `cargo run -q -p clm -- context land | head`, `cargo run -p clm -- pending land |& head` and `clm hashes land | grep x`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: help piped
- GIVEN the commands `clm land --help | head`, `clm land -h | head`, `clm land impl/x --help |& head` and `cargo run -q -p clm -- land --help | head`
- WHEN `clm hook guard` runs on each
- THEN each exits 0 and prints nothing

#### Scenario: help elsewhere than the land's arguments
- GIVEN the commands `clm land spec/--help | tail`, `echo --help | clm land spec/x | tail` and `cargo run --help -- land | tail`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 with a reason naming `guard-refuses-piped-land`

#### Serves
- capability/land-safely

### Requirement: hook-budgets
`clm-guard` SHALL offer `Budgets`, a push budget and a lookup budget, with `Budgets::HOOK` a push budget of 20 seconds and a lookup budget of 30 seconds, and `guard_within(event, budgets)`, which SHALL judge `event` as `guard` does, its `push-guard` queries sharing `budgets`' push budget and its `release-guard` lookups its lookup budget. `guard(event)` SHALL return what `guard_within(event, Budgets::HOOK)` returns, and `guard_hook` and `clm hook guard`'s call SHALL judge with `Budgets::HOOK` too.

#### Scenario: the hook's budgets
- GIVEN `clm-guard`
- WHEN `Budgets::HOOK` is read
- THEN its push budget is 20 seconds and its lookup budget 30 seconds

#### Scenario: the hook judges with the hook's budgets
- GIVEN an event whose `cwd` is a repository under CLM on `impl/x/b` whose git answers at once, a fake `gh` for `o/r` on the `PATH` that reports a CI run, and the commands `git push`, `gh run rerun 42` and `git push origin HEAD:main`
- WHEN `guard`, `guard_within` with `Budgets::HOOK`, `guard_hook` for Claude Code and `clm hook guard`'s call each judge the three, and `guard_within` judges the first two again with a push budget of zero and with a lookup budget of zero
- THEN with the hook's budgets each gives `Allow`, `Allow` and `Block`, `guard_hook` and the call printing and exiting as Claude Code's rendering of that; with no push budget `git push` is blocked with a reason saying git didn't answer in time and `gh run rerun 42` is allowed; and with no lookup budget `git push` is allowed and `gh run rerun 42` is blocked naming `humans-cut-releases`

#### Serves
- architecture/land-locally

### Requirement: push-guard-scoped-to-clm
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

### Requirement: host-routes-guard
`clm hook guard` SHALL exit 2, writing a reason that names `guard-blocks-push-to-main` to stderr, for a segment that runs `gh`, found as `release-guard` finds it, whatever its checkout, under CLM or not, with:
- `api`, a token whose path, before any `?` and without a trailing `/`, ends in `git/refs/heads/main`, and a write (`release-guard`); or
- `api`, a path ending in `/merges`, a write, and a field whose value, after the option or its `=`, is `base=main` or `base=refs/heads/main`, or an `--input` token; or
- `api`, a path ending in `/pulls/<number>/merge`, and a write; or
- `pr merge`; or
- `api`, a token `graphql`, and a token containing `mergePullRequest`, `mergeBranch`, `enablePullRequestAutoMerge` or `updateRef`.

Reading through the host, and merging into any other branch, SHALL stay allowed.

#### Scenario: merging into main through the host
- GIVEN the commands `gh api -X PATCH repos/o/r/git/refs/heads/main -f sha=abc`, `gh api -X POST repos/o/r/merges -f base=main -f head=impl/x/b`, `gh api repos/o/r/merges --field=base=refs/heads/main -f head=x`, `gh api -X PUT repos/o/r/pulls/3/merge`, `gh pr merge 3 --squash`, `gh api graphql -f query='mutation { mergePullRequest(input: {pullRequestId: "P"}) { clientMutationId } }'` and `gh api graphql -f query='mutation { updateRefs(input: {repositoryId: "R", refUpdates: []}) { clientMutationId } }'`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: reading or merging elsewhere through the host
- GIVEN the commands `gh api repos/o/r/git/refs/heads/main`, `gh api -X PATCH repos/o/r/git/refs/heads/mainline -f sha=abc`, `gh api -X POST repos/o/r/merges -f base=impl/x/b -f head=main`, `gh api repos/o/r/pulls/3/merge`, `gh pr view 3`, `gh pr create --title t --body "then gh pr merge it"` and `gh api graphql -f query='query { repository(owner: "o", name: "r") { id } }'`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: the host's routes outside CLM
- GIVEN an event whose `cwd` is a scratch repository with no `specs/SPEC.md`
- WHEN `clm hook guard` runs on `gh pr merge 3 --squash` and on `gh api -X PATCH repos/o/r/git/refs/heads/main -f sha=abc`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: a gh call behind a prefix
- GIVEN the command `sudo gh pr merge 3 --squash`
- WHEN `clm hook guard` runs on it
- THEN it exits 2 naming `guard-blocks-push-to-main`

### Requirement: release-run-lookup
To judge a `gh run rerun` (`release-guard`), the guard SHALL look each run up through the REST API, which every `gh` version serves: `gh api [--hostname <host>] repos/<owner>/<repo>/actions/runs/<run-id>`, and for a job ID first `gh api [--hostname <host>] repos/<owner>/<repo>/actions/jobs/<job-id>`, whose `run_id` names the run. The repository is `[<host>/]<owner>/<repo>`, and a host in it becomes `--hostname <host>`. When the segment has `-R`/`--repo`, which wins as it does for `gh`, its value is the repository; a value that isn't of that form counts as a failed lookup. Otherwise the guard SHALL first run `gh repo view --json url`, once per segment, which resolves the repository as `gh run rerun` does, and take the repository from the `url` it reports: the part after `://` split at `/` into exactly three non-empty parts, `<host>/<owner>/<repo>`, so `--hostname <host>` is always passed. A `repo view` that fails, or a `url` that isn't of that form, makes every lookup of the segment a failed lookup. Each `gh` call SHALL run in the checkout the segment runs in, found as `push-guard` finds a push's checkout (the event's `cwd`, moved by each earlier `cd <dir>` it can follow), with the segment's own `GH_REPO=` and `GH_HOST=` assignments in its environment, so `gh` resolves the repository and host as it would for the re-run, and the lookup asks that host. A lookup that fails, returns a job whose `run_id` isn't a positive integer, or returns a run without a string `path` and `name` counts as a release run. The guard SHALL stop looking up at the first release run, and its `gh` calls SHALL share the lookup budget (`hook-budgets`), so the guard answers before the hook's own timeout, which Claude Code treats as allowing: a call still running when the budget runs out is killed, not left running after the hook exits, and counts as a failed lookup; a call not yet started when the budget is spent SHALL NOT be started, and counts as a failed lookup.

In the scenarios below, a fake `gh` for `o/r` answers `repo view --json url` with `https://github.com/o/r`, unless the scenario says otherwise.

#### Scenario: the re-run's repository and host
- GIVEN a fake `gh` that records the arguments, directory and `GH_REPO` and `GH_HOST` of each call, answers `repo view --json url` with `https://ghe.example.com/o/r` when its `GH_REPO` is `o/r` and with `https://github.com/p/q` otherwise, and reports a CI run for any run, and an event whose `cwd` has a subdirectory `other`
- WHEN `clm hook guard` runs on `cd other && gh run rerun 42`, `gh run rerun 42 -R o/r`, `gh run rerun 42 --repo ghe.example.com/o/r`, `GH_REPO=o/r gh run rerun 42 -R x/y` and `GH_HOST=ghe.example.com GH_REPO=o/r gh run rerun 42`
- THEN each exits 0, and the run lookups ask, with these arguments in any order before the path: `api --hostname github.com repos/p/q/actions/runs/42` in `other`, after a `repo view` in `other`; `api repos/o/r/actions/runs/42`; `api --hostname ghe.example.com repos/o/r/actions/runs/42`; `api repos/x/y/actions/runs/42`; and `api --hostname ghe.example.com repos/o/r/actions/runs/42`, after a `repo view` with `GH_HOST` `ghe.example.com` and `GH_REPO` `o/r`; all but the first in the event's `cwd`, and only the first and last after a `repo view`

#### Scenario: a repository that can't be read
- GIVEN a fake `gh` for `o/r` that reports a CI run for any lookup
- WHEN `clm hook guard` runs on `gh run rerun 42 -R https://github.com/o/r` and on `gh run rerun 42 -R r`
- THEN each exits 2

#### Scenario: a repository that can't be resolved
- GIVEN a fake `gh` that reports a CI run for any `api` lookup, and fails on `repo view`, or answers it with a `url` of `o/r`
- WHEN `clm hook guard` runs on `gh run rerun 42` with each
- THEN each exits 2

#### Scenario: run can't be looked up
- GIVEN a fake `gh` for `o/r` that fails on `api`, one whose `api` prints something that isn't a run or a job, and one whose job reports `run_id` `"x"`
- WHEN `clm hook guard` runs on `gh run rerun 42` and on `gh run rerun --job 900` with each
- THEN each exits 2

#### Scenario: a lookup that never answers
- GIVEN a fake `gh` for `o/r`, first on the `PATH` the guard runs with, that never answers `api`, and one that never answers `repo view`, each writing its process ID to a file and then `exec`ing a sleep of 60 seconds
- WHEN `guard_within` runs with a lookup budget of 1 second on `gh run rerun 42` and on `gh run rerun --job 900` with each
- THEN each returns `Block` within 5 seconds, and no process with a recorded ID is left once it has returned

#### Serves
- capability/guard-sessions

#### Needs
- vcs/bounded-runs

### Requirement: block-refuses-whole-command
When `clm hook guard` refuses a `Bash` command (exit 2), under any rule and for any agent harness, an ask that the agent harness can't confirm and so renders as a refusal included, the reason it writes to stderr SHALL end with one line saying that the whole command line was refused and nothing in it ran, the commands chained before and after the refused part included, so the session must run again any of them it still needs. Claude Code, and an agent harness that refuses an ask, cancel the whole tool call on a pre-tool hook's exit 2, while each rule's reason speaks only of the part it judged, so without this line a session reads the commands before the refused one as having run. The line SHALL come once, after every rule's reason and after what the agent harness adds to a refused ask, however many rules refused the command. A command the guard allows, or asks about on an agent harness that confirms it (exit 0), SHALL NOT carry it.

#### Scenario: a commit chained before a piped land
- GIVEN the commands `git commit -m m && clm land impl/x | tail` and `git add f && git push origin HEAD:main`, from a repository under CLM
- WHEN `clm hook guard` runs on each
- THEN each exits 2, and the last line of its reason says the whole command line was refused and nothing in it ran, naming `nothing in it ran`

#### Scenario: two rules refuse one command
- GIVEN the command `git push origin main; clm land spec/x | tail`, from a repository under CLM
- WHEN `clm hook guard` runs on it
- THEN it exits 2 with a reason naming `guard-blocks-push-to-main` and `guard-refuses-piped-land`, and `nothing in it ran` appears once, on its last line

#### Scenario: an ask the agent harness refuses
- GIVEN a pre-tool event for `git commit -m m && clm approve spec/x`
- WHEN `clm hook guard --agent-harness codex` and `clm hook guard --agent-harness none` run on it
- THEN each exits 2 with a reason that says to run the command in the human's own terminal, and `nothing in it ran` appears once, on its last line

#### Scenario: allowed and asked commands
- GIVEN the commands `git commit -m m && clm land impl/x` and `clm approve spec/x`
- WHEN `clm hook guard` runs on each
- THEN neither's output names `nothing in it ran`

### Requirement: guard-per-agent-harness
`clm-guard` SHALL offer `guard_hook(agent_harness, event)`, which reads the event as `agent-harness/hook-events` reads that agent harness's events, judges the event's shell command and `cwd` by the same rules whatever the agent harness, and returns the stdout, stderr and exit code `agent-harness/hook-events` gives for the decision under that agent harness, with the line `block-refuses-whole-command` adds to an ask it gives as a refusal: an allow prints nothing and exits 0; a block writes its reason to stderr and exits 2; an ask, on an agent harness that can confirm, prints the `ask` permission decision's JSON and exits 0, and on one that can't, writes its reason and a line naming the human's own terminal to stderr and exits 2. An event for a tool other than `Bash`, the shell tool as every agent harness's event names it, and an event that can't be parsed, SHALL be allowed, as `approve-guard` says for every other event. `guard` and `guard_within` SHALL return `agent-harness`'s `Decision`, and `ask_json(reason)` SHALL be the `ask` permission decision's JSON that `agent-harness/hook-events` gives for Claude Code, so each format is written once.

#### Scenario: push to main under Codex
- GIVEN a Codex pre-tool event for `git push origin main` in a repository under CLM
- WHEN `guard_hook` runs for `Codex`
- THEN it returns Codex's block, naming `guard-blocks-push-to-main`

#### Scenario: approve under Codex
- GIVEN a Codex pre-tool event for `clm approve spec/x`
- WHEN `guard_hook` runs for `Codex`
- THEN it returns Codex's block, its reason saying to run the command in the human's own terminal

#### Scenario: Claude Code unchanged
- GIVEN a Claude Code `PreToolUse` event for `clm approve spec/x`
- WHEN `guard_hook` runs for `ClaudeCode`
- THEN it returns the `ask` permission decision's JSON and exit 0

#### Needs
- agent-harness/hook-events
- agent-harness/abilities

#### Serves
- capability/work-from-any-agent-harness
- architecture/agent-harness-adapters
- architecture/land-locally

### Requirement: commands-in-crate
`clm-guard` SHALL offer `clm hook guard` as a call of its own, `guard_command`, that takes the arguments the command line gives after `clm hook guard`, the text on standard input, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself. It SHALL load the `defects` setting under the environment lookup it is given, not the process's. It SHALL take no repository root, since `clm hook guard` has no `--root` and the guard places each command by its event's `cwd`. For arguments it can't parse, an option it doesn't have (`--root` among them) or `--agent-harness` with no value, it SHALL write a usage error that starts `error:` and names the argument to the error stream, nothing to the output stream, and return 2. Given `--help` or `-h`, it SHALL write `clm hook guard`'s help to the output stream, nothing to the error stream, and return 0: the description ``PreToolUse: exit 2 with the reason when a Bash command would cut a release or push to `main`; print the `ask` decision when it would run `clm approve`, or file a CLM defect as an issue``, the line `Usage: clm hook guard [OPTIONS]`, and the option `--agent-harness <AGENT_HARNESS>`, described as ``The agent harness running the hook: `claude-code` (the default), `codex` or `none` `` with `[default: claude-code]`. With an `--agent-harness` name `agent-harness` doesn't know, it SHALL write ``unknown agent harness `<name>` `` and a newline to the error stream, nothing to the output stream, and return 1, a status no agent harness reads as a block, so the command goes through as when the guard can't run (`guard-fails-open`). When a write to either stream fails, the call SHALL return 2. The one exception is the unknown-agent-harness line, whose failed write returns 1, as that case does; no other case returns 1.

#### Scenario: a command called in a test
- GIVEN a fixture repository
- WHEN `clm-guard`'s call runs with the arguments of `clm hook guard` and two buffers
- THEN the buffers hold what `clm hook guard` prints to standard output and to standard error there, and the call returns the status `clm hook guard` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN it is passed `--nope`, `--root .`, or `--agent-harness` with no value
- THEN it returns 2 with a usage error starting `error:` and naming the option in the error buffer, nothing in the output buffer, and its caller's process goes on

#### Scenario: the help
- GIVEN the same call
- WHEN it is passed `--help`, and `-h`
- THEN each returns 0 with nothing in the error buffer, and the output buffer holds the description ``PreToolUse: exit 2 with the reason when a Bash command would cut a release or push to `main`; print the `ask` decision when it would run `clm approve`, or file a CLM defect as an issue``, the line `Usage: clm hook guard [OPTIONS]` and the option `--agent-harness <AGENT_HARNESS>` with `[default: claude-code]`, the description being the buffer's first line

#### Scenario: an agent harness it doesn't know
- GIVEN a `PreToolUse` event for `clm approve spec/x`
- WHEN the call runs on it with `--agent-harness nope`
- THEN it returns 1, the error buffer holding exactly ``unknown agent harness `nope` `` and a newline, and the output buffer nothing

#### Scenario: settings from the call's environment
- GIVEN an event whose `cwd` has no settings, for `clm defect --title x < report.md`, and a process environment with no `CLM_DEFECTS`
- WHEN the call runs on it with a lookup that holds no `CLM_DEFECTS`, and with one that holds `CLM_DEFECTS` `backlog`
- THEN the first returns 2 with a reason naming `defect-ask-shows-report` in the error buffer, and the second returns 0 with both buffers empty

#### Scenario: a stream that fails
- GIVEN an output stream whose every write fails, with an event for `clm approve spec/x`, and an error stream whose every write fails, with an event for `git push origin HEAD:main` from a repository under CLM
- WHEN the call runs on each
- THEN each returns 2

#### Serves
- architecture/path-ownership
- capability/prove-by-tests
