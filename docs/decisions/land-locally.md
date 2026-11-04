---
title: "Branches land through clm land on the machine that runs it; no hosted CI runs checks or merges"
date: 2026-10-26
status: accepted
---

`clm land <branch>` does what CI's `check`, `gate` and `merge` jobs, `clm ci merge`, `clm ci pin-approval` and `clm watch` did: gate the branch against `origin/main`, merge it in a scratch tree, run the project's check command and `clm check` on the result, record what it did in the ledger (`per-writer-ledgers`), and push the merge commit to `main` without force, retrying when `main` moved. Every tree on `main` was still checked before it got there; what changed is who ran the checks: the machine that runs the command, which is the session's machine for one person and whatever a team points at it.

Why:
- The decisions were already in `clm`; Actions only ran them and stored two labels. Metered minutes on a private repository were paying for a runner CLM didn't need, and the dependency on Actions kept GitHub as the decision host, which `host-adapters` ends.
- One lander serialises the push race that `merge-by-push-race` paid for in reruns, and the `checked` record keyed on the code key keeps the doc-only saving of `light-ci-for-doc-prs` without a cache to keep warm.
- Offline works: with no reachable `origin`, `clm land` lands on the local `main` and `clm reconnect` fast-forwards the host later.

The gate stays a pure function of two trees and one flag: `clm land` computes `--approved` from the ledger and the `approval` policy, so the gate reads no ledger and no host. `clm land` is the only thing that pushes `main`; the guard still blocks `git push` there, and `clm land`'s own gate is what refuses an unapproved HARD branch, deterministically. A protected `main` on a host is fine as long as the landing identity may push it.

Superseded with this: `gate-without-branch-protection`, `protected-main`, `merge-tested-tree`, `merge-by-push-race`, `ci-logic-in-clm`, `light-ci-for-doc-prs`, `prebuilt-clm-for-doc-prs`, `warm-ci-cache`, `watch-and-resume`, `watch-expects-head`, `watch-reruns-cancelled`, `watch-rerun-grace` and `flaky-failures-rerun`. The release workflow stays on Actions for now (`host-ci-for-releases`).

Rejected: a self-hosted Actions runner (free minutes, no spec change, but the dependency and the decision host stay) and keeping Actions as a supported runner for checks (a team that wants a hosted runner runs `clm land` there; CLM generates no workflow for it).
