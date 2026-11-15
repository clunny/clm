---
status: applied
resolves: [clm-s-cli-tests-prove-other-components-through-the-binary]
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`: fix the repository's shape. This is its first change of four. Each crate owns its commands, with their arguments, what they print and their exit status, and offers each as a call a test can make in-process. Each component's tests move out of the root's test files into its own crate, unchanged but for the helper that ran the binary. `crates/clm` is left to parse, find the root, load settings and route.

It adds 25 requirements and removes none. No test is dropped: `moved-tests.txt` lists each one that moves.

## Problem

Measured on `main` at `4f96740b`:

- The top-level files of `crates/clm/tests/` hold 1,049 tests in 62 files. 872 of them (83%), in 38 files, cite a requirement of another component; 725 of those run the `clm` binary. They are the root's files because Cargo gives a built binary only to its own package's tests.
- No crate offers a call that takes a command's arguments and streams. Each returns lines or a result, and `crates/clm/src/main.rs` chooses the stream, the `clm <command>:` prefix and the exit status, which is what those tests assert. About 1,850 of its 2,824 lines are that, edited by every build as wiring, though the root's Design calls the file dispatch only.
- So a component's requirement is met partly by the root's code and proven wholly in the root's files. `path-ownership` carries a clause for it, which lets a build change any file that cites its component (`own-files-at-the-gate`), and the mutation pass can't run such a test in the package it mutates.
- The root measures 174,665 code tokens against a budget of 40,000: 31,000 of source and 135,000 of tests, of which 95,000 is fifteen files counted whole to the root because a few of their tests cite it.

Outside `main.rs`, library code reads the process's environment, directory or input in eight files only, so the commands can be called in-process. What a lookup can't carry is the environment of the children a command starts: many of these tests give the binary a `PATH` holding a stand-in for `git`, `gh` or `claude`, a `TMPDIR` or a `GIT_` variable. The Rust tests run one per process under `cargo-nextest`, so a moved test sets those on its own process.

## Approaches

Recorded in `commands-live-in-their-crates`:

- **Each crate owns its commands, arguments included.** Taken. A requirement's code and its test end up in one component, a moved test keeps its arguments and its assertions, and a component that adds an option can prove it in its own files.
- **Leave the tests in place and move their ownership**, a directory or a `tests` glob per component. Not taken: every component's proof would still depend on the root's binary, and `components-own-their-tests` already rejected it here. It is used for the tests that need no binary (`20261115-tests-in-their-own-files`).
- **Assert on what the libraries return today.** Not taken: the tests would stop proving stream and exit status.
- **Argument definitions kept in the root.** Not taken: a new option could be tested only by the root.

## Outcome

- **`commands-in-crate`, one requirement in each of 23 components**: `apply`, `check`, `check.affected`, `check.health`, `check.unreferenced`, `config`, `context`, `effort`, `gate`, `host`, `init`, `land`, `land.approval`, `ledger`, `orchestrator`, `orchestrator.checkpoint`, `orchestrator.run`, `orchestrator.triage`, `plugin.hooks`, `plugin.hooks.guard`, `rename`, `sync` and `view.repo`. Each names the commands its crate offers as a call, and states that every test of the component's requirements calls its own crate from its own files. None needs another, so they build in parallel; three need `model/violation-line`. Each Contract gains the call. `clm approve` is `land.approval`'s, whose requirements state it, so the root comes to depend on `land.approval`.
- **`model/violation-line`**: the line a violation prints as, which `apply`, `gate` and `ledger` print without depending on `check`.
- **`clm/routes-only`** needs all 23, and the two backlog items that empty the root's files block it. The root then prints nothing of its own for a routed command, exits with what the call returns, and proves that with tests that run the binary.
- **The root's Design** says who owns a command, and tells each build how its tests move while the old files still exist.
- **Which commands stay the root's**: `clm contribution`, `clm upgrade`, `clm verify`, `clm machine`, `clm agent-harness`, `clm build` and `clm fingerprint`, whose behaviour the root's own requirements state. Whether those requirements belong to their components is left for the plan's phase 5.

`moved-tests.txt` holds 882 lines, one per test and component it cites: the component, the file, the test's name and whether it runs the binary. 674 lines are tests of the components above; the 208 others, of `ci`, the `plugin` family, `view`, `check.coverage` and `upgrade`, move under `20261115-tests-in-their-own-files`.

**Also, by the human on 2026-11-15, an exception to the freeze:** the builder check runs on the build model, `opus`, wherever it is launched (`model-per-launch`). No requirement states its model, so the decision changes here and the three prompts that name `sonnet` are `backlog/the-builder-check-still-runs-on-sonnet`.

## What it removes

Nothing. No requirement, scenario or test is dropped. A test that can't move as it stands, because what it observes exists only in a process, stays where it is and its build files a `question` item that blocks `clm/routes-only`. A test that cites two components, or the root as well, is never deleted by the first build to reach it: 9 tests are on two components' lines and 6 also cite the root.

One thing changes in what a test proves. `view` states what `clm status` and `clm view` print, and `check.coverage` what `clm pending` and `clm coverage` print, but the crates that offer those commands lie above them. Their 63 tests come to assert the same text on what their own crate returns, and `view.repo/commands-in-crate` and `check/commands-in-crate` prove that the command writes that text unchanged, with its stream and exit status.

## Constraints

- **Every land leaves `main` green.** A build adds its call, points its own arm of `main.rs` at it and moves its own tests in one branch. The clause that lets it delete them from the root's files stays in force until every move is built (`20261115-builds-change-only-their-own-files`).
- **No build edits another component's files**, beyond the wiring and that clause.
- **Adopters.** No file, key or format a project holds changes, so there is no migration.
- **`clm land` gains no check.** The moved tests stop starting a process each, so the land's own check gets faster.
- **Size.** The root falls to about 60,000 code tokens, still over its budget. What remains is `dev-install` and the scans of source and guide text that phase 5 takes.

## Open questions

- 23 builds each edit `crates/clm/src/main.rs`. `clm rebase` reports the conflicts and each session resolves its own, as the root's Design says of wiring.
- A crate the web app compiles to WebAssembly must keep its argument parser out of that build; the root's Design says so and each build chooses how.
- How many tests can't move is not known. The builder check sampled ten: `ledger`'s, `plugin.hooks`' and the guard's move with their helper changed; `host`'s, `orchestrator.run`'s, `init`'s and `orchestrator.checkpoint`'s set the child's `PATH` or `GIT_` variables and need the process-environment rule; and of `land`'s 15, those that copy the binary, replace `git` or run two lands at once can't move as they stand. Each such test is a `question` item for the human, and `routes-only` waits on them.
- A test that sets its process's environment is only safe one per process. `cargo test` on such a crate, which the land's check never runs, would let tests see each other's variables.
