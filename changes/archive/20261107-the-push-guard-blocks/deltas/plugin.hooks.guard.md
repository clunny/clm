---
component: plugin.hooks.guard
base: { contract: "4e5b17974779c4a6", design: "4330a5fa99350bd7" }
---

## MODIFIED Contract
The rules behind `clm hook guard` (`hook-logic-in-clm`): which Bash commands the guard asks the human about or blocks, and why. It asks the human to confirm `clm approve`, `clm capabilities decide` and `clm architecture decide` (`approve-in-session`), blocks pushes to `main` from a repository under CLM (`guard-blocks-push-to-main`), release runs (`humans-cut-releases`) and a `clm land` whose output is piped (`guard-refuses-piped-land`), and asks about `clm defect` when the report leaves the project. Crate `clm-guard`: `guard(event)`, which returns a `Decision` (`Allow`, `Ask(reason)` or `Block(reason)`), and `ask_json(reason)`, the `ask` permission decision's JSON; `crates/clm`'s `clm hook guard` prints and exits by them. How a command is read before the rules judge it is `plugin.hooks.guard.shell`.

## MODIFIED Design
Crate `clm-guard`, at `crates/clm-guard/` (`guard-split`): `lib.rs` (`guard`, `Decision`, `ask_json`, and the `clm` call a segment makes), `push.rs`, `release.rs` and `gh.rs`. Its module `shell` is `plugin.hooks.guard.shell`: `guard` walks the command once and judges each piece the walk yields, with that piece's checkout. It loads the `defects` setting through `clm-hooks`' `settings(cwd)`, the one way the hooks and `clm defect` load settings, so they agree on the repository, and tells whether a push is under CLM with `clm-hooks`' `clm_root`, the same way. It runs `git` and `gh` only through `clm-git` (`git-through-layer`). The scenarios that run the built binary (a fake `gh` or `git` on the `PATH`, the time budgets, the defect ask) are tested in `crates/clm/tests/guard.rs`, since Cargo gives only the root package's tests the binary (`tests-count-where-cited`). A test file cites this component's requirements or the shell's, never both, so each counts toward one component's size.

A `cargo run` is how this repository's `CLAUDE.md` tells sessions to run CLM, so it is the documented way, not obfuscation, but the guard doesn't work out which package it runs or which tokens reach the binary (`guard-asks-any-cargo-run`, `approve-guard`). Asking too often is the safe side, so another binary's `approve` and a value that happens to be the word, as in `--title approve`, are asked about too. A segment's `git` call is found the same way as a plain `clm` call, so prose that quotes a command, such as a pull request body, isn't mistaken for one.

**Pushes** (`push-guard`). The command hasn't been allowed yet, so the guard passes git nothing from it beyond the options the requirement lists. A query git can't answer (no upstream, no tracking ref for the destination, or a `simple` push it would refuse anyway) decides nothing. `@{push}` follows `push.default=upstream` on a branch cut from `origin/main`. The 20-second budget of the push queries and the 30-second budget of the release lookups (`release-guard`) together stay under Claude Code's 60-second hook timeout, which would let the command through; the clock for the push queries starts at the command's first push query. A query that runs out of time blocks the push: git stalling, as on a `--git-dir` on a hung file system, is no answer. A `remote.<name>.push` refspec counts whichever remote it names, since asking which remote a push uses would need more of git's rules than a backstop is worth. The branch is always `main`, the only base CLM's flow uses (`no-stacked-prs`). GraphQL's `updateRef` and `updateRefs` are blocked for any ref, since their ref IDs can't be read. CLM never merges a pull request (`land-locally`), so none of the host routes to `main` is part of its flow. `clm land` pushes `main` from inside the binary, not through a guarded `git push`, so it passes; what keeps a session from landing an unapproved branch that needs review is `clm land`'s gate, which is deterministic (`land-locally`). The guard is a backstop for accidents, not a shell parser: `eval`, a command held in a variable, an `export`ed `GIT_DIR`, a script file, a refspec `xargs` reads from stdin, a `cd` into a directory on a hung file system (resolving it isn't a git query, so it has no budget), a command's output piped into a shell (`echo '...' | bash`), a refspec held in a variable (`git push origin $B`, or a `for` loop's variable), which a session's own scripts write for their branches so that treating it as possibly `main` would block them, a GraphQL query read from a file, and the contents API writing a file to `main` get through.

**Repositories not under CLM** (`push-guard-scoped-to-clm`). The guard protects the `main` that `clm land` owns, so a `git push` from a repository with no `specs/SPEC.md`, such as a scratch repository a session makes to try something, or another project where the plugin is installed for the user, is not judged. The test is made only for a push a rule would block, so other pushes pay nothing for it. It starts with the file system walk `clm_root` makes (`clm_root` alone can't tell a directory in no git repository from one in a repository without `specs/SPEC.md`, so the guard also looks for the `.git` above), and ends with one `git ls-files` query for a committed `specs/SPEC.md` anywhere in the git repository, within the push budget, since a CLM repository may sit in a subdirectory of a larger one (`clm`) and share its `main`. Whatever the guard can't place counts as under CLM: a directory in no git repository (a push from there fails or names a git directory), a checkout it fell back to after a `cd` it couldn't follow, a `-C` directory that doesn't exist, and a push naming its git directory or work tree, so a rule is never less guarded than before for a push from a checkout under CLM. A push from a repository not under CLM whose remote is a CLM repository's, such as `git push <url-of-a-clm-repository> main` from a scratch repository, gets through: telling would mean reading the remote, which a backstop for accidents isn't worth. So does a push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (a `git clone --no-checkout`, or a worktree at a commit before `specs/` existed), and one whose work tree is moved by an exported variable or a `core.worktree` set in its config rather than on the command; and the file-system walk, like a `cd`, has no budget on a hung file system. The host's routes (`gh`) are judged whatever the checkout, since the repository they write to is named by a path or resolved by `gh`, not by the checkout alone.

**Re-run lookups** (`release-run-lookup`). With no `-R`/`--repo`, the guard asks `gh repo view --json url` which repository a `gh run rerun` targets, since `gh api` takes `{owner}/{repo}` from the checkout but its host from `GH_HOST` or the default host, not from that repository; the lookup then asks the host the re-run asks.

**Approval** (`approve-in-session`). The `ask` permission decision is `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": ...}}` on stdout and exit 0. Claude Code then shows the human the reason before the command runs, in any permission mode; it doesn't promise to show the command beside it, so a reason that asks the human to judge text quotes that text; unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve. A `clm defect` ask (`defect-approval`) is made the same way; its reason quotes each `clm defect` line and the report it sends (`defect-ask-shows-report`), says the report leaves the project as a public issue, and that when no one can approve (an unattended session), the defect goes under `## CLM defects` in the pull request body instead.

## MODIFIED Requirements

### Requirement: push-guard <!-- base: c75aa11406fd2598 -->
`clm hook guard` SHALL exit 2, writing a reason that names `guard-blocks-push-to-main` to stderr, for a `Bash` command with a segment that runs `git push` and:
- has an `--all` or `--mirror` token; or
- has a refspec whose destination is `main` or `refs/heads/main`; or
- has no refspec, or only `HEAD`, and, in the checkout it pushes from, the current branch is `main`, or a `remote.<name>.push` value, from the segment's `-c` options or from `git config --get-regexp '^remote\..*\.push$'`, has a destination that reaches `main` as an explicit refspec's would (`main`, `heads/main`, `refs/heads/main`, empty, or a wildcard that can match `main`); or
- has no refspec, and `git rev-parse --symbolic-full-name @{push}` in that checkout prints `refs/remotes/<remote>/main`, `<remote>` holding no `/`. Only `push.default` and a configured `remote.<name>.push` move where a bare `HEAD` goes, and the second is checked above, so `@{push}` isn't asked for a push of `HEAD`.

Each of these queries SHALL run through `clm-git`'s layer in that checkout, with those of the segment's `-c` options whose name starts with `push.`, `remote.` or `branch.` before the subcommand, its `--git-dir` and `--work-tree` options, each alone or with `=` and the value glued on, and its `GIT_DIR` and `GIT_WORK_TREE` assignments in the environment; a query that fails decides nothing. The queries of one command SHALL share a budget of 20 seconds, counted from the start of its first push query, each run through `clm-git`'s `run_within` with what remains of the budget as its limit (`bounded-runs`): a query still running when the budget runs out is killed, a query not yet started then SHALL NOT be started, and either, like any query `run_within` reports as timed out, SHALL block the push, with a reason that names `guard-blocks-push-to-main`, says git didn't answer in time so the guard couldn't tell whether the push reaches `main`, and suggests pushing with an explicit `HEAD:<branch>`. Other `-c` options SHALL NOT be passed, since the command hasn't been allowed yet. The checkout is the segment's (`guard-checkout`), then moved by the segment's `-C` directory. For these cases the reason SHALL say which test decided (the branch is `main`, the branch pushes to `main`, or a configured push refspec sends to `main`) and suggest pushing with an explicit `HEAD:<branch>`. When the guard keeps a `cd` it couldn't follow for the segment (`guard-checkout`), the reason for these cases SHALL also say that it couldn't follow that `cd`, quoting it as kept (such as `cd w`, `cd $W` or `cd`), and that it judged the push from the checkout it fell back to, instead of saying that is the checkout the push runs from. The reason for a query that ran out of time doesn't change.

A segment runs git when its command (`guard-command-position`) is `git` or ends in `/git`. Its subcommand is the first token after that which isn't an option or the value of `-C`, `-c`, `--git-dir`, `--work-tree` or `--namespace`. Its refspecs are the positional arguments after the first (the remote), skipping the values of `-o`, `--push-option`, `--repo`, `--receive-pack` and `--exec`. A refspec's destination is the part after its last `:`, or the whole refspec when it has none, without a leading `+`.

The `git push` cases above SHALL hold only for a push under CLM: when a case would block a push that isn't under CLM, the guard SHALL allow it instead, whatever its refspecs and options (`guard-blocks-push-to-main`). A push isn't under CLM when all of these hold: the event has a `cwd`; the push's checkout (the segment's, moved by its `-C` directories), canonicalized (symlinks and `..` resolved), is an existing directory; `plugin.hooks`' `clm_root` finds no CLM repository from that directory, and that directory or one of its ancestors holds `.git`, which together mean the first directory holding `.git` going up is reached with none up to and including it holding `specs/SPEC.md` (`reuse-first`); the segment has no `--git-dir` or `--work-tree` option, no `-c` option whose name is `core.worktree` or `core.bare`, and no `GIT_DIR`, `GIT_WORK_TREE` or `GIT_COMMON_DIR` assignment; and no `cd` the guard couldn't follow is kept for the push, as the reasons above would name it: a kept `cd` counts unless the push's own absolute `-C`, or an absolute `env -C` or `sudo -D` (`guard-checkout`), set the checkout; and, last, `git ls-files -z -- ':(top,glob)**/specs/SPEC.md'`, run in the checkout as the queries above are, within the same budget, succeeds and prints nothing, so a CLM repository in a subdirectory of the git repository keeps its `main` guarded from anywhere in that git repository. The test SHALL be made only for a push a case above would block, the `--all`, `--mirror` and explicit refspec cases included, so a push of another branch runs no query for it; a query that fails or runs out of the budget leaves the push under CLM. Every other push, such as one from an event with no `cwd`, from a directory in no git repository, from a checkout the guard fell back to, from a `-C` path that isn't a directory, or naming another git directory or work tree, counts as under CLM, since the guard can't tell which repository it reaches. A scratch repository made with `git init` inside a checkout under CLM is a repository of its own, so a push from it isn't under CLM.

It SHALL also exit 2, naming `guard-blocks-push-to-main`, for a segment that runs `gh`, found as `release-guard` finds it, whatever its checkout, since the repository a `gh` call writes to is named by its path or resolved by `gh`, not by the checkout alone, with:
- `api`, a token whose path, before any `?` and without a trailing `/`, ends in `git/refs/heads/main`, and a write: a `-f`, `-F`, `--field`, `--raw-field` or `--input` token, or `-X` or `--method` set to anything but `GET`; or
- `api`, a path ending in `/merges`, a write, and a field whose value, after the option or its `=`, is `base=main` or `base=refs/heads/main`, or an `--input` token; or
- `api`, a path ending in `/pulls/<number>/merge`, and a write; or
- `pr merge`; or
- `api graphql` and a token containing `mergePullRequest`, `mergeBranch`, `enablePullRequestAutoMerge` or `updateRef`.

Pushing to, deleting, or merging into any other branch SHALL stay allowed.

#### Scenario: pushing to main
- GIVEN the command `git push origin HEAD:main`
- WHEN `clm hook guard` runs on it
- THEN it exits 2 naming `guard-blocks-push-to-main`

#### Scenario: other ways to reach main
- GIVEN the commands `git push --force origin main`, `git push origin +main`, `git push origin impl/x/b:refs/heads/main`, `git -C ../w push origin --delete main` and `git push --mirror origin`
- WHEN `clm hook guard` runs on each
- THEN each exits 2

#### Scenario: implicit push from main
- GIVEN an event whose `cwd` is a repository under CLM (it holds `specs/SPEC.md`) on branch `main`, or a directory `sub` of one, and the commands `git push` and `git push origin HEAD`
- WHEN `clm hook guard` runs on each
- THEN each exits 2, with a reason suggesting `HEAD:<branch>`

#### Scenario: a branch that pushes to main
- GIVEN an event whose `cwd` is a repository under CLM with a remote `origin` holding `main`, on branch `impl/x/b` whose upstream is `origin/main`
- WHEN `clm hook guard` runs on `git -c push.default=upstream push`, on `git -c remote.origin.push=HEAD:main push`, and on `git push` with `push.default` set to `upstream` in the repository's config, and again with `remote.origin.push` set to `HEAD:main` there instead
- THEN each exits 2

#### Scenario: another checkout's git directory
- GIVEN an event whose `cwd` is on `impl/x/b`, and `m` a repository on `main`
- WHEN `clm hook guard` runs on `GIT_DIR=m/.git git push` and on `git --git-dir=m/.git push`
- THEN each exits 2

#### Scenario: pushing other branches
- GIVEN the commands `git push -u origin impl/x/b`, `git push --force-with-lease origin impl/x/b`, `git push origin --delete impl/x/b` and `git push origin main:impl/x/b`, `git push` with a `cwd` on branch `impl/x/b` whose upstream is `origin/impl/x/b`, `git push` with a `cwd` on branch `impl/x/c` with no upstream, `git -c push.default=upstream push -u origin HEAD` with a `cwd` on branch `impl/x/b` whose upstream is `origin/main`, and `git -C ../w push` with a `cwd` on `main` and `../w` on `impl/x/b`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: cd into a directory the command creates
- GIVEN an event whose `cwd` is a repository under CLM on `main`, and no directory `w` under it
- WHEN `clm hook guard` runs on `git worktree add -q w -b spec/x && cd w && git commit -m m && git push -u origin HEAD`
- THEN it exits 2 with a reason that quotes `cd w`, says the guard couldn't follow it, and suggests `HEAD:<branch>`, while `git worktree add -q w -b spec/x && cd w && git push -u origin spec/x` exits 0

#### Scenario: moving main through the API
- GIVEN the command `gh api -X PATCH repos/o/r/git/refs/heads/main -f sha=abc`
- WHEN `clm hook guard` runs on it
- THEN it exits 2, while `gh api repos/o/r/git/refs/heads/main` and `gh api -X PATCH repos/o/r/git/refs/heads/mainline -f sha=abc` exit 0

#### Scenario: merging into main through the host
- GIVEN the commands `gh api -X POST repos/o/r/merges -f base=main -f head=impl/x/b`, `gh api repos/o/r/merges --field=base=refs/heads/main -f head=x`, `gh api -X PUT repos/o/r/pulls/3/merge`, `gh pr merge 3 --squash`, `gh api graphql -f query='mutation { mergePullRequest(input: {pullRequestId: "P"}) { clientMutationId } }'` and `gh api graphql -f query='mutation { updateRefs(input: {repositoryId: "R", refUpdates: []}) { clientMutationId } }'`
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Scenario: reading or merging elsewhere through the host
- GIVEN the commands `gh api -X POST repos/o/r/merges -f base=impl/x/b -f head=main`, `gh api repos/o/r/pulls/3/merge`, `gh pr view 3`, `gh pr create --title t --body "then gh pr merge it"` and `gh api graphql -f query='query { repository(owner: "o", name: "r") { id } }'`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Scenario: a push git doesn't answer
- GIVEN a fake `git`, first on the `PATH` the guard runs with, that writes its process ID to a file and then `exec`s a sleep of 60 seconds
- WHEN `clm hook guard` runs on `git push` and on `git push origin impl/x/b`
- THEN the first exits 2 within 25 seconds, with a reason that says git didn't answer in time and suggests `HEAD:<branch>`, and no process with the recorded ID is left once the guard has exited; the second exits 0 without running the fake `git`

#### Scenario: prose about pushing
- GIVEN the commands `gh pr create --title t --body "never run git push origin main"` and `git commit -m "explain git push origin main"`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

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

#### Scenario: the host's routes outside CLM
- GIVEN an event whose `cwd` is a scratch repository with no `specs/SPEC.md`
- WHEN `clm hook guard` runs on `gh pr merge 3 --squash` and on `gh api -X PATCH repos/o/r/git/refs/heads/main -f sha=abc`
- THEN each exits 2 naming `guard-blocks-push-to-main`

#### Needs
- plugin.hooks.guard.shell/guard-checkout
