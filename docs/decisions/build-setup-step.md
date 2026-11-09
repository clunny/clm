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

**What `clm init` seeds.** With `--lang typescript`, `[build] setup = "test ! -f package.json || npm ci"`, so a repository with no code yet passes it. Rust needs none: cargo resolves from the worktree's own manifest and compiles into its own `target/`. Python gets none: a shared virtualenv is chosen by whatever runs the tests, so a setup alone can't redirect it, and the project decides where its environment lives. `/clm:init` tells the human the rule and, for an existing codebase, proposes the setup its toolchains need (`plugin.init/init-skill-build-setup`); an existing project is told by an advised note migration (`upgrade/migration-build-setup`).

**Alternatives.**
- *Only state the rule in the build prompt.* Rejected: the failure is silent, and each session would rediscover what to install.
- *Have CLM install per toolchain.* Rejected: CLM would name toolchains, and get a project's own arrangement wrong.
- *Run the setup from `clm run`.* Rejected: `clm run` runs in the orchestrating checkout, before Claude Code makes the build's worktree.
- *A worktree-creation hook of Claude Code.* Rejected: it would tie a rule every project carries to one agent's hook, and drafts' code-fix branches and attended sessions would still need the command.
