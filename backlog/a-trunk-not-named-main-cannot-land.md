---
title: "A repository whose trunk isn't named main can't land, and init doesn't warn"
kind: idea
issues: [283]
components: [land, sync, orchestrator, gate, init, config, plugin, plugin.hooks.guard, host]
---

CLM assumes the trunk is a branch named `main` everywhere: the land target, the gate's base, rebase's base, the claim and next reads, `pr open`, the push guard and the skills' worktrees. In a repository whose trunk is `master` (git's default without `init.defaultBranch`), `clm init` succeeds and tells the human to push to `main`, and nothing says the branch is missing until the first `clm land` refuses with "no `main` to land on". No setting names another branch, so adopting CLM means renaming the trunk first, which breaks other clones, CI triggers and branch protection keyed to the old name. Reported in GitHub issue #283.

Direction settled at intake: the project defines its own trunk in `clm.toml`, a committed setting defaulting to `main` that `clm init` fills from the current branch, rather than detecting it from `origin/HEAD`. A committed setting is the same in every clone, CI run and draft, while `origin/HEAD` is absent with no remote (`host = "none"`) and may be unset or differ per clone. Every command and skill reads it in place of `main`, and the push guard and the `clm` commands that refuse to push `main` protect that branch instead, so `land-locally` and `guard-blocks-push-to-main` still hold; both, and `no-local-land-with-remote`, name `main` in their wording and would be reworded to name the configured trunk. Until then, `clm init` should say up front when the repository has no `main`.
