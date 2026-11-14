---
title: "With host = \"none\", the clone keeps a remote of its own"
date: 2026-11-07
status: accepted
---

Refines `host-adapters` (whose `none` first answered every operation with nothing to do), `no-local-land-with-remote` and `land-locally` (which first landed on the local `main` alone for `none`). A repository with `host = "none"` had no remote, and only `clm land` knew it: it landed on the local `main` and printed `(local)`. Every other step of the loop reads `origin`: `clm rebase` rebases onto `origin/main`, `clm next` and `clm proposals` fetch first, claims are compare-and-swap pushes (`lease-claims`), and the skills and the prompts `clm draft` and `clm run` write cut their worktrees from `origin/main` and gate against it. So with no remote nothing but a land ran (GitHub issues #285 and #283).

With `host = "none"` and no `origin`, `clm` now makes `clm/remote.git` in the clone's common git directory a bare repository and adds it as `origin` by its absolute path (`host/none-keeps-a-remote`). It does so before any command but `--version`, `help`, `guide` and `hook` (`clm/none-remote-before-commands`), and `clm init` does so after writing a `clm.toml` that says `none` (`init/init-keeps-a-remote`). When it creates the remote in a clone that has a `main`, `clm` says to run `clm reconnect`, which pushes `main` and the ledgers; it pushes nothing itself, so `main` moves only through `clm land` and `clm reconnect` (`land-locally`, `guard-blocks-push-to-main`). A `none` repository with an `origin` other than its own gets a warning on every command naming `host = "git"`. Every worktree of a clone shares its common git directory, so this remote is the one shared copy every session of the clone sees, as `origin` is on any other host: a claim is still a leased push, and of two racing sessions one wins.

From then on `none` differs from `git` only in what `clm init` writes and in having no server: every command, skill and prompt runs unchanged, and `land`, `land.approval` and `ledger` lose the branches that skipped the fetch and the push for `none`.

Rejected:
- **The clone as its own remote**: read the local branch wherever `origin/<branch>` is read, fetch and push nothing, and take claims with `git update-ref <ref> <new> <old>`, which is as atomic as a leased push. It would be a mode in about 30 source files across `sync`, `orchestrator` and its children, `land`, `view` and `host`, a second claim mechanism beside `lease-claims`, and a conditional base in every skill and prompt that names `origin/main`: two ways of doing each step, kept in step by hand (`redesign-before-workaround`).
- **Accept that `none` only lands**, and tell a person who wants to build to add a bare repository as `origin` with `host = "git"`. `host-adapters` promises the loop works with no host, and that setup is what CLM can do itself (`reuse-first`).
- **Create the remote only in `clm init`, and have `clm land` refuse without it.** A repository set up before would fail at `clm rebase` or `clm next` with git's own error, never reaching CLM's message.

Consequences:
- A repository that chose `none` while having a real `origin` now pushes its lands there. `none` means no remote (`no-local-land-with-remote`); such a repository should say `git`.
- `origin`'s path is absolute, since git reads a relative local path against the directory a command runs in, which differs per worktree. Moving the repository's directory needs `git remote set-url origin <new path>`.
- Moving to a real remote is `git remote set-url origin <url>`, setting `host`, and `clm reconnect`, which now fetches with `--prune` so the old remote's tracking refs don't stand for the new one's branches.
