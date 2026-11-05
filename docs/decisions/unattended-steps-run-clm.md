---
title: "Unattended steps that write to origin run only clm commands, under allow rules clm init seeds"
date: 2026-11-05
status: accepted
---

Builds and drafts run as background subagents under the human's permission mode (`builds-in-subagents`). In Claude Code's auto mode a classifier judges each command no allow rule covers, and it denied two steps the prompts require: a build's `git push --force-with-lease` after `clm rebase` rewrote its branch (issues #109, #154), and a draft's `clm land spec/<id>` of its own change (#126). Releasing a claim by deleting its branch on `origin` was denied the same way. The human had to run each step by hand. The human chose this design in triage on 2026-11-05.

**Every step that writes to `origin` is a `clm` command.** A build or draft pushes its branch with `clm branch push` and deletes a branch with `clm branch delete`, beside `clm claim` and `clm land`. `clm branch push` pushes only the checked-out branch, leasing on the tip the clone last fetched and, with git's `--force-if-includes`, only when the branch has held that tip, so a session overwrites only its own work (`lease-claims`). `clm branch delete` deletes only a tip this clone pushed, read from the remote-tracking ref's reflog, so a session can release its own claim but never another session's, which would bypass the claim's expiry and takeover. Both refuse `main` and the ledgers' `clm/` branches themselves, since the guard sees `clm branch ...`, never the `git push` inside it (`guard-blocks-push-to-main`).

**`clm init` seeds narrow allow rules for them.** A project allow rule resolves before the auto-mode classifier, and subagents, background and worktree ones included, run under the main session's rules; auto mode drops broad rules such as `Bash(*)`, so each rule names one `clm` subcommand: `Bash(clm <subcommand>)` and `Bash(clm <subcommand> *)` for `apply`, `branch`, `check`, `claim`, `defect`, `gate`, `land`, `pr`, `rebase` and `workaround`, the `clm` commands the prompts run that write. `clm approve` is never among them, and the guard's `ask` for it holds over any allow rule (`approval-policy`). Allowing `clm land` is safe because it is the gate: it refuses a `spec review` branch without the human's approval.

**Repositories set up earlier.** `clm init` changes no existing file on a rerun (`init-rerun`), so it adds the rules only on its first run, and a rerun names the missing ones in its next steps. The `build` and `triage` skills run `clm init --check-permissions` before their first launch and tell the human what to add, never writing `.claude/settings.json` themselves: it is a protected path, and the human's settings.

**Alternatives.**
- *An allow rule for `git push --force-with-lease`.* Rejected: it would let a session force-push any branch, another session's claim or spec branch included.
- *The draft hands its land to the triage session.* Rejected: the main session meets the same classifier, so the step only moves.
- *A push built into `clm rebase` or `clm land`.* `clm rebase` is also run by hand where no push is wanted, and `clm land` never pushes the branch; a push of its own, beside `clm branch delete` on the host adapter's `branch` command, keeps each command doing one thing.
- *`clm init` rewriting the settings on every run, as it does `.gitignore`.* Rejected by the human's design: a rerun leaves project files alone, and a rule a project removed on purpose would come back.

**Unverified premise.** That a worktree-isolated background subagent honours the project's allow rule is implied by Claude Code's docs, not stated. A human checks it by experiment (`backlog/check-subagent-honours-allow-rule`) before the rules are seeded.

This amends `builds-in-subagents`, which says `clm` ships no allowlist: `clm run` still sets no mode and no allowlist of its own, and the rules `clm init` seeds are the project's, for its own sessions.
