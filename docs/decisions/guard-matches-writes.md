---
title: "The spec-approved guard matches label writes per command segment"
date: 2026-10-21
status: accepted
---

The first guard blocked any command holding `spec-approved` next to a word such as `-l`, `--label` or `/labels`. Those words appear in reads too (`gh pr list --label`, `grep -l`, a GET on `/labels`), so checking whether a PR is approved was refused, and Claude learned to work around the guard.

The guard now splits a command into segments and acts on a segment only when it writes: `gh pr|issue edit|create` with a label flag, or a `gh api` call that sends fields or uses a non-GET method, mentioning `spec-approved`. Since `approve-in-session`, it answers such a segment with Claude Code's `ask` decision, so the human confirms it, rather than blocking it. Removing `spec-approved` passes, since that only withdraws approval.

A heredoc's body belongs to the segment that opened it, not to segments of its own, so text written with `cat > f <<'EOF'` isn't mistaken for commands, and a body sent with `gh api ... --input -` is judged with its `gh` call. A body fed to a shell (`bash <<'EOF'`) is still judged as commands, since the shell runs it.

GraphQL's `addLabelsToLabelable` takes label node IDs, so the word `spec-approved` needn't appear. The guard asks about that mutation for any label. Adding labels through GraphQL isn't part of any CLM flow, and `gh pr edit --add-label` covers the legitimate cases.

This stays a text heuristic, not a shell parser (`hook-logic-in-clm`): a deliberately obfuscated command can still get through. The guard catches accidents; `humans-review-intent` rests on the human's approval.
