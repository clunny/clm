---
component: plugin.hooks.guard
base: { contract: "e760d43f2984451f", design: "1524b044c75ea912" }
---

## MODIFIED Contract
The rules behind `clm hook guard` (`hook-logic-in-clm`): which Bash commands the guard asks the human about or blocks, and why. It asks the human to confirm `clm approve`, `clm capabilities decide` and `clm architecture decide` (`approval-policy`), blocks pushes to `main` from a repository under CLM (`guard-blocks-push-to-main`), release runs (`release-by-dispatch`) and a `clm land` whose output is piped (`guard-refuses-piped-land`), and asks about `clm defect` when the report leaves the project. Crate `clm-guard`: `guard(event)`, which returns a `Decision` (`Allow`, `Ask(reason)` or `Block(reason)`); `guard_within(event, budgets)`, the same judgement with the time `Budgets` its git queries and host lookups may spend, `guard` being `guard_within` with `Budgets::HOOK` (`hook-budgets`); and `ask_json(reason)`, the `ask` permission decision's JSON; `crates/clm`'s `clm hook guard` prints and exits by them. How a command is read before the rules judge it is `plugin.hooks.guard.shell`.

## MODIFIED Design
Crate `clm-guard`, at `crates/clm-guard/` (`guard-split`). Its module `shell` is `plugin.hooks.guard.shell`: `guard` walks the command once and judges each piece the walk yields, with that piece's checkout. It loads the `defects` setting through `clm-hooks`' `settings(cwd)`, the one way the hooks and `clm defect` load settings, so they agree on the repository, and tells whether a push is under CLM with `clm-hooks`' `clm_root`, the same way. The scenarios that run the built binary (a fake `gh` or `git` on the `PATH`, the defect ask) are tested in `crates/clm/tests/guard.rs`, since Cargo gives only the root package's tests the binary (`tests-count-where-cited`). The scenarios of a query or lookup that runs out of time call `guard_within` with budgets of about a second, in `crates/clm-guard/tests/budgets.rs`, a test file of its own whose one test function puts the hanging fakes first on its process's `PATH`: `nextest` runs each test in a process of its own, and with one function a plain `cargo test` shares that environment with no other test. A test file cites this component's requirements or the shell's, never both, so each counts toward one component's size.

A `cargo run` is how this repository's `CLAUDE.md` tells sessions to run CLM, so it is the documented way, not obfuscation, but the guard doesn't work out which package it runs or which tokens reach the binary (`guard-asks-any-cargo-run`, `approve-guard`). Asking too often is the safe side, so another binary's `approve` and a value that happens to be the word, as in `--title approve`, are asked about too. A segment's `git` call is found the same way as a plain `clm` call, so prose that quotes a command, such as a pull request body, isn't mistaken for one.

**Pushes** (`push-guard`, `host-routes-guard`). The command hasn't been allowed yet, so the guard passes git nothing from it beyond the options the requirement lists. A query git can't answer (no upstream, no tracking ref for the destination, or a `simple` push it would refuse anyway) decides nothing. `@{push}` follows `push.default=upstream` on a branch cut from `origin/main`. The push and lookup budgets (`hook-budgets`) together stay under Claude Code's 60-second hook timeout, which would let the command through. A `remote.<name>.push` refspec counts whichever remote it names, since asking which remote a push uses would need more of git's rules than a backstop is worth. The branch is always `main`, the only base CLM's flow uses (`no-stacked-prs`). GraphQL's ref IDs can't be read, so `updateRef` and `updateRefs` are blocked for any ref. CLM never merges a pull request (`land-locally`), so none of the host routes to `main` is part of its flow. `clm land` pushes `main` from inside the binary, not through a guarded `git push`, so it passes; what keeps a session from landing an unapproved branch that needs review is `clm land`'s gate, which is deterministic (`land-locally`). The guard is a backstop for accidents, not a shell parser: `eval`, a command held in a variable, an `export`ed `GIT_DIR`, a script file, a refspec `xargs` reads from stdin, a `cd` into a directory on a hung file system (resolving it isn't a git query, so it has no budget), a command's output piped into a shell (`echo '...' | bash`), a refspec held in a variable (`git push origin $B`, or a `for` loop's variable), which a session's own scripts write for their branches so that treating it as possibly `main` would block them, a GraphQL query read from a file, and the contents API writing a file to `main` get through.

**Repositories not under CLM** (`push-guard-scoped-to-clm`). The guard protects the `main` that `clm land` owns, so a push from a scratch repository a session makes to try something, or from another project where the plugin is installed for the user, is not judged. The test is made only for a push a rule would block, so other pushes pay nothing for it. `clm_root` alone can't tell a directory in no git repository from one in a repository without `specs/SPEC.md`, so the guard also looks for the `.git` above; the `git ls-files` query then finds a CLM repository in a subdirectory of a larger one, which shares its `main`. Whatever the guard can't place counts as under CLM, so a push from a checkout under CLM is always judged. These get through, as a backstop for accidents accepts: a push from a repository not under CLM whose remote is a CLM repository's (telling would mean reading the remote); a push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (a `git clone --no-checkout`, or a worktree at a commit before `specs/` existed); one whose work tree is moved by an exported variable or a `core.worktree` set in its config rather than on the command; and the file-system walk, like a `cd`, has no budget on a hung file system. The host's routes are judged whatever the checkout, since the repository they write to is named by a path or resolved by `gh`, not by the checkout alone.

**Re-run lookups** (`release-run-lookup`). With no `-R`/`--repo`, the guard asks `gh repo view` which repository a re-run targets, since `gh api` takes `{owner}/{repo}` from the checkout but its host from `GH_HOST` or the default host, not from that repository.

**Approval** (`approval-policy`). The `ask` permission decision is `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "ask", "permissionDecisionReason": ...}}` on stdout and exit 0. Claude Code shows the human the `ask` decision's reason before the command runs, in any permission mode; it doesn't promise to show the command beside it, so a reason that asks the human to judge text quotes that text. Unattended sessions run under `dontAsk`, which denies `ask`, so they can never approve.

## MODIFIED Requirements

### Requirement: release-guard <!-- base: bbbfbcb2409d870c -->
`clm hook guard` SHALL exit 2, writing a reason that names `humans-cut-releases` to stderr, for a `Bash` command with a segment that:
- runs `gh workflow run` whose workflow argument (its first word after `workflow run` that isn't an option or the value of `-R`/`--repo`) names the release workflow or the site workflow: a file name `release.yml`, `release.yaml`, `site.yml` or `site.yaml`, a path ending in one of them after a `/`, or `release` or `site`, each in any case; or
- runs `gh release` with `create`, `edit`, `upload`, `delete` or `delete-asset`; or
- runs `gh api` with a token containing `/dispatches` or `/releases`, and with a `-f`, `-F`, `--field`, `--raw-field` or `--input` token, or with `-X` or `--method` set to anything but `GET`; or
- runs `gh run rerun` with a run ID, or with a job ID given by `-j` or `--job` (`-j <id>`, `-j<id>`, `--job <id>` or `--job=<id>`), when a run it re-runs is a release run or can't be looked up (`release-run-lookup`). A site run counts as a release run: re-running it would deploy the site again. A segment with both is judged by each. Re-running a release run would publish, merge or deploy again. A `gh run rerun` with neither stays allowed: `gh` then asks for a run interactively, which a session's Bash can't answer.

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
