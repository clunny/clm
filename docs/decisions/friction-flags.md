---
title: "Build sessions flag friction against a requirement as backlog items the model keeps honest"
date: 2026-10-28
status: superseded
superseded_by: workaround-flags
---

A build that works around a scenario, fakes a dependency to test a requirement, changes another node's Design to make one buildable, or stops to raise a spec change, has learned something about that requirement that nothing recorded once its branch landed. The next build paid the same price, and nobody was told to reconsider the requirement.

So a session flags friction with `clm friction <node>/<slug> --title <title>`, the situation on stdin, the way it reports a CLM defect with `clm defect` (`defect-reports`). The flag is a backlog item of a new kind, `friction`, naming the requirement at its current version (`requirement: <node>/<slug>@<version>`). Adding a backlog item never makes a code branch mixed (`gate/backlog-additions`), so a build files it on its own `impl/*` branch and commits it with its work. A flag is evidence, never a block: the session still builds the requirement, and nothing the queue, the gate or `clm check` reads changes.

The triggers are a fixed list in the build prompt (`orchestrator.run/prompt-friction`), so flagging isn't left to mood: a special case or workaround to satisfy a scenario; a test that could only be written by faking something the requirement doesn't mention; a Design change in another node to make it buildable; a stop to raise a spec change, or a scenario the spec-auditor found the code can't observe.

The model rejects a `friction` item whose requirement is gone or whose version isn't current, as it rejects a `blocks` entry naming a requirement that doesn't exist. So the spec change that drops or rewrites a requirement must delete its flags, or re-record them at the new version when the friction still applies, and `clm check` tells it to. A rewritten requirement starts with a clean score, and history stays in git.

`clm health` lists the flags and counts them into the requirement's score; `clm sync` files the reconsider question at the threshold (`health-advises`). `clm triage` groups only `bug`, `idea` and `deferred` items and lists `question` and `human` ones, so flags are never drafted as work.

Rejected:
- **A ledger record.** The ledger holds approvals, checks, lands and reviews; nothing reads it for this, and a build has no writer there.
- **A `bug` item with a title convention.** `clm triage` would group and draft it, and the bug count would stop meaning bugs.
- **Counting commits or build attempts instead.** Nothing records which requirement a commit was for, and a long build isn't a bad requirement. Rebuild churn from git history stays a backlog idea (`rebuild-churn-signal`).
- **The session judging whether to file a question itself.** Judgment advises; the threshold is the program's (`deterministic-enforcement`).

**Amended 2026-11-01** (`20261101-triage-owns-friction-deferred`). `clm triage` now prints a `friction` line per untaken flag, last in its output, and the `triage` skill shows each to the human, who has it drafted, kept or dropped (`triage-owns-backlog-work`). A flag is still never drafted unless the human asks.

**Amended 2026-11-01** (`20261101-friction-flag-rejected-against`). A pending change's resulting graph doesn't judge flags (`apply/resulting-graph`). A flag can hold only the version the current graph gives, so a build that flags a requirement a `proposed` change rewrites, such as the contract half of an `expand-contract` pair that waits on `main` for its moment, could otherwise keep the flag neither at the current version, which the resulting graph rejects, nor at the new one, which the current graph rejects, and had to drop the evidence. The duty to delete or re-record the flag falls on the branch that applies the change, where loading reports it once `clm apply` has run, as the `propose` skill already says.
