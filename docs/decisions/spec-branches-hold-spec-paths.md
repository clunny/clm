---
title: "clm land refuses a spec branch holding files outside the spec paths"
date: 2026-11-03
status: accepted
---

`clm land` accepts `code` branches, since builds are code branches, and blocks only `mixed`. So a fork's code-only branch pushed to `origin` would have had its code run and landed. Accepted contributions always arrive as `spec/<id>` (`outside-contributions-are-intent`), so `clm land` now refuses a `spec/*` branch with a file outside the spec paths, before the lock, the merge or any check.

**It reads the files, not the gate's first line.** The gate prints `code` for a branch that only adds a backlog item (`gate/backlog-additions`), and such a branch is a legitimate suggestion, so a rule on the first line would refuse it. A `clm rename` branch rewrites test markers but follows wholly from its recorded pairs, so it's exempt, as `gate/classify` treats it.

**It lives in `clm land`, not the gate.** The gate is a pure function of two trees and one flag (`land-locally`) and reads no branch name. `clm-gate` supplies the list of files; `clm land`, which knows the branch, applies the rule.

It applies in every project. A `spec/*` branch shouldn't hold code anyway, so it should only stop branches that are already wrong.

It can't catch a branch pushed under another name, so `/clm:contribution` always uses `spec/<id>`.

Rejected:
- **Recording where a branch came from.** Git doesn't record a fork, so a land would have to ask the host, and no gate or land reads the host (`git-is-truth`).
- **Commit signatures from listed people on code branches.** It would stop outside code under any branch name, but needs every project to sign and adopt `docs/allowed_signers`, which is a much larger decision.
