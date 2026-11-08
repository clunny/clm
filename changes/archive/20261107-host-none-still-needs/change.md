---
status: done
resolves:
  - host-none-still-needs-a-remote-for-rebase-next-claims-and-skills
  - a-trunk-not-named-main-cannot-land
---

## Intent

Make the whole propose, build and land loop run in a repository with `host = "none"` and no remote, by giving such a clone a remote of its own: a bare repository inside its git directory, which CLM creates and adds as `origin`. Every command and skill then reads, fetches, claims and pushes exactly as on any other host, and the special local-land paths that `host = "none"` needed in `land`, `land.approval` and `ledger` go.

It also makes `main` the only trunk CLM supports (`trunk-named-main`, `trunk-is-main`) rather than adding a setting for another branch name, and has `clm init` say so up front, with the rename command, in a repository without a `main` (`init/init-names-missing-main`), so the first `clm land` no longer finds it. That resolves `a-trunk-not-named-main-cannot-land` by its warning half and declines its configurable-trunk half, whose cost every later change would pay.

## Problem

`land/offline-land` lets a repository with `host = "none"` land on its local `main`, but nothing else in the loop has a no-remote path: `clm rebase` fetches `origin` and rebases onto `origin/main`, ending `rebase failed`; `clm next` and `clm proposals` fetch first and exit 2, so `/clm:build` and `clm runner` never start; claims take their lock by pushing a branch (`lease-claims`); and the init, propose, build and triage skills, and the prompts `clm draft` and `clm run` write, cut their worktrees from `origin/main` and run `clm gate --base origin/main`. The `host-adapters` constraint promises that a repository works with no host at all, and its scenario covers only proposing, approving and landing. Reported in GitHub issues #285 and #283.

Separately, CLM assumes the trunk is named `main` everywhere, and in a repository on `master` `clm init` succeeds and tells the human to push to `main`; nothing says the branch is missing until the first `clm land` refuses with "no `main` to land on" (#283).

## Approaches

- **The clone keeps a remote of its own (chosen, `none-keeps-its-own-remote`).** With `host = "none"` and no `origin`, `clm` creates a bare repository at `clm/remote.git` in the clone's common git directory and adds it as `origin`. Every worktree of the clone shares it, so it is the shared copy every session sees: claims stay git's compare-and-swap push (`lease-claims` unchanged), `origin/main` exists for every command, prompt and skill, and `clm branch push`, `clm branch delete` and `clm land` work as on `git`. Nothing in `sync`, `orchestrator`, `plugin` or the prompts changes (`reuse-first`); `land`, `land.approval` and `ledger` lose their `host = "none"` branches (`no-dead-code`). The remote is created by `clm` before a command runs, so a repository set up before this needs no step of its own, and by `clm init`, so the bootstrap push to `main` has somewhere to go.
- **The clone is its own remote: every read of `origin/<branch>` reads the local branch, nothing fetches or pushes, and claims lock with `git update-ref <ref> <new> <old>`.** What the item proposed. Rejected: it is a mode switch through every command that reads `origin/main` (about 30 source files across `sync`, `orchestrator`, `orchestrator.run`, `orchestrator.triage`, `orchestrator.checkpoint`, `orchestrator.runner`, `land`, `land.approval`, `view` and `host`), a second claim mechanism beside `lease-claims`, and a conditional base in every skill and prompt that names `origin/main` (nine plugin files and the generated draft and build prompts), each a place for the two modes to drift (`redesign-before-workaround`).
- **Accept: `host = "none"` lands only; a person who wants to build adds a bare repository as `origin` and sets `host = "git"`.** Rejected: `host-adapters` promises the loop works with no host, and the only route left would be a manual setup that `clm init` could do itself.
- **Create the remote only in `clm init`, and have `clm land` refuse with a pointer to it.** Rejected: a repository set up before this would fail at `clm rebase` or `clm next` with git's own error, long before any message of CLM's; creating it before each command covers both with one rule.

For the trunk's name:

- **`main` is the only trunk, and `clm init` says so up front (chosen, `trunk-is-main`).** A new constraint `trunk-named-main` states it, and `init/init-names-missing-main` prints the rename as the first next step when the repository has no `main`. One requirement, no change to any other command.
- **A `trunk` setting in `clm.toml`, read everywhere in place of `main`.** What #283 asked for. Rejected: `main` and `origin/main` are written in about 800 places across specs, code, skills and prompts, and every later change would have to use the setting; a missed literal shows only in a repository on another branch name. The adopter's cost it saves is one rename.
- **Detect the trunk from `origin/HEAD`.** Rejected: absent with `host = "none"`, and may be unset or differ per clone.

## Outcome

- `host/none-keeps-a-remote`: `clm-host`'s `keep_remote(root, host)` creates the bare repository and the `origin` remote when `host` is `none` and the clone has no `origin`, and pushes nothing.
- `clm/none-remote-before-commands`: `clm` calls it before every command but `--version`, `help`, `guide` and `hook`, after the upgrade check; when it created the remote and the clone has a `main`, it says to run `clm reconnect`, and pushes nothing itself, so `main` moves only through `clm land` and `clm reconnect` (`land-locally`, `guard-blocks-push-to-main`). It warns when `host = "none"` has an `origin` other than the clone's own, naming `host = "git"`.
- `init/init-keeps-a-remote`: `clm init` calls it after writing a `clm.toml` with `host = "none"`, so the bootstrap commit's push (`init-next-steps`) has an `origin`.
- `land/offline-land`: `clm land` lands through `origin` on every host; `(local)` lands go. `clm reconnect` stays, for moving onto a newly set remote, and now prunes stale remote-tracking refs when it fetches. `land/selected-tests` loses its no-remote parenthetical, and `land`'s Contract and Design their offline wording.
- `land.approval/approval-from-base` and `signed-land`: the base is always the fetched `origin/main`; the `no remote` scenario goes.
- `ledger/append` and `ledger`'s Design: `append` pushes and `compact` fetches on every host, `none` included.
- The remaining `host = "none"` wording goes: `land`'s step 7 and `land/land-push-retry` (the unpushed-record warning now holds on every host), `land.approval`'s Held paragraph, `sync`'s Design (the fold), and the `host` Contract's opening.
- `init/init-names-missing-main`: in a repository whose current branch isn't `main` and that has no `main` locally or on its remote, `clm init` writes its files and prints, first among its next steps, that CLM lands only on `main` and the `git branch -m <current> main` that renames to it.
- `ARCHITECTURE.md` gains the constraint `trunk-named-main`, enforced by that requirement, and `docs/decisions/trunk-is-main.md` records why.
- `ARCHITECTURE.md`'s `host-adapters` scenario covers building as well as proposing and landing, and names `host/none-keeps-a-remote` among its enforcers; `docs/guide/settings.md` and the glossary say what `none` now does.

## Constraints and edge cases

- **A `host = "none"` repository with a real `origin`** (a person who chose `none` to avoid pull requests): its lands now push `main` to that `origin`, where today they stay local. `no-local-land-with-remote` already reads `none` as "no remote"; such a repository should say `git`, and every `clm` command warns so (`clm/none-remote-before-commands`) rather than a migration, since a migration writes only files and the case is a clone's remote, not a file (`upgrades-carry-migrations`).
- **Moving the repository directory** leaves `origin`'s absolute path pointing at the old place, and every fetch fails until `git remote set-url origin <new path>`. Git resolves a relative local path against the directory a command runs in, which differs per worktree, so the path must be absolute. Accepted; the settings guide says so.
- **Moving to a real remote**: `git remote set-url origin <url>`, set `host`, then `clm reconnect`, which pushes `main` and the ledgers. Claim and spec branches in flight stay on the old private remote and reach the new one with their next `clm branch push`.
- **Disk**: the private remote holds a second copy of what is pushed to it. Accepted: the same as any remote on the same disk, and bounded by the repository's size.
- **Repositories with lands made locally by an older `clm`**: the first command creates the remote and says to run `clm reconnect`, which pushes their `main` and ledgers, so their history is kept. Until then `clm rebase` and `clm next` fail on the missing `origin/main`, as on any host before its first push.
- **`clm init` in a repository with no commit**: the remote is created empty, and the human's push of the bootstrap commit (`init-next-steps`) gives it `main`, as on any host.

## Open questions

- **`plugin/skills/init/SKILL.md`** steps 5 and 6 still branch on "with no `origin`", which `clm init` no longer leaves; `crates/clm/tests/plugin.rs` asserts that sentence. Harmless dead wording, left for a later edit rather than adding a plugin requirement here.
- **`land/reconnect-names-rebase`**'s scenario describes a `main` landed locally while offline, which only an older `clm` can now produce; its build builds that state by hand. Left as written.

## The human's answers

On #283's item, at intake: "should we allow the project to define its own 'main' branch, eg in clm.toml"

On this change's first design, the private bare remote: "would it make more sense to allow local (host="none") setups use local git main instead of origin main?", then "can we take them both now?"

Then: "I'm reconsidering. If this change adds complexity and creates maintenance burdon, maybe we should keep it as origin/main (only support that as an opinionated constraint) and use the origin/main in .clm as the solution chose. What is the correct and what is the maintainable solution?" After the recommendation to keep the private bare remote and require a trunk named `main`, with `clm init` saying so up front, asked whether to fold that into this change: "folded in"

