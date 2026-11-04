---
title: "CI merges main into a behind PR and lands the tree it checked"
date: 2026-10-22
status: superseded
superseded_by: land-locally
---

Refines `gate-without-branch-protection`, which required branches to be up to date with `main`. Merges by the CI bot don't trigger CI on `main`, so being up to date was what made the tree that lands the tree that was tested. But it failed every open PR whenever anything merged, even when the PRs shared no files. Each then needed a rebase, a push and a full CI run.

Now CI's `merge` job lands every passing PR itself, never through `gh pr merge`. It re-runs the gate on the head against the current `main`, merges the head into it with GitHub's merge message, and runs the `check` job's checks and `clm gate` on the merge commit unless its tree is the head's own. Then it pushes it to `main` without force. If `main` moved in the meantime, the push is refused, and it starts again from a fresh fetch, up to ten times (`merge-by-push-race`). Every tree on `main` is still one that was checked. The gate now blocks only what that merge can't settle: a textual conflict, or a file under `specs/` changed on both sides, which `clm sync` regenerates rather than merges (`regenerate-dont-merge`).

Rejected:
- **Waiving the rule when the approval fingerprint is unchanged.** The fingerprint shows the PR's own diff is the same, not that its merge with the new `main` was tested.
- **Having CI rebase behind PRs.** Workflow-token pushes start no workflows, so this needs a PAT or an App token. It also costs a full CI run per PR per merge, and rewrites authors' branches.

If the repo gains branch protection with GitHub's merge queue, the queue does the same job natively, and this can be retired.
