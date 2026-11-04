---
title: "Sessions report CLM's own defects as they happen; issues leave a project only with the human's approval"
date: 2026-10-22
status: accepted
---

A CLM defect is anything that CLM itself (its CLI, CI, gate rules, plugin, skills or method) would have to change so it doesn't happen again. A session's own mistake that CLM caught, or that it fixed, is CLM working and isn't one. Sessions report each defect with `clm defect` when it happens, and `propose` and `implement` check for missed ones before every PR, listing them under `## CLM defects`. The session-start hook states the rule, so it reaches every session in every project, not only those with a CLAUDE.md that mentions it.

Where a report goes is the `defects` setting (`layered-settings`):
- `backlog`, in CLM's own repository: a `kind: bug` item, deleted as usual when a change resolves it. Once fixed, the lesson lives in the spec, so no permanent log is kept.
- `ask`, the default elsewhere: an issue on `clunny/clm`. Every issue needs the human's approval, which the guard enforces by answering `clm defect` with Claude Code's `ask` permission decision, so the human sees the full report and can approve, reject or ask for changes. Unattended sessions can't be asked, so they list defects in the PR body for the human to file.
- `off`: nothing.

A report describes only the process: the CLM step or command, what happened, what was expected, the rule or decision involved. It holds nothing of the project (file names, code, spec text, output), because other projects may be private client work. Keeping content out is the session's judgment, and the human's approval is the check (`deterministic-enforcement`: judgment advises, the permission prompt gates).

In CLM's own repository (`backlog`), a session fixes a small defect in the moment rather than leaving it filed (`20261028-fix-defects-in-the-moment`). A defect is small when its fix needs no spec change (nothing under `specs/`, `changes/`, `docs/`, `ARCHITECTURE.md` or `CLAUDE.md`) and doesn't noticeably grow the session's work. On an `impl/*` branch whose nodes hold the fix, it is folded in and not reported (`fold-small-defer-large`); otherwise it is reported and, once the session's branch has landed and brought the item to `main`, fixed before the session ends on its own code-fix branch with `clm claim <node> --fix backlog/<slug>` (`code-fix-resolves-bug`), so its item lives only until that fix lands. Only a defect that is a large amount of work or needs a spec change is left in the backlog, and an unattended session folds what fits and reports the rest. Filing everything had filled the backlog with small bugs, each costing a later session a claim, a branch and a land for a fix the reporting session could have made with the context already in hand. Folding alone was rejected because most defects lie in a node other than the one being worked on; a side branch for every defect, even one the current branch could hold, adds branches and lands for no gain; and starting the side fix the moment the defect is met would need a claim with no item on `main`, since `clm claim --fix` reads the item there, relaxing `code-fix-resolves-bug`'s rule that only a named bug opens a converged node. Adopters (`ask`) don't hold CLM's code, so their rule is unchanged.

Issues are inbound reports, not CLM state: a human triages them into `backlog/` (`git-is-truth`). Rejected alternatives: instructions alone (untestable, and they never reach other projects' ad-hoc sessions), and a `Stop` hook asking after every turn (noisy, invites filler reports).
