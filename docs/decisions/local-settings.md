---
title: "A clone's own settings live in an untracked clm.local.toml in its main worktree"
date: 2026-10-27
status: accepted
---

A fifth settings layer, `clm.local.toml`, sits between the environment variable and the repository's `clm.toml`: a key's value comes from its variable, then `clm.local.toml`, then `clm.toml`, then the user's `config.toml`, then its default (`layered-settings`). It is like Claude Code's `settings.local.json`: settings for this repository on this machine, never committed. `clm init` adds it to `.gitignore`.

**Why a layer per clone.** `max_builds` (`build-places`) needs to differ per repository on each machine: how many builds a machine can hold depends on the repository's build size and the machine's disk. The variable alone resets with every shell, the user's `config.toml` applies to every repository, and `clm.toml` is shared by every clone. Putting the cap per machine only, like `writer`, was rejected by the human for that reason.

**Where it is read.** From the CLM repository's directory in the clone's main worktree, the first worktree `git worktree list --porcelain` lists, whichever worktree `clm` runs in. Builds, drafts and session branches run in linked worktrees, and a linked worktree never holds an untracked file of the main one, so a file read from the current worktree would be missing exactly where the cap matters. One file per clone serves all its worktrees. `clm-config` finds the main worktree from the files git keeps (the worktree's `.git` file and the git directory's `commondir`), so it still runs no git.

**Every key.** The file may set every key, `writer` and the repository-only keys included. `writer` is refused in `clm.toml` because a committed value would make everyone write as one person; a local file is one person's by construction. The repository-only keys are refused in the user's file so nobody weakens a repository's checks from outside it; a clone's own file is inside the clone, where its owner could edit `clm.toml` locally anyway, and the land's checks and approvals are recorded against what ran. A missing file is no layer; an invalid one is an error naming the file, as for `clm.toml`. A bare repository has no main worktree, so it has no local layer.

**Tables are one key.** `[node_size]`, `[land]` and `[[detectors]]` come whole from the first layer that sets them; layers never merge within a table, so what a file says is all of that table, and errors and `clm config get` name the one file the value came from.

(Amended 2026-11-05, `20261105-clm-upgrade`: every key but `upgraded` and `declined_migrations`, which record the repository's state and only `clm.toml` may set; and `skip_migrations` only this file may set, so a skip of a broken migration never reaches another clone (`upgrade-migrations`).)

(Amended 2026-11-12, `20261112-config-env`: `[env]` is the one table whose layers merge, name by name, since its variables are independent of one another (`env-table`).)
