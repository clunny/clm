---
status: applied
resolves:
  - host-none-still-needs-a-remote-for-rebase-next-claims-and-skills
---

## Intent

Make the whole propose, build and land loop run in a repository with `host = "none"` and no remote, by giving such a clone a remote of its own: a bare repository inside its git directory, which CLM creates and adds as `origin`. Every command and skill then reads, fetches, claims and pushes exactly as on any other host, and the special local-land paths that `host = "none"` needed in `land`, `land.approval` and `ledger` go.

## Problem

`land/offline-land` lets a repository with `host = "none"` land on its local `main`, but nothing else in the loop has a no-remote path: `clm rebase` fetches `origin` and rebases onto `origin/main`, ending `rebase failed`; `clm next` and `clm proposals` fetch first and exit 2, so `/clm:build` and `clm runner` never start; claims take their lock by pushing a branch (`lease-claims`); and the init, propose, build and triage skills, and the prompts `clm draft` and `clm run` write, cut their worktrees from `origin/main` and run `clm gate --base origin/main`. The `host-adapters` constraint promises that a repository works with no host at all, and its scenario covers only proposing, approving and landing. Reported in GitHub issues #285 and #283.

## Approaches

- **The clone keeps a remote of its own (chosen, `none-keeps-its-own-remote`).** With `host = "none"` and no `origin`, `clm` creates a bare repository at `clm/remote.git` in the clone's common git directory and adds it as `origin`. Every worktree of the clone shares it, so it is the shared copy every session sees: claims stay git's compare-and-swap push (`lease-claims` unchanged), `origin/main` exists for every command, prompt and skill, and `clm branch push`, `clm branch delete` and `clm land` work as on `git`. Nothing in `sync`, `orchestrator`, `plugin` or the prompts changes (`reuse-first`); `land`, `land.approval` and `ledger` lose their `host = "none"` branches (`no-dead-code`). The remote is created by `clm` before a command runs, so a repository set up before this needs no step of its own, and by `clm init`, so the bootstrap push to `main` has somewhere to go.
- **The clone is its own remote: every read of `origin/<branch>` reads the local branch, nothing fetches or pushes, and claims lock with `git update-ref <ref> <new> <old>`.** What the item proposed. Rejected: it is a mode switch through every command that reads `origin/main` (about 30 source files across `sync`, `orchestrator`, `orchestrator.run`, `orchestrator.triage`, `orchestrator.checkpoint`, `orchestrator.runner`, `land`, `land.approval`, `view` and `host`), a second claim mechanism beside `lease-claims`, and a conditional base in every skill and prompt that names `origin/main` (nine plugin files and the generated draft and build prompts), each a place for the two modes to drift (`redesign-before-workaround`).
- **Accept: `host = "none"` lands only; a person who wants to build adds a bare repository as `origin` and sets `host = "git"`.** Rejected: `host-adapters` promises the loop works with no host, and the only route left would be a manual setup that `clm init` could do itself.
- **Create the remote only in `clm init`, and have `clm land` refuse with a pointer to it.** Rejected: a repository set up before this would fail at `clm rebase` or `clm next` with git's own error, long before any message of CLM's; creating it before each command covers both with one rule.

## Outcome

- `host/none-keeps-a-remote`: `clm-host`'s `keep_remote(root, host)` creates the bare repository and the `origin` remote when `host` is `none` and the clone has no `origin`, and pushes nothing.
- `clm/none-remote-before-commands`: `clm` calls it before every command but `--version`, `help`, `guide` and `hook`, and when it created the remote and the clone has a `main`, runs `clm reconnect`'s push of `main` and the ledgers, so `main` still moves only through `land` (`land-locally`).
- `init/init-keeps-a-remote`: `clm init` calls it after writing a `clm.toml` with `host = "none"`, so the bootstrap commit's push (`init-next-steps`) has an `origin`.
- `land/offline-land`: `clm land` lands through `origin` on every host; `(local)` lands go. `clm reconnect` stays, for moving onto a newly set remote, and now prunes stale remote-tracking refs when it fetches. `land/selected-tests` loses its no-remote parenthetical, and `land`'s Contract and Design their offline wording.
- `land.approval/approval-from-base` and `signed-land`: the base is always the fetched `origin/main`; the `no remote` scenario goes.
- `ledger/append`: pushes on every host, `none` included.
- `ARCHITECTURE.md`'s `host-adapters` scenario covers building as well as proposing and landing, and names `host/none-keeps-a-remote` among its enforcers; `docs/guide/settings.md` and the glossary say what `none` now does.

## Constraints and edge cases

- **A `host = "none"` repository with a real `origin`** (a person who chose `none` to avoid pull requests): its lands now push `main` to that `origin`, where today they stay local. `no-local-land-with-remote` already reads `none` as "no remote"; such a repository should say `git`. Accepted.
- **Moving the repository directory** leaves `origin`'s absolute path pointing at the old place, and every fetch fails until `git remote set-url origin <new path>`. Git resolves a relative local path against the directory a command runs in, which differs per worktree, so the path must be absolute. Accepted; the settings guide says so.
- **Moving to a real remote**: `git remote set-url origin <url>`, set `host`, then `clm reconnect`, which pushes `main` and the ledgers. Claim and spec branches in flight stay on the old private remote and reach the new one with their next `clm branch push`.
- **Disk**: the private remote holds a second copy of what is pushed to it. Accepted: the same as any remote on the same disk, and bounded by the repository's size.
- **Repositories with lands made locally by an older `clm`**: their local `main` and ledgers are pushed by the reconnect that follows the remote's creation, so their history is kept.
- **`clm init` in a repository with no commit**: the remote is created empty, and the human's push of the bootstrap commit (`init-next-steps`) gives it `main`, as on any host.
