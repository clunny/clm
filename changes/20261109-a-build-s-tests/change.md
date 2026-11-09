---
status: applied
resolves:
  - a-build-s-tests-can-run-against-the-main-checkout-s-dependencies
---

## Intent

A build's tests run against its own worktree's code and dependencies, never the main checkout's. A project declares in `clm.toml`'s `[build] setup` the command that makes a fresh worktree ready to run its tests, as it declares `[land] check`; `clm setup` runs it; and the `clm run` and `clm draft` prompts have the session run `clm setup` before its first test run. CLM names no toolchain: the project's command holds what its toolchains need. The rule is a new carried constraint, `tests-run-in-their-own-tree` (`replaceable`), so every CLM-built project gets it from the `clm` it runs (`backlog/a-build-s-tests-can-run-against-the-main-checkout-s-dependencies`, GitHub issue #406).

## Problem

Each build runs in a worktree under `.claude/worktrees/`, inside the main checkout, with nothing installed. A toolchain whose lookup reaches the parent directories resolves the main checkout's packages instead of failing: Node walks up to its `node_modules`, and a virtualenv the checkouts share, holding an editable install, imports the main checkout's source. The build's tests then run against code that isn't the branch's, so a test-first red or green can mean nothing, and nothing shows it. Issue #406 saw it three times in one session with Node.

The design isn't wrong so much as incomplete: `tests-run-concurrently` already makes the project responsible for its tests' host resources and gives it a value to key them on; nothing makes it responsible for what its tests resolve, or gives it a step to prepare a fresh tree.

## Approaches

1. **State the rule in the build prompt only.** Cheap, but the failure is silent, and each session would rediscover, per toolchain, what to install. Rejected.
2. **CLM installs per toolchain** (detect `package.json`, make a virtualenv). Puts toolchain knowledge in CLM's core and still misses the project's own arrangement (where its environment lives, which package manager). Rejected (`reuse-first`, `yagni`).
3. **A declared setup command, run by a `clm` command before the first test run** (chosen). The same shape as `[land] check` and `[verify] check`. A `clm` command, not the script itself, because Claude Code's worktree isolation refuses a script path it can't show stays inside the worktree, as it refused `.github/scripts/ci-check.sh` (`land-check-command`).
4. **A Claude Code worktree-creation hook.** Ties a carried rule to one agent's hook, and drafts' code fixes and attended sessions would still need a command. Rejected.

The decision `build-setup-step` records the choice and why `[land] check` doesn't run the setup.

## Outcome

- `ARCHITECTURE.md` gains `tests-run-in-their-own-tree`, a `replaceable` carried constraint with its guarantee, after `tests-run-concurrently`.
- `config`: `[build]` with `setup`, a repository-only table key (`build-setup-setting`); `clm config list` prints it after `land` (`config-list`, modified); the guide's `settings` page gets its section.
- `orchestrator.run`: `clm setup` (`setup-command`), and the build prompt's setup paragraph (`prompt-setup`). The root's dispatch gets the `clm setup` arm in the build, as a command's build edits the root's dispatch files; the glossary lists `clm setup` under Working.
- `orchestrator.triage`: the draft prompt's code-fix paragraph runs `clm setup` too (`draft-prompt-setup`), since a draft's code fix is a build in a worktree like any other.
- `init`: `clm init --lang typescript` seeds `[build] setup = "test ! -f package.json || npm ci"` (`init-build-setup`). Rust needs none (cargo resolves from the worktree's own manifest and target). Python gets none: a shared virtualenv is chosen by whatever runs the tests, so a setup alone can't redirect it, and where the environment lives is the project's choice. The upgrade corpus is built with `--lang rust`, so its `clm.toml` is unchanged.
- `plugin.init`: `/clm:init` tells the human the rule and, for an existing codebase, proposes the setup its toolchains need (`init-skill-build-setup`).
- `upgrade`: an advised note migration, `20261109-a-build-s-tests`, tells existing projects the rule and how to check and set `[build] setup`; the stamp marks it taken (`migration-build-setup`).
- `plugin.build`: builds get the step from `clm run`'s prompt, but the `build` skill's own defect fixes, in a worktree it adds under `.claude/worktrees/`, run `clm setup` too (`build-defect-fixes-run-setup`); so does `propose`'s hand-run code fix (`plugin.propose/propose-fix-runs-setup`). Both added after the architecture audit found them uncovered.
- This repository sets no `[build] setup`: its Rust crates need none, and its npm packages sit in subdirectories (`web/`, `plugin-panel/`, ...) whose `node_modules` a worktree's lookup never reaches from `.claude/worktrees/<name>/web/`, so their tests fail rather than resolve the main checkout's; its check script runs `npm ci` itself. So no `settings-before-use` ordering is needed.

## Constraints

- `[land] check` never runs the setup: CI, `[verify]` and a land's own tree run the check in trees no build prepared, so the check stays self-sufficient.
- `clm setup` takes no lock and sets no variable: it installs, it doesn't test.
- The new key is read before anything sets it (`settings-before-use`): nothing in this repository sets it.

## Edge cases and workarounds

- A setup that Claude Code refuses or denies (the auto-mode classifier judges `clm setup`, which no seeded allow rule covers) stops the build with `stopped: setup failed`, reported with `clm defect`: with no setup, no test result is to be trusted. No allow rule is seeded for `clm setup`, so `init-seeds-permissions` and its migration stay as they are.
- A project with no `[build] setup` whose tests do reach outside the worktree: the build prompt has the session record a `bug` naming `tests-run-in-their-own-tree` and install what its tests need by hand before trusting a result.
- The setup runs again after the session's own edit to the dependency declarations or a `clm rebase` that brought one in; which files those are is the session's judgment, since CLM names no toolchain.

## Open questions

- **An allow rule for `clm setup`** (architecture audit, `unattended-steps-run-clm`). The change seeds none: the decision's seeded list is for commands that write to the branch, `origin` or the backlog, and `clm setup` writes only to the worktree. But it is a step every unattended build now needs, and a classifier denial stops the build with `stopped: setup failed`. Seeding `Bash(clm setup)` and `Bash(clm setup *)` would change `init/init-seeds-permissions`, need a new note migration (the existing one's list is frozen) and this repository's `.claude/settings.json`. Left for the human: accept the classifier's judgement, as the change does, or ask for the rule as a follow-up.
- **`upgrade/migration-build-setup`'s needs.** The builder check judged `config/build-setup-setting` and `orchestrator.run/setup-command` over-declared, since the note only names them. They are kept on purpose: a note must not tell projects to set a key their `clm` rejects or run a command it lacks.

## Builder check and audit

Settled in the change: the settings field's name (`build: Setting<Option<Build>>`), `clm setup`'s `--root` and its message for a command killed by a signal, a stronger no-setup scenario (`declares no setup step`), how `clm draft` gets the setting (passed by `crates/clm`, so `clm-triage` reads no settings), `--lang typescript` among several languages, and the land-dir migration test that must expect the new note beside its own. The audit found hand-run code fixes in fresh worktrees uncovered: `plugin.build/build-defect-fixes-run-setup` and `plugin.propose/propose-fix-runs-setup` now cover them, and the constraint's `#### Untested` says that running `clm setup` rests on the prompts and skills.
