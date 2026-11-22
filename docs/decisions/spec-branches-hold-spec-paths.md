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

**Amended 2026-11-22 (`20261121-wave-3-land-by-the-rule`).** Once that change's builds have landed, `clm land` judges a branch named `spec/<id>` by the kind the gate gives its head, computed in-process before the lock, and no longer by its files alone: it lands only when the kind is `spec` (review or not), and refuses `code`, `mixed` and `light`, naming each changed file that isn't a spec path. Two things made the files-only reading wrong. A backlog-only branch now reads `spec`, as does any branch with no code file, so the kind and the files no longer disagree for the branches this rule was written for. And a `light` branch, scenarios added together with the code that proves them, lands on checks with no review; an outside contribution is landed under a `spec/<id>` name, so that name must keep code off `main` without a human's review, which only the kind can say. No rule rests on the name alone: it is a claim the gate's reading must agree with.
