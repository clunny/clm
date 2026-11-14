---
title: "CLM's commands avoid words Claude Code's worktree isolation refuses: `clm hash` becomes `clm hashes`"
date: 2026-10-24
status: accepted
---

Claude Code can isolate a session in a git worktree, and then checks every Bash command it runs to show it stays inside that worktree. It refuses any command containing the word `hash`, which it reads as the shell builtin that runs a string: `clm hash plugin` and `cargo run -p clm -- hash plugin` alike. `propose` takes every delta base from that command and `implement` every marker version, so an isolated session could do neither except by writing a placeholder base and reading the real one out of `clm check`'s `apply/stale-base` message (`clm-hash-is-refused-in-worktree-isolated-claude-code-sessions`).

The check is Claude Code's, and CLM is built on Claude Code (`host-adapters`), so CLM adapts: the command is `clm hashes`, which still says what it prints, the bases a delta records and the versions a marker cites. `clm hash` was kept as a hidden alias while skills and docs moved (`expand-contract`), and `20261024-drop-hash-alias` removed it.

A new `clm` subcommand shouldn't be a word a shell treats as a builtin that runs or evaluates its arguments (`hash`, `eval`, `exec`, `command`, `builtin`, `source`), since the isolation check may refuse it the same way.

Rejected:
- **`clm base`.** Fits deltas, but not marker versions, which the same command prints.
- **Element hashes in `clm context`.** Puts bookkeeping in the agent's bundle (`spine-scoped-context`), and a reader of one version must read the whole bundle.
- **A permanent alias.** Cheaper, but the human chose to remove `hash` once nothing names it.
