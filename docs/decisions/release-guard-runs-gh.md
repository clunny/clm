---
title: "The release guard runs gh itself, outside the host adapter"
date: 2026-10-26
status: accepted
---

Refines `host-adapters`. Its rule is that a caller never runs `gh` or `az` itself: pages, remote branch deletion and issues go through the `host` node, picked by the `host` setting. One caller is different in kind. The guard in `clm-hooks` judges a `gh run rerun` that the session itself is about to run (`plugin.hooks/release-guard`, `release-run-lookup`), and looks the run up with `gh api` on the repository and host that command names (its `-R`, `GH_REPO=` and `GH_HOST=`), in the checkout it runs in. The thing being guarded is a `gh` call against GitHub Actions, so the lookup is `gh` whatever the `host` setting says: a repository whose `host` is `none` can still have a session that runs `gh run rerun`, and the guard must still answer for it.

So `host/git-through-layer` names `clm-hooks` as an exception, limited to that one lookup: the scan allows exactly one `run` call with `gh` as its program in `clm-hooks`, and no `clm_git::gh` or `az` there. Every other crate outside `clm-git` and `clm-host` (and `clm-watch`, until `20261027-remove-watch-node` removed it) holds no `"gh"` or `"az"` literal at all, which catches `clm_git::run(.., "gh", ..)` as well as `clm_git::gh`.

`clm-web`'s `gh` module is an HTTP client for GitHub's API in a Worker; it runs no process. The rule is about the `gh` and `az` CLIs, so it doesn't count. `web` reaching GitHub through its App is already part of `host-adapters`.

Alternatives considered:
- A `host` operation that looks up a workflow run. It would tie the guard to the `host` setting, which says where CLM's own pages and issues live, not where the command being guarded points; on `git` and `none` the adapter would answer nothing and every re-run would count as a failed lookup and be blocked. It would also put a CI concept into an adapter whose hosts "run no checks". The release workflow is the one job left on Actions (`host-ci-for-releases`); if it leaves, the guard's re-run rule goes with it.
- Narrowing the rule to `clm_git::gh` only, allowing `run(.., "gh", ..)` anywhere. That leaves the sentence the bug was about unenforced: any crate could run `gh` through `run` and no test would notice.
