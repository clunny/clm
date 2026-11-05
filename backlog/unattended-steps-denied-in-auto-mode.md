---
title: "Builds and drafts stop under Claude Code's auto mode at their push and land steps"
kind: bug
components: [orchestrator.run, orchestrator.triage, plugin, init, land]
issues: [109, 154, 126]
---

The build and triage skills tell the human that auto or bypass mode lets builds and drafts run alone, but under auto mode Claude Code's classifier denies two of their required steps:

- **Builds (#109, #154):** after `clm rebase` rewrites a build's `impl/<component>` branch onto a newer `main`, the build prompt requires `git push --force-with-lease` before `clm pr open` and `clm land`. The classifier denies it as destructive, and the build stops with a checked branch it can't land. Releasing an orphaned claim by deleting its remote branch is denied the same way.
- **Drafts (#126):** when `clm gate` prints `spec`, the `clm draft` prompt has the draft land its own change with `clm land spec/<id>`. The classifier denies it as a merge without review, and the human lands it by hand.

In both cases the human has to run the step themselves.

**The design the human chose (2026-11-05, triage):** unattended steps run only `clm` commands, and `clm init` seeds narrow allow rules for them.

- A narrow allow rule in the project's `.claude/settings.json` resolves before the auto-mode classifier, and subagents, background and worktree ones included, run under the main session's rules (Claude Code's permission-modes and auto-mode docs). Broad rules such as `Bash(*)` are dropped in auto mode, so the rules must be narrow.
- The build must never need a raw force-push. A `clm` command pushes a rebased claim branch itself, as `clm land` or a push subcommand, keeping the claim's compare-and-swap (`lease-claims`) and refusing `main` (`guard-blocks-push-to-main`). Seeding an allow rule for `git push --force-with-lease` would let a session force-push any branch, another session's claim or spec branch included, so it's rejected.
- Allowing `clm land` is safe because `clm land` is itself the gate: it refuses a `spec review` branch without approval. Having the draft hand its change to the triage session to land was rejected: the main session meets the same classifier, so the step only moves.
- `clm init` writes the narrow allow rules for the `clm` commands the unattended prompts run into `.claude/settings.json`. `clm approve` is never among them, and the guard's `ask` decision still holds over any allow rule. Repositories set up earlier don't get the file rewritten (`init-rerun`), so the build and triage skills check for the rules, deterministically through a `clm` command, before their first launch, and tell the human what to add when they're missing.
- First check by experiment that a worktree-isolated background subagent honours the project's allow rule; the docs imply it but don't state it.
