---
title: "The push guard judges only pushes from repositories under CLM"
date: 2026-11-07
status: accepted
---

Refines `guard-blocks-push-to-main`. The guard blocked every `git push` that reached `main`, from whatever checkout it ran in. A session in a CLM repository that made a throwaway repository in its scratchpad (`git init`, its own remote, no `specs/SPEC.md`) to try a host behaviour was blocked from `git push -u origin main` there, and had to rename the scratch repository's default branch. The plugin can also be installed for the user rather than the project, and then every session, in any repository, had its pushes to `main` blocked.

`clm land` is the only way onto `main` in a repository under CLM (`land-locally`); a repository with no `specs/SPEC.md` has no `clm land`, and its `main` is not CLM's to protect. So the guard now judges a `git push` only when it is under CLM, and lets the rest through without asking git anything.

**How it tells.** Only for a push the guard would otherwise block, so other pushes pay nothing. First the walk `clm-hooks`' `clm_root` already makes for the hooks and `clm defect` (`reuse-first`): going up from the push's checkout, the first directory holding `.git` is reached without any directory up to it holding `specs/SPEC.md`. Then one `git ls-files` query, within the push budget, for a committed `specs/SPEC.md` anywhere in that git repository, since a CLM repository may sit in a subdirectory of a larger one and share its `main` (`clm`'s root spec allows it), and a push from the larger repository's top must stay blocked (`land-locally`). A scratch repository inside a CLM checkout has its own `.git`, so it is a repository of its own. Whatever the guard can't place counts as under CLM, so a push from a checkout under CLM is never less guarded than before: a directory in no git repository, a checkout it fell back to after a `cd` it couldn't follow, a `-C` directory that doesn't exist, and a push that names its git directory or work tree (`--git-dir`, `--work-tree`, `GIT_DIR`, `GIT_WORK_TREE`).

**What stays.** The host's routes to `main` through `gh` are judged whatever the checkout: a `gh api` path names any repository, and `gh` resolves a pull request's repository from its own settings as well as the checkout. A push from a clone of a CLM repository whose index holds no `specs/SPEC.md` (`git clone --no-checkout`, a worktree at a commit before `specs/` existed) gets through, as does a push from a scratch repository straight to a CLM repository's URL; reading the remote to tell would cost more than a backstop for accidents is worth (`guard-fails-open`, `hook-logic-in-clm`).

**Alternatives considered.**
- *Ask instead of block outside CLM.* Unattended sessions run under `dontAsk`, which denies an ask, so a draft or build spiking a host behaviour would still be stopped, and an attended session would be asked about a repository CLM has no rule for.
- *Tell by `clm.toml`.* A repository under CLM is one with `specs/SPEC.md`: that is what `clm init` writes, what `/clm:init` checks for, and what the hooks already use. A second test could disagree with them.
- *Leave it, and have sessions name their scratch branch something else.* That is the workaround the item reported, a rule a session must know to step around (`defect-reports`).
