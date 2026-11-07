---
status: applied
resolves: [the-push-guard-blocks-pushes-to-main-in-repositories-that-aren-t]
---

## Intent

Let a session push `main` in a git repository that isn't under CLM. The guard blocked every `git push` reaching `main` from whatever checkout it ran in, so a session that made a throwaway repository in its scratchpad (`git init`, its own remote, no `specs/SPEC.md`) to try a host behaviour was blocked from `git push -u origin main` and had to rename the branch to get past it. A repository with no CLM state has no `clm land`, so `land-locally` has nothing there for the guard to protect. This change scopes `push-guard`'s `git push` cases to pushes under CLM, amends the `guard-blocks-push-to-main` constraint in `ARCHITECTURE.md` to say so, and records the choice as `push-guard-scoped-to-clm`.

## Problem

`guard-blocks-push-to-main` was written for this repository's own sessions, where every checkout is under CLM. But sessions also make repositories of their own, and the plugin can be installed for the user, in which case its guard runs in every repository the user opens. The guard then blocks a push to a `main` no CLM rule owns, and the only way past is a workaround the session must know about.

## Approaches

- **Ask instead of block outside CLM.** Rejected: unattended sessions run under `dontAsk`, which denies an ask, so drafts and builds would still be stopped, and an attended human would be asked about a repository CLM has no rule for.
- **Tell a repository under CLM by `clm.toml`.** Rejected: `specs/SPEC.md` is what `clm init` writes, what `/clm:init` checks for, and what `clm-hooks`' `clm_root` already tests for the hooks and `clm defect`. A second test could disagree with them (`reuse-first`).
- **Scope the host's `gh` routes too.** Rejected: a `gh api` path names any repository, and `gh` resolves a pull request's repository from its own settings as well as the checkout, so the checkout doesn't tell which `main` they move. No reported case needs them scoped (`yagni`).
- **Judge `git push` only under CLM, by `clm_root` and one `git ls-files` query, and count whatever the guard can't place as under CLM (chosen).** The test runs only for a push a rule would block. The file-system walk the guard already depends on finds the git repository; the query finds a committed `specs/SPEC.md` anywhere in it, since a CLM repository may sit in a subdirectory of a larger git repository and share its `main` (architecture audit finding). A push from a checkout under CLM is never less guarded than before.

## Outcome

- `plugin.hooks.guard/push-guard` allows a `git push` it would block when the push isn't under CLM: the event has a `cwd`; the checkout is an existing directory in a git repository, with no `specs/SPEC.md` from it up to the directory holding `.git`; the segment names no git directory or work tree (`--git-dir`, `--work-tree`, `-c core.worktree`, `-c core.bare`, `GIT_DIR`, `GIT_WORK_TREE`, `GIT_COMMON_DIR`); no `cd` the guard couldn't follow is kept for the push; and `git ls-files` finds no committed `specs/SPEC.md` anywhere in the repository. Everything else counts as under CLM. The existing scenarios that run from a git repository now say it is under CLM, and five scenarios are added. It needs `plugin.hooks.guard.shell/guard-checkout`.
- `plugin.hooks.guard.shell/guard-checkout` says `push-guard` reads the kept `cd` to count a push as under CLM, and that its scenarios run in repositories under CLM; the shell's Design says every repository its tests make holds `specs/SPEC.md`. Its build makes the tests' checkout helper in `crates/clm-guard/tests/shell.rs` write `specs/SPEC.md`, so those tests keep seeing blocked pushes once `push-guard` is built.
- `plugin.hooks.guard`'s Contract and Design say pushes to `main` are blocked from repositories under CLM, how the guard tells, and the gaps it leaves.
- `ARCHITECTURE.md`'s `guard-blocks-push-to-main` excepts a push from a git repository not under CLM, counts one it can't tell as under CLM, and gains a scenario for it (hand edit).
- `CAPABILITIES.md`'s `guard-sessions` scenario `push to main` says the session is in a repository under CLM, as the `ARCHITECTURE.md` scenario does (hand edit; architecture audit finding). Its capability test already runs in one, so it doesn't change.
- `docs/decisions/push-guard-scoped-to-clm.md` records the choice and the alternatives.

## Constraints

- Build order: `guard-checkout` first. Its build only adds `specs/SPEC.md` to the shell tests' repositories, which the current guard ignores, so those tests pass before and after `push-guard` is built; built the other way round, `push-guard`'s land would fail the shell tests, whose repositories hold no `specs/SPEC.md`.
- `plugin.hooks.guard`'s own tests that expect a block from a git repository (`crates/clm-guard/tests/guard.rs`) need `specs/SPEC.md` in their fixtures; that is `push-guard`'s build. The tests whose event `cwd` is `/tmp` or a bare temporary directory are in no git repository, so they count as under CLM and don't change, `crates/clm/tests/guard.rs`'s `a push git doesn't answer` among them.
- `/clm:init`'s bootstrap is unchanged: `clm init` writes `specs/SPEC.md` before the bootstrap commit, so the guard still blocks the session from pushing it, and the human pushes it.
- `clm_root` returns nothing both for a directory in no git repository and for one in a repository without `specs/SPEC.md`, so the guard pairs it with its own look for `.git` above the checkout, which needs no change to `plugin.hooks` (builder check finding).
- Edge cases and workarounds known now, each a gap of a backstop stated in the guard's Design: a push from a repository not under CLM straight to a CLM repository's URL; a push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (`git clone --no-checkout`, a worktree at a commit before `specs/` existed); a work tree moved by an exported variable or a configured `core.worktree`. No workaround.

## Open questions

- The skills `build`, `contribution`, `health`, `help`, `propose`, `triage` and `upgrade` say "Never push `main` by any means but `clm land`" without limit (architecture audit finding). Left as written: each skill runs in a project under CLM and speaks of that project's `main`, which this change keeps guarded; a scratch repository a session makes is not the project. Scope the sentence if sessions are seen refusing to push a scratch repository's `main`.
