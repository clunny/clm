---
component: plugin.hooks.guard
base: { contract: "f4fab32142cf4b47", design: "6f6069619144dacf" }
---

## MODIFIED Contract

The rules behind `clm hook guard` (`hook-logic-in-clm`): which Bash commands the guard asks the human about or blocks, and why. It asks the human to confirm `clm approve`, `clm capabilities decide` and `clm architecture decide` (`approval-policy`), blocks pushes to `main` from a repository under CLM (`guard-blocks-push-to-main`), release runs (`release-by-dispatch`) and a `clm land` whose output is piped (`guard-refuses-piped-land`), and asks about `clm defect` when the report leaves the project. Crate `clm-guard`: `guard(event)`, which returns a `Decision` (`Allow`, `Ask(reason)` or `Block(reason)`); `guard_within(event, budgets)`, the same judgement with the time `Budgets` its git queries and host lookups may spend, `guard` being `guard_within` with `Budgets::HOOK` (`hook-budgets`); and `ask_json(reason)`, the `ask` permission decision's JSON; `clm hook guard`'s call, below, prints and exits by them. How a command is read before the rules judge it is `plugin.hooks.guard.shell`.

`clm-guard` offers `clm hook guard` as a call (`commands-in-crate`, `commands-live-in-their-crates`): it takes the command's arguments as the command line gives them, the text on standard input, the environment as a lookup and a stream each for standard output and standard error, and returns the exit status. It takes no repository root: the guard places each command by its event's `cwd`. `crates/clm` routes the command to it and adds nothing to what it writes.



## MODIFIED Design

Crate `clm-guard`, at `crates/clm-guard/` (`guard-split`). Its module `shell` is `plugin.hooks.guard.shell`: `guard` walks the command once and judges each piece the walk yields, with that piece's checkout. It loads the `defects` setting with `clm-config`'s `load`, at the CLM root `clm-hooks`' `clm_root` finds from the event's `cwd` and under the call's own environment lookup: the root search the hooks and `clm defect` use, so they agree on the repository, with the environment the call was given, which `clm-hooks`' `settings(cwd)` reads from the process instead. It tells whether a push is under CLM with `clm_root`, the same way. The scenarios that need a fake `gh` or `git` on the `PATH`, and the defect ask, are tested in `crates/clm-guard/tests/guard.rs`: each calls `clm-guard`'s call of `clm hook guard` and sets `PATH` on its own process for the length of the call (`commands-live-in-their-crates`). The scenarios of a query or lookup that runs out of time call `guard_within` with budgets of about a second, in `crates/clm-guard/tests/budgets.rs`, a test file of its own whose one test function puts the hanging fakes first on its process's `PATH`: `nextest` runs each test in a process of its own, and with one function a plain `cargo test` shares that environment with no other test. A test file cites this component's requirements or the shell's, never both, so each counts toward one component's size.

A `cargo run` is how this repository's `CLAUDE.md` tells sessions to run CLM, so it is the documented way, not obfuscation, but the guard doesn't work out which package it runs or which tokens reach the binary (`guard-asks-any-cargo-run`, `approve-guard`). Asking too often is the safe side, so another binary's `approve` and a value that happens to be the word, as in `--title approve`, are asked about too. A segment's `git` call is found the same way as a plain `clm` call, so prose that quotes a command, such as a pull request body, isn't mistaken for one.

**Pushes** (`push-guard`, `host-routes-guard`). The command hasn't been allowed yet, so the guard passes git nothing from it beyond the options the requirement lists. A query git can't answer (no upstream, no tracking ref for the destination, or a `simple` push it would refuse anyway) decides nothing. `@{push}` follows `push.default=upstream` on a branch cut from `origin/main`. The push and lookup budgets (`hook-budgets`) together stay under Claude Code's 60-second hook timeout, which would let the command through. A `remote.<name>.push` refspec counts whichever remote it names, since asking which remote a push uses would need more of git's rules than a backstop is worth. The branch is always `main`, the only base CLM's flow uses (`no-stacked-prs`). GraphQL's ref IDs can't be read, so `updateRef` and `updateRefs` are blocked for any ref. CLM never merges a pull request (`land-locally`), so none of the host routes to `main` is part of its flow. `clm land` pushes `main` from inside the binary, not through a guarded `git push`, so it passes; what keeps a session from landing an unapproved branch that needs review is `clm land`'s gate, which is deterministic (`land-locally`). The guard is a backstop for accidents, not a shell parser: `eval`, a command held in a variable, an `export`ed `GIT_DIR`, a script file, a refspec `xargs` reads from stdin, a `cd` into a directory on a hung file system (resolving it isn't a git query, so it has no budget), a command's output piped into a shell (`echo '...' | bash`), a refspec held in a variable (`git push origin $B`, or a `for` loop's variable), which a session's own scripts write for their branches so that treating it as possibly `main` would block them, a GraphQL query read from a file, and the contents API writing a file to `main` get through.

**Repositories not under CLM** (`push-guard-scoped-to-clm`). The guard protects the `main` that `clm land` owns, so a push from a scratch repository a session makes to try something, or from another project where the plugin is installed for the user, is not judged. The test is made only for a push a rule would block, so other pushes pay nothing for it. `clm_root` alone can't tell a directory in no git repository from one in a repository without `specs/SPEC.md`, so the guard also looks for the `.git` above; the `git ls-files` query then finds a CLM repository in a subdirectory of a larger one, which shares its `main`. Whatever the guard can't place counts as under CLM, so a push from a checkout under CLM is always judged. These get through, as a backstop for accidents accepts: a push from a repository not under CLM whose remote is a CLM repository's (telling would mean reading the remote); a push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (a `git clone --no-checkout`, or a worktree at a commit before `specs/` existed); one whose work tree is moved by an exported variable or a `core.worktree` set in its config rather than on the command; and the file-system walk, like a `cd`, has no budget on a hung file system. The host's routes are judged whatever the checkout, since the repository they write to is named by a path or resolved by `gh`, not by the checkout alone.

**Re-run lookups** (`release-run-lookup`). With no `-R`/`--repo`, the guard asks `gh repo view` which repository a re-run targets, since `gh api` takes `{owner}/{repo}` from the checkout but its host from `GH_HOST` or the default host, not from that repository.

**Approval** (`approval-policy`). The `ask` permission decision is `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": ...}}` on stdout and exit 0. Claude Code shows the human the `ask` decision's reason before the command runs, in any permission mode; it doesn't promise to show the command beside it, so a reason that asks the human to judge text quotes that text. Unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve.



## MODIFIED Requirements



### Requirement: commands-in-crate <!-- base: 11cfbd2d387a71c9 -->
`clm-guard` SHALL offer `clm hook guard` as a call of its own that takes the command's arguments as the command line gives them, the text on standard input, the environment as a lookup, and a stream each for standard output and standard error. The call SHALL write to those streams everything the command prints, each line on the stream and with the prefix the command gives it, and SHALL return the command's exit status, never ending the process itself. It SHALL take no repository root, since `clm hook guard` has no `--root` and the guard places each command by its event's `cwd`; for arguments it can't parse it SHALL write the usage error to the error stream and return 2. What `crates/clm` passes it today, of the settings and of other components' results, it takes as further arguments, since a crate loads only the settings of a component it depends on. Every test of `plugin.hooks.guard`'s requirements that sat in the top-level test files of `crates/clm/tests/` SHALL sit in `plugin.hooks.guard`'s own files and call `clm-guard` itself, this call where the requirement states what a command prints or how it exits: none runs the `clm` binary (`commands-live-in-their-crates`).

#### Scenario: a command called in a test
- GIVEN a fixture repository, and a test in `plugin.hooks.guard`'s own files
- WHEN it calls `clm-guard`'s call with the arguments of `clm hook guard` and two buffers
- THEN the buffers hold what `clm hook guard` prints to standard output and to standard error there, and the call returns the status `clm hook guard` exits with

#### Scenario: arguments it can't parse
- GIVEN the same call
- WHEN a test passes it an option `clm hook guard` doesn't have
- THEN it returns 2 with the usage error in the error buffer, and the test's process goes on

#### Serves
- architecture/path-ownership
- capability/prove-by-tests

### Requirement: hook-budgets <!-- base: 0a955c8104cc23c0 -->
`clm-guard` SHALL offer `Budgets`, a push budget and a lookup budget, with `Budgets::HOOK` a push budget of 20 seconds and a lookup budget of 30 seconds, and `guard_within(event, budgets)`, which SHALL judge `event` as `guard` does, its `push-guard` queries sharing `budgets`' push budget and its `release-guard` lookups its lookup budget. `guard(event)` SHALL return what `guard_within(event, Budgets::HOOK)` returns, and `clm hook guard`, whose call renders for the agent harness its arguments name and loads settings with its own environment lookup (`guard-per-agent-harness`, `commands-in-crate`), SHALL judge with `Budgets::HOOK` too. No environment variable, setting or argument SHALL change the budgets `clm hook guard` uses: together they keep the hook under Claude Code's 60-second hook timeout, which lets the command through.

#### Scenario: the hook's budgets
- GIVEN `clm-guard`
- WHEN a test reads `Budgets::HOOK`
- THEN its push budget is 20 seconds and its lookup budget 30 seconds

#### Scenario: nothing shortens the hook's budgets
- GIVEN the workspace
- WHEN a test in `clm-guard` reads every file under `crates/clm-guard/src` and `crates/clm/src`
- THEN no file under `crates/clm/src` names `guard_within` or `clm-guard`'s `Budgets`, and no file under `crates/clm-guard/src` outside a `//` comment calls `std::env::var` or `var_os` with a name holding `BUDGET`

#### Scenario: the hook judges with the hook's budgets
- GIVEN the workspace
- WHEN a test in `clm-guard` reads `crates/clm-guard/src/lib.rs`
- THEN the body of `guard` is one call of `guard_within` with `event` and `Budgets::HOOK`, and the judging that `clm hook guard`'s call runs passes `Budgets::HOOK`

#### Serves
- architecture/land-locally
