---
title: "Branches land through clm land on the machine that runs it; no hosted CI runs checks or merges"
date: 2026-10-26
status: accepted
---

`clm land <branch>` does the whole landing: it gates the branch against `origin/main`, merges it in a scratch tree, runs the project's check command and `clm check` on the result, records what it did in the ledger (`per-writer-ledgers`), and pushes the merge commit to `main` without force, retrying when `main` moved. No hosted CI runs a check or merges a branch. The checks run on the machine that runs the command, which is the session's machine for one person and whatever a team points at it. Lands waiting on one clone may share one run of the checks, on the tree that holds them all, so every commit a push leaves `main` at is a checked tree, though not every commit behind it (`batched-lands`).

Why:
- The decisions were already in `clm`; hosted CI only ran them and stored two labels. Metered minutes on a private repository were paying for a runner CLM didn't need, and the dependency on Actions kept GitHub as the decision host, which `host-adapters` ends.
- One lander per clone serialises the pushes to `main`, where concurrent merge jobs raced and paid for the race in reruns, and the `checked` record keyed on the code key gives a doc-only land its saving without a cache to keep warm.
- `main` has one copy, the remote's. `clm land` lands through `origin` on every host and refuses when `origin` can't be reached, landing nothing locally (`no-local-land-with-remote`). With `host = "none"` the clone keeps a remote of its own, a bare repository in its common git directory, so the same flow runs with no server (`none-keeps-its-own-remote`).

The gate stays a pure function of two trees and one flag: `clm land` computes `--approved` from the ledger and the `approval` policy, so the gate reads no ledger and no host. `clm land` is the only thing that pushes `main`, apart from the release workflow's merge of its version bump (`host-ci-for-releases`); the guard blocks a session's `git push` there (`guard-blocks-push-to-main`), and `clm land`'s own gate is what refuses, deterministically, a branch that needs review and has no approval (`yes-no-review`, `approval-policy`). A protected `main` on a host is fine as long as the landing identity may push it.

**Doc-only paths.** A land's checks are keyed on the merged tree's *code key*, a hash of the tree with the doc-only paths left out (`ledger/doc-only`): every path under `specs/`, `changes/`, `backlog/` or `docs/`, the guide's pages under `docs/guide/` and `docs/spec-format.md` included (`guide-pages-are-doc-only`), and `ARCHITECTURE.md`, `CAPABILITIES.md`, `README.md`, `CLAUDE.md` and `AGENTS.md`. The list names what is safe to skip, not what is code: a path not on it counts as code and gets every check, so a new directory fails safe. Two trees that differ only in doc-only paths have one code key, so a land whose merged tree's key already has a `checked` record, as a spec land on checked code does, runs only `clm check` and the tests whose `clm:reads` markers name a file it changed (`doc-readers-marked`). The fmt, lint, build and test steps can't fail on a diff that changes no code and that no test reads, so running them again would only cost time. A list of code paths was rejected: a new code directory would skip the checks until someone added it.

One gap is accepted: `web`'s build reads the invited logins from `specs/web/SPEC.md`'s Contract (`invite-only-web`), and the tests of that build hold no `clm:reads` marker for it, so a spec land that malforms the list isn't caught at that land. The next code land's `web` tests fail on it, and so does the release's deploy, loudly and before anything is served.

The release workflow stays on Actions for now (`host-ci-for-releases`).

Rejected: a self-hosted Actions runner (free minutes, no spec change, but the dependency and the decision host stay) and keeping Actions as a supported runner for checks (a team that wants a hosted runner runs `clm land` there; CLM generates no workflow for it).
