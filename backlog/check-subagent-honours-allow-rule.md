---
title: "Confirm in notespeed that an auto-mode build pushes and lands through the seeded clm rules"
kind: human-task
components: [init, plugin]
origin: 20261105-unattended-steps-denied-in
---

`20261105-unattended-steps-denied-in` rests on Claude Code's docs: a narrow allow rule in the project's settings resolves before the auto-mode classifier, and a subagent's tool calls are judged with the main session's rules. A test on 2026-11-05 couldn't settle it before the change is built (see `docs/decisions/unattended-steps-run-clm.md`): the classifier refused the probe subagent's *launch*, whose task spelled out the force-push, with the Bash rule loaded, so the push itself was never reached. Only a built change, whose prompts name only `clm` commands, can be tested, so this item blocks nothing.

Once `host/branch-push`, `init/init-seeds-permissions` and the prompts' move to `clm branch push` are built and released:

1. In notespeed, add the rules `clm init --check-permissions` names to `.claude/settings.json` and commit them.
2. Start Claude Code in a worktree session (`claude -w <name>`) in auto mode and run `/clm:build` while another component lands on `main`, so a build has to `clm rebase` and push its rebased claim branch.
3. Confirm the build pushes with `clm branch push` and lands, with no classifier denial; and that a triage draft whose gate prints `spec` lands itself.

When both run, delete this item. When either is denied, report it with `clm defect`, quoting the denial, and replace this item with a `question` for a redesign.
