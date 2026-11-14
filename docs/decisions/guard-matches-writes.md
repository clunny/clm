---
title: "The guard judges a command per segment, and acts only on a segment that writes"
date: 2026-10-21
status: accepted
---

The first guard blocked any command holding `spec-approved` next to a word such as `-l`, `--label` or `/labels`. Those words appear in reads too (`gh pr list --label`, `grep -l`, a GET on `/labels`), so checking whether a PR was approved was refused, and Claude learned to work around the guard.

So the guard splits a command into segments and acts on a segment only when it writes: the segment's own command, with its own arguments, is what a rule judges, never a word that appears somewhere in the line. A segment that records a human's decision, such as `clm approve`, is answered with Claude Code's `ask` decision, so the human confirms it, rather than blocked (`approval-policy`).

The rule this was first written for, writes of the `spec-approved` label, went with the labels: approvals are ledger records (`per-writer-ledgers`), and a `gh` call that adds or removes a label does nothing CLM reads, so the guard passes it. The segment reading stayed, and the later rules build on it (`guard-blocks-push-to-main`, `guard-reads-quotes-and-wrappers`, `guard-shell-forms`).

A heredoc's body belongs to the segment that opened it, not to segments of its own, so text written with `cat > f <<'EOF'` isn't mistaken for commands, and a body sent with `gh api ... --input -` is judged with its `gh` call. A body fed to a shell (`bash <<'EOF'`) is still judged as commands, since the shell runs it.

This stays a text heuristic, not a shell parser (`hook-logic-in-clm`): a deliberately obfuscated command can still get through. The guard catches accidents; `humans-review-intent` rests on the human's approval.
