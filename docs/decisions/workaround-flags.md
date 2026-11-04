---
title: "Build sessions flag a workaround against a requirement as a backlog item the model keeps honest"
date: 2026-11-02
status: accepted
---

This restates `friction-flags`, with its amendments, in the words `plain-terms-renames` chose: the command `clm workaround`, the backlog kind `workaround` and the files `backlog/workaround-<node>-<slug>-<n>.md` (`20261102-rename-states-and-keys`). The rule is unchanged.

A build that works around a scenario, fakes a dependency to test a requirement, changes another node's Design to make one buildable, or stops to raise a spec change, has learned something about that requirement that nothing recorded once its branch landed. So the session flags it with `clm workaround <node>/<slug> --title <title>`, the situation on stdin, as it reports a CLM defect with `clm defect` (`defect-reports`). The flag is a backlog item of kind `workaround` naming the requirement at its current version (`requirement: <node>/<slug>@<version>`). Adding a backlog item never makes a code branch mixed (`gate/backlog-additions`), so a build files it on its own `impl/*` branch and commits it with its work. A flag is evidence, never a block: the session still builds the requirement, and nothing the queue, the gate or `clm check` reads changes.

The triggers are a fixed list in the build prompt (`orchestrator.run/prompt-workaround`), so flagging isn't left to mood: a special case or workaround to satisfy a scenario; a test that could only be written by faking something the requirement doesn't mention; a Design change in another node to make it buildable; a stop to raise a spec change, or a scenario the spec-auditor found the code can't observe.

The model rejects a `workaround` item whose requirement is gone or whose version isn't current, as it rejects a `blocks` entry naming a requirement that doesn't exist. So the spec change that drops or rewrites a requirement deletes its flags, or re-records them at the new version when the workaround still applies, and `clm check` tells it to. A pending change's resulting graph doesn't judge flags (`apply/resulting-graph`): the duty falls on the branch that applies the change, where loading reports it once `clm apply` has run. A rewritten requirement starts with a clean score, and history stays in git.

`clm health` lists the flags and counts them into the requirement's score; `clm rebase` files the reconsider question at the threshold (`health-advises`). `clm triage` prints a `workaround` line per untaken flag, last in its output, and the `triage` skill shows each to the human, who has it drafted, kept or dropped (`triage-owns-backlog-work`); a flag is never drafted unless the human asks.

Rejected, as in `friction-flags`: a ledger record (nothing reads it for this, and a build has no writer there); a `bug` item with a title convention (`clm triage` would draft it, and the bug count would stop meaning bugs); counting commits or build attempts (nothing records which requirement a commit was for); and the session judging whether to file a question itself (the threshold is the program's, `deterministic-enforcement`).
