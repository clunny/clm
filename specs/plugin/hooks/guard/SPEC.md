---
title: Guard
status: active
owners: ["@clunny"]
depends_on: [plugin.hooks, plugin.hooks.guard.shell, config, vcs, harness]
paths: ["crates/clm-guard/Cargo.toml", "crates/clm-guard/src/*.rs", "crates/clm-guard/tests/guard.rs", "crates/clm-guard/tests/git_layer.rs", "crates/clm-guard/tests/budgets.rs"]
successor: ""
serves: [capability/guard-sessions, capability/work-from-any-harness, architecture/approval-policy, architecture/guard-blocks-push-to-main, architecture/harness-adapters]
---

## Contract
The rules behind `clm hook guard` (`hook-logic-in-clm`): which Bash commands the guard asks the human about or blocks, and why. It asks the human to confirm `clm approve`, `clm capabilities decide` and `clm architecture decide` (`approve-in-session`), blocks pushes to `main` from a repository under CLM (`guard-blocks-push-to-main`), release runs (`humans-cut-releases`) and a `clm land` whose output is piped (`guard-refuses-piped-land`), and asks about `clm defect` when the report leaves the project. Crate `clm-guard`: `guard(event)`, which returns a `Decision` (`Allow`, `Ask(reason)` or `Block(reason)`); `guard_within(event, budgets)`, the same judgement with the time `Budgets` its git queries and host lookups may spend, `guard` being `guard_within` with `Budgets::HOOK` (`hook-budgets`); and `ask_json(reason)`, the `ask` permission decision's JSON; `crates/clm`'s `clm hook guard` prints and exits by them. How a command is read before the rules judge it is `plugin.hooks.guard.shell`.

## Design
Crate `clm-guard`, at `crates/clm-guard/` (`guard-split`). Its module `shell` is `plugin.hooks.guard.shell`: `guard` walks the command once and judges each piece the walk yields, with that piece's checkout. It loads the `defects` setting through `clm-hooks`' `settings(cwd)`, the one way the hooks and `clm defect` load settings, so they agree on the repository, and tells whether a push is under CLM with `clm-hooks`' `clm_root`, the same way. The scenarios that run the built binary (a fake `gh` or `git` on the `PATH`, the defect ask) are tested in `crates/clm/tests/guard.rs`, since Cargo gives only the root package's tests the binary (`tests-count-where-cited`). The scenarios of a query or lookup that runs out of time call `guard_within` with budgets of about a second, in `crates/clm-guard/tests/budgets.rs`, a test file of its own whose one test function puts the hanging fakes first on its process's `PATH`: `nextest` runs each test in a process of its own, and with one function a plain `cargo test` shares that environment with no other test. A test file cites this component's requirements or the shell's, never both, so each counts toward one component's size.

A `cargo run` is how this repository's `CLAUDE.md` tells sessions to run CLM, so it is the documented way, not obfuscation, but the guard doesn't work out which package it runs or which tokens reach the binary (`guard-asks-any-cargo-run`, `approve-guard`). Asking too often is the safe side, so another binary's `approve` and a value that happens to be the word, as in `--title approve`, are asked about too. A segment's `git` call is found the same way as a plain `clm` call, so prose that quotes a command, such as a pull request body, isn't mistaken for one.

**Pushes** (`push-guard`, `host-routes-guard`). The command hasn't been allowed yet, so the guard passes git nothing from it beyond the options the requirement lists. A query git can't answer (no upstream, no tracking ref for the destination, or a `simple` push it would refuse anyway) decides nothing. `@{push}` follows `push.default=upstream` on a branch cut from `origin/main`. The push and lookup budgets (`hook-budgets`) together stay under Claude Code's 60-second hook timeout, which would let the command through. A `remote.<name>.push` refspec counts whichever remote it names, since asking which remote a push uses would need more of git's rules than a backstop is worth. The branch is always `main`, the only base CLM's flow uses (`no-stacked-prs`). GraphQL's ref IDs can't be read, so `updateRef` and `updateRefs` are blocked for any ref. CLM never merges a pull request (`land-locally`), so none of the host routes to `main` is part of its flow. `clm land` pushes `main` from inside the binary, not through a guarded `git push`, so it passes; what keeps a session from landing an unapproved branch that needs review is `clm land`'s gate, which is deterministic (`land-locally`). The guard is a backstop for accidents, not a shell parser: `eval`, a command held in a variable, an `export`ed `GIT_DIR`, a script file, a refspec `xargs` reads from stdin, a `cd` into a directory on a hung file system (resolving it isn't a git query, so it has no budget), a command's output piped into a shell (`echo '...' | bash`), a refspec held in a variable (`git push origin $B`, or a `for` loop's variable), which a session's own scripts write for their branches so that treating it as possibly `main` would block them, a GraphQL query read from a file, and the contents API writing a file to `main` get through.

**Repositories not under CLM** (`push-guard-scoped-to-clm`). The guard protects the `main` that `clm land` owns, so a push from a scratch repository a session makes to try something, or from another project where the plugin is installed for the user, is not judged. The test is made only for a push a rule would block, so other pushes pay nothing for it. `clm_root` alone can't tell a directory in no git repository from one in a repository without `specs/SPEC.md`, so the guard also looks for the `.git` above; the `git ls-files` query then finds a CLM repository in a subdirectory of a larger one, which shares its `main`. Whatever the guard can't place counts as under CLM, so a push from a checkout under CLM is always judged. These get through, as a backstop for accidents accepts: a push from a repository not under CLM whose remote is a CLM repository's (telling would mean reading the remote); a push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (a `git clone --no-checkout`, or a worktree at a commit before `specs/` existed); one whose work tree is moved by an exported variable or a `core.worktree` set in its config rather than on the command; and the file-system walk, like a `cd`, has no budget on a hung file system. The host's routes are judged whatever the checkout, since the repository they write to is named by a path or resolved by `gh`, not by the checkout alone.

**Re-run lookups** (`release-run-lookup`). With no `-R`/`--repo`, the guard asks `gh repo view` which repository a re-run targets, since `gh api` takes `{owner}/{repo}` from the checkout but its host from `GH_HOST` or the default host, not from that repository.

**Approval** (`approve-in-session`). The `ask` permission decision is `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": ...}}` on stdout and exit 0. Claude Code shows the human the `ask` decision's reason before the command runs, in any permission mode; it doesn't promise to show the command beside it, so a reason that asks the human to judge text quotes that text. Unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve.

## Requirements

### Requirement: push-guard
`clm hook guard` SHALL exit 2, writing a reason that names `guard-blocks-push-to-main` to stderr, for a `Bash` command with a segment that runs `git push` under CLM (`push-guard-scoped-to-clm`) and:
- has an `--all` or `--mirror` token; or
- has a refspec whose destination is `main` or `refs/heads/main`; or
- has no refspec, or only `HEAD`, and, in the checkout it pushes from, the current branch is `main`, or a `remote.<name>.push` value, from the segment's `-c` options or from `git config --get-regexp '^remote\..*\.push$'`, has a destination that reaches `main` as an explicit refspec's would (`main`, `heads/main`, `refs/heads/main`, empty, or a wildcard that can match `main`); or
- has no refspec, and `git rev-parse --symbolic-full-name @{push}` in that checkout prints `refs/remotes/<remote>/main`, `<remote>` holding no `/`. Only `push.default` and a configured `remote.<name>.push` move where a bare `HEAD` goes, and the second is checked above, so `@{push}` isn't asked for a push of `HEAD`.

Each of these queries SHALL run in that checkout, with those of the segment's `-c` options whose name starts with `push.`, `remote.` or `branch.` before the subcommand, its `--git-dir` and `--work-tree` options, each alone or with `=` and the value glued on, and its `GIT_DIR` and `GIT_WORK_TREE` assignments in the environment; a query that fails decides nothing. The queries of one command SHALL share the push budget (`hook-budgets`), counted from the start of its first push query: a query still running when the budget runs out is killed, a query not yet started then SHALL NOT be started, and either SHALL block the push, with a reason that names `guard-blocks-push-to-main`, says git didn't answer in time so the guard couldn't tell whether the push reaches `main`, and suggests pushing with an explicit `HEAD:<branch>`. Other `-c` options SHALL NOT be passed, since the command hasn't been allowed yet. The checkout is the segment's (`guard-checkout`), then moved by the segment's `-C` directory. For these cases the reason SHALL say which test decided (the branch is `main`, the branch pushes to `main`, or a configured push refspec sends to `main`) and suggest pushing with an explicit `HEAD:<branch>`. When the guard keeps a `cd` it couldn't follow for the segment (`guard-checkout`), the reason for these cases SHALL also say that it couldn't follow that `cd`, quoting it as kept (such as `cd w`, `cd $W` or `cd`), and that it judged the push from the checkout it fell back to, instead of saying that is the checkout the push runs from. The reason for a query that ran out of time doesn't change.

A segment runs git when its command (`guard-command-position`) is `git` or ends in `/git`. Its subcommand is the first token after that which isn't an option or the value of `-C`, `-c`, `--git-dir`, `--work-tree` or `--namespace`. Its refspecs are the positional arguments after the first (the remote), skipping the values of `-o`, `--push-option`, `--repo`, `--receive-pack` and `--exec`. A refspec's destination is the part after its last `:`, or the whole refspec when it has none, without a leading `+`.

Pushing to or deleting any other branch SHALL stay allowed.

#### Scenario: ways to reach main
- GIVEN the commands `git push origin HEAD:main`, `git push --force origin main`, `git push origin +main`, `git push origin impl/x/b:refs/heads/main`, `git -C ../w push origin --delete main` and `git push --mirror origin`, from a repository under CLM
- WHEN `clm hook guard` runs on each
- THEN each exits 2 naming `guard-blocks-push-to-main`

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

#### Scenario: a push git doesn't answer
- GIVEN a fake `git`, first on the `PATH` the guard runs with, that writes its process ID to a file and then `exec`s a sleep of 60 seconds
- WHEN `guard_within` runs with a push budget of 1 second on `git push` and on `git push origin impl/x/b`
- THEN the first returns `Block` within 5 seconds, with a reason that says git didn't answer in time and suggests `HEAD:<branch>`, and no process with the recorded ID is left once the guard has exited; the second returns `Allow` without running the fake `git`

#### Scenario: prose about pushing
- GIVEN the commands `gh pr create --title t --body "never run git push origin main"` and `git commit -m "explain git push origin main"`
- WHEN `clm hook guard` runs on each
- THEN each exits 0

#### Needs
- plugin.hooks.guard.shell/guard-checkout

### Requirement: release-guard
`clm hook guard` SHALL exit 2, writing a reason that names `humans-cut-releases` to stderr, for a `Bash` command with a segment that:
- runs `gh workflow run` whose workflow argument (its first word after `workflow run` that isn't an option or the value of `-R`/`--repo`) names the release workflow or the site workflow: a file name `release.yml`, `release.yaml`, `site.yml` or `site.yaml`, a path ending in one of them after a `/`, or `release` or `site`, each in any case; or
- runs `gh release` with `create`, `edit`, `upload`, `delete` or `delete-asset`; or
- runs `gh api` with a token containing `/dispatches` or `/releases`, and with a `-f`, `-F`, `--field`, `--raw-field` or `--input` token, or with `-X` or `--method` set to anything but `GET`; or
- runs `gh run rerun` with a run ID, or with a job ID given by `-j` or `--job` (`-j <id>`, `-j<id>`, `--job <id>` or `--job=<id>`), when a run it re-runs is a release run or can't be looked up (`release-run-lookup`, `flaky-failures-rerun`). A site run counts as a release run: re-running it would deploy the site again. A segment with both is judged by each. Re-running a release run would publish, merge or deploy again. A `gh run rerun` with neither stays allowed: `gh` then asks for a run interactively, which a session's Bash can't answer.

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

#### Scenario: creating a release directly
- GIVEN the command `gh release create v0.2.0 --generate-notes`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

#### Scenario: dispatching through the API
- GIVEN the command `gh api repos/o/r/actions/workflows/release.yml/dispatches -f ref=main -f inputs[version]=0.2.0`
- WHEN `clm hook guard` runs on it
- THEN it exits 2

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

#### Serves
- capability/guard-sessions

### Requirement: defect-approval
`clm hook guard` SHALL judge a `Bash` command with a segment that runs `clm defect`, or a `cargo run` that `approve-guard` treats as running it, whose arguments (for a `cargo run`, its tokens after the subcommand) hold neither `--help` nor `-h`, when the `defects` setting for the event's `cwd` is `ask` or can't be loaded and no other guard rule blocks the command (`defect-ask-shows-report`). A segment's report *can be shown* when its standard input (`guard-standard-input`) is a heredoc the shell sends as written. When every such segment's report can be shown, the guard SHALL print `hookSpecificOutput` with `permissionDecision: "ask"` and exit 0, with a reason that quotes, for each such segment in order, its own text as written, without its body, and its report, the body whole, says the report leaves the project as a public issue, and says that, when no one can approve, the defect goes under `## CLM defects` in the pull request body instead; no reason the guard gives SHALL call a branch's pull request a page. When a segment's report can't be shown, it SHALL exit 2, writing to stderr a reason that names `defect-ask-shows-report`, says the guard can't show the report to the human, and tells the session to send the report as the body of a `<<'EOF'` heredoc on the `clm defect` line. When another guard rule also asks, it SHALL print one `ask` decision whose reason gives both rules' reasons; a refusal wins over any ask. A heredoc with no line ending it is no heredoc (`guard-heredocs`), so its report can't be shown. No guard rule SHALL match the heredoc body of such a segment. With `backlog` or `off` it SHALL treat the segment like any other.

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
`clm hook guard` SHALL read a `PreToolUse` event from stdin and, for a `Bash` command, judge each segment of it separately. It SHALL answer with the `ask` permission decision on stdout and exit 0, with a reason that names `humans-review-intent` and the command, when no other guard rule blocks the command and a segment runs `clm approve`, `clm capabilities decide` or `clm architecture decide`, outside the heredoc bodies that `guard-heredocs` says match no rule. It SHALL NOT ask about reading a capability's or an architectural constraint's cost (`clm capabilities`, `clm capabilities <slug>`, `clm architecture`, `clm architecture <slug>`), about `clm review`, which is no command, about `clm land`, or about a `gh` call that adds or removes a label. For every other event, including input it can't parse, it SHALL exit 0 and print nothing.

A segment runs `clm` when its command (`guard-command-position`) is `clm` or ends in `/clm`, and `clm`'s arguments are the tokens after it. A segment is a `cargo run` when its command is `cargo` or ends in `/cargo` and its subcommand is `run` or `r`; the subcommand is the first token after `cargo` that doesn't start with `-` or `+` and isn't the value of `--color`, `--config`, `-C` or `-Z`. The guard SHALL NOT work out which package or binary a `cargo run` runs, or which of its tokens reach the binary (`guard-asks-any-cargo-run`): it SHALL treat a `cargo run` as running `clm approve` when one of its tokens after the subcommand is exactly `approve`, as running `clm capabilities decide` when one of them is exactly `capabilities` and a later one exactly `decide`, as running `clm architecture decide` when one of them is exactly `architecture` and a later one exactly `decide`, and as running `clm defect` when one of them is exactly `defect`. A cargo alias that runs `run` under another name isn't a `cargo run`. Every guard rule that looks for a `clm` call SHALL use these definitions.

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

#### Serves
- capability/guard-sessions
- capability/review-capability-cost
- architecture/approval-policy
- architecture/humans-review-intent
- architecture/guard-blocks-push-to-main

### Requirement: git-through-layer
`clm-guard` SHALL list `clm-git` among its dependencies and SHALL run no process itself: every `git` and `gh` call goes through `clm-git`'s `run_within` (`git-plumbing-layer`), and it SHALL bound a call's time only through `run_within`'s limit, never by waiting on a channel for a thread that runs the call (`bounded-runs`).

#### Scenario: covered by the scan
- GIVEN the workspace
- WHEN a test in `clm-guard` reads `crates/clm-guard/Cargo.toml` and every file under `crates/clm-guard/src`, its `shell` module included
- THEN `clm-git` is a dependency and no file holds `Command::new`, `std::process` or `recv_timeout`

#### Serves
- architecture/reuse-first

#### Needs
- vcs/bounded-runs

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
`clm-guard` SHALL offer `Budgets`, a push budget and a lookup budget, with `Budgets::HOOK` a push budget of 20 seconds and a lookup budget of 30 seconds, and `guard_within(event, budgets)`, which SHALL judge `event` as `guard` does, its `push-guard` queries sharing `budgets`' push budget and its `release-guard` lookups its lookup budget. `guard(event)` SHALL return what `guard_within(event, Budgets::HOOK)` returns, and `clm hook guard` SHALL judge through `guard`. No environment variable, setting or argument SHALL change the budgets `clm hook guard` uses: together they keep the hook under Claude Code's 60-second hook timeout, which lets the command through.

#### Scenario: the hook's budgets
- GIVEN `clm-guard`
- WHEN a test reads `Budgets::HOOK`
- THEN its push budget is 20 seconds and its lookup budget 30 seconds

#### Scenario: nothing shortens the hook's budgets
- GIVEN the workspace
- WHEN a test in `clm-guard` reads every file under `crates/clm-guard/src` and `crates/clm/src`
- THEN no file under `crates/clm/src` names `guard_within` or `Budgets`, and no file under `crates/clm-guard/src` outside a `//` comment calls `std::env::var` or `var_os` with a name holding `BUDGET`

#### Scenario: the hook judges with the hook's budgets
- GIVEN the workspace
- WHEN a test in `clm-guard` reads `crates/clm-guard/src/lib.rs`
- THEN the body of `guard` is one call of `guard_within` with `event` and `Budgets::HOOK`

#### Serves
- architecture/guard-blocks-push-to-main

### Requirement: push-guard-scoped-to-clm
`push-guard`'s cases SHALL hold only for a push under CLM: when a case would block a push that isn't under CLM, the guard SHALL allow it instead, whatever its refspecs and options (`guard-blocks-push-to-main`). A push isn't under CLM when all of these hold: the event has a `cwd`; the push's checkout (the segment's, moved by its `-C` directories), canonicalized (symlinks and `..` resolved), is an existing directory; `plugin.hooks`' `clm_root` finds no CLM repository from that directory, and that directory or one of its ancestors holds `.git`, which together mean the first directory holding `.git` going up is reached with none up to and including it holding `specs/SPEC.md` (`reuse-first`); the segment has no `--git-dir` or `--work-tree` option, no `-c` option whose name is `core.worktree` or `core.bare`, and no `GIT_DIR`, `GIT_WORK_TREE` or `GIT_COMMON_DIR` assignment; and no `cd` the guard couldn't follow is kept for the push, as `push-guard`'s reasons would name it: a kept `cd` counts unless the push's own absolute `-C`, or an absolute `env -C` or `sudo -D` (`guard-checkout`), set the checkout; and, last, `git ls-files -z -- ':(top,glob)**/specs/SPEC.md'`, run in the checkout as `push-guard`'s queries are, within the same budget, succeeds and prints nothing, so a CLM repository in a subdirectory of the git repository keeps its `main` guarded from anywhere in that git repository. The test SHALL be made only for a push a `push-guard` case would block, the `--all`, `--mirror` and explicit refspec cases included, so a push of another branch runs no query for it; a query that fails or runs out of the budget leaves the push under CLM. Every other push, such as one from an event with no `cwd`, from a directory in no git repository, from a checkout the guard fell back to, from a `-C` path that isn't a directory, or naming another git directory or work tree, counts as under CLM, since the guard can't tell which repository it reaches. A scratch repository made with `git init` inside a checkout under CLM is a repository of its own, so a push from it isn't under CLM.

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

#### Needs
- plugin.hooks.guard.shell/guard-checkout

### Requirement: host-routes-guard
`clm hook guard` SHALL exit 2, writing a reason that names `guard-blocks-push-to-main` to stderr, for a segment that runs `gh`, found as `release-guard` finds it, whatever its checkout, under CLM or not, with:
- `api`, a token whose path, before any `?` and without a trailing `/`, ends in `git/refs/heads/main`, and a write: a `-f`, `-F`, `--field`, `--raw-field` or `--input` token, or `-X` or `--method` set to anything but `GET`; or
- `api`, a path ending in `/merges`, a write, and a field whose value, after the option or its `=`, is `base=main` or `base=refs/heads/main`, or an `--input` token; or
- `api`, a path ending in `/pulls/<number>/merge`, and a write; or
- `pr merge`; or
- `api graphql` and a token containing `mergePullRequest`, `mergeBranch`, `enablePullRequestAutoMerge` or `updateRef`.

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
- host/git-through-layer

### Requirement: block-refuses-whole-command
When `clm hook guard` blocks a `Bash` command (exit 2), under any rule, the reason it writes to stderr SHALL end with one line saying that the whole command line was refused and nothing in it ran, the commands chained before and after the refused part included, so the session must run again any of them it still needs. Claude Code cancels the whole tool call on a `PreToolUse` hook's exit 2, while each rule's reason speaks only of the part it judged, so without this line a session reads the commands before the refused one as having run. The line SHALL come once, after every rule's reason, however many rules refused the command. A command the guard allows or asks about SHALL NOT carry it.

#### Scenario: a commit chained before a piped land
- GIVEN the commands `git commit -m m && clm land impl/x | tail` and `git add f && git push origin HEAD:main`, from a repository under CLM
- WHEN `clm hook guard` runs on each
- THEN each exits 2, and the last line of its reason says the whole command line was refused and nothing in it ran, naming `nothing in it ran`

#### Scenario: two rules refuse one command
- GIVEN the command `git push origin main; clm land spec/x | tail`, from a repository under CLM
- WHEN `clm hook guard` runs on it
- THEN it exits 2 with a reason naming `guard-blocks-push-to-main` and `guard-refuses-piped-land`, and `nothing in it ran` appears once, on its last line

#### Scenario: allowed and asked commands
- GIVEN the commands `git commit -m m && clm land impl/x` and `clm approve spec/x`
- WHEN `clm hook guard` runs on each
- THEN neither's output names `nothing in it ran`

### Requirement: guard-per-harness
`clm-guard` SHALL offer `guard_hook(harness, event)`, which reads the event with `harness`'s `parse_event` for that harness, judges the event's shell command and `cwd` by the same rules whatever the harness, and returns the stdout and exit code `harness`'s `render` gives for that harness, so an `ask` on a harness that can't confirm blocks the command. An event for another tool than the harness's shell tool SHALL be allowed, as a non-`Bash` event is today, and an event that can't be parsed SHALL return what the guard returns for one today. `guard` and `guard_within` SHALL return `harness`'s `Decision`, and `ask_json` SHALL be `harness`'s Claude Code rendering of an `Ask`, so each format is written once.

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
- harness/hook-events
- harness/abilities

#### Serves
- capability/work-from-any-harness
- architecture/harness-adapters
- architecture/guard-blocks-push-to-main
