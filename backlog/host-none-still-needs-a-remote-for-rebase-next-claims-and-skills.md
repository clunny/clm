---
title: "Rebase, next, claims and the skills need a remote even with host = \"none\""
kind: bug
issues: [285, 283]
components: [sync, orchestrator, plugin, host, land]
---

A repository set up with `host = "none"` and no `origin` can land locally (`land/offline-land`), but the rest of the loop can't run there. `clm rebase` always fetches `origin` and rebases onto `origin/main`, ending `rebase failed`; `clm next` and `clm proposals` fetch `origin` first and exit 2, so `/clm:build` and `clm runner` never start; and the init, propose, build and triage skills cut their worktrees from `origin/main`, which doesn't exist. The only working route today is a local bare repository added as `origin` with a host other than `none`, which defeats `host = "none"`.

Expected: with `host = "none"`, every command and skill that reads `origin/main` reads local `main`, and nothing fetches or pushes (rebase, next, proposals, claim, status, triage and the skills' worktrees), so the whole propose, build and land loop works with no remote, as `host-adapters` and the `git-host` capability promise.

Claims take their lock by pushing a branch, so with no remote they need another atomic lock that every worktree of the clone sees, such as a local ref created only when absent (`git update-ref <ref> <new> <zero-oid>`). That is a design question: `lease-claims` words claims as "compare-and-swap pushes", so it needs widening or a decision.

`sync/rebase` and orchestrator's read-from-`origin/main` rule state today's behaviour as written, so this starts on a spec branch. Reported in GitHub issue #285, and in #283's "Related" paragraph.
