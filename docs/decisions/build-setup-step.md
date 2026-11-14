---
title: "A build runs the project's declared setup before its first test run"
date: 2026-11-09
status: accepted
---

A project declares in `clm.toml`'s `[build] setup` the shell command that makes a fresh worktree ready to run its tests, such as one installing its dependencies into it (`config/build-setup-setting`). `clm setup` runs it in the CLM root of the checkout it runs in (`orchestrator.run/setup-command`), and `clm run`'s and `clm draft`'s prompts have the session run `clm setup` before its first test run and again when its dependencies change (`orchestrator.run/prompt-setup`, `orchestrator.triage/draft-prompt-setup`). The rule it serves is the carried constraint `tests-run-in-their-own-tree`.

**Why.** Each build runs in a worktree under `.claude/worktrees/`, inside the main checkout, with nothing installed. A toolchain whose lookup reaches a parent directory resolves the main checkout's packages instead of failing: Node walks up to the main checkout's `node_modules`, and a virtualenv the checkouts share, holding an editable install, imports the main checkout's source. The build's tests then run against code that isn't the branch's, so a test-first red or green means nothing (GitHub issue #406 saw it three times in one session). The failure is silent, so a rule that only asks builds to take care would not be followed reliably; a declared step CLM runs is.

**Why the project declares it, as it declares `[land] check`.** Which toolchain a project uses, and where its dependencies must go for its tests to find them, only the project knows. CLM naming toolchains (detecting a `package.json`, creating a virtualenv) would put per-toolchain knowledge in CLM's core and still miss the project's own arrangement. A shell command the project owns is the shape `[land] check` and `[verify] check` already have (`reuse-first`).

**Why a `clm` command runs it.** Claude Code's worktree isolation refuses a script path it can't show stays inside the worktree, as it refused `.github/scripts/ci-check.sh` in a build (`land-check-command`). A `clm` command names no path for it to judge, as `clm land --check-only` doesn't.

**Why `[land] check` doesn't use it.** CI, `[verify]` and a land's own tree run the check in trees no build prepared, so the check stays self-sufficient, installing what it needs itself, as the starter `scripts/clm-check.sh` does with `npm ci`. Running the setup before every land's check too would cost each land an install for nothing.

**What `clm init` seeds.** With `--lang typescript`, `[build] setup = "test ! -f package.json || npm ci"`, so a repository with no code yet passes it. Rust needs none: cargo resolves from the worktree's own manifest and compiles into its own `target/`. Python gets none: a shared virtualenv is chosen by whatever runs the tests, so a setup alone can't redirect it, and the project decides where its environment lives. `/clm:init` tells the human the rule and, for an existing codebase, proposes the setup its toolchains need; an existing project is told by an advised note migration (`upgrade/migration-build-setup`).

**Alternatives.**
- *Only state the rule in the build prompt.* Rejected: the failure is silent, and each session would rediscover what to install.
- *Have CLM install per toolchain.* Rejected: CLM would name toolchains, and get a project's own arrangement wrong.
- *Run the setup from `clm run`.* Rejected: `clm run` runs in the orchestrating checkout, before Claude Code makes the build's worktree.
- *A worktree-creation hook of Claude Code.* Rejected: it would tie a rule every project carries to one agent's hook, and drafts' code-fix branches and attended sessions would still need the command.

## The rule in full

`ARCHITECTURE.md`'s entry `tests-run-in-their-own-tree` states this in short (`human-layer-entries-stay-short`); the rule in full:

A build's tests SHALL run against the code and the dependencies of the tree it builds in, never against another checkout's: a toolchain whose lookup reaches a parent directory, or an environment the clone's checkouts share, must not resolve the main checkout's packages or source in its place. A build's tree is a fresh worktree inside the main checkout with nothing installed, so the project declares in `[build] setup` the command that makes such a tree ready to run its tests, such as one installing its dependencies into it, and every session that tests code in such a tree (a `clm run` build, a draft's code fix, a skill's hand-run fix) runs it with `clm setup` before its first test run there; CLM names no toolchain, and the project's command holds what its toolchains need (`build-setup-step`).

**Scenario: a package lookup that walks up**
- GIVEN a project whose test runner looks for packages in the parent directories of the tree it runs in, a main checkout with its packages installed, and a build in a fresh worktree under it
- WHEN the build runs its first test
- THEN `clm setup` has run the project's `[build] setup` in the worktree, and the test runs against the worktree's own packages and code

**Guarantee**
A build's red and green test runs exercise the branch's own code with the dependencies its own tree declares, so a test-first build's result means what it says, and what a build saw pass is what its land checks.

**Untested**
That a session runs `clm setup` before its first test, and again when its dependencies change, rests on the prompts and skills that tell it to, as `path-ownership`'s rule rests on the build prompt: no program sees a session's test runs. Whether a project's toolchains reach outside the tree they run in, and whether its `[build] setup` installs everything its tests need, are things CLM can't see; a gap shows as a test that passes in a build and fails in a fresh tree, or the reverse. `[land] check` stays a command that needs no setup before it, since CI and `[verify]` run it in trees no build prepared.
