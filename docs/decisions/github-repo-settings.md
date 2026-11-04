---
title: "The settings of the clunny/clm repository on GitHub"
date: 2026-11-03
status: accepted
---

GitHub's settings aren't in git, so this is their record. The proposing session of `20261103-outside-contributions` applies them with `gh api` once that change lands, on the human's approval of the change. They apply to this repository only.

| Area | Setting | Why |
|---|---|---|
| Profile | A description, and the topics `claude-code`, `spec-driven-development`, `ai-coding`, `developer-tools` and `rust` | So people can find it and tell what it is |
| Features | Wiki and Projects off, Issues on, Discussions off | Documents and plans live in git (`git-is-truth`), and issues are enough for one maintainer |
| Merging | Merge commits only, with squash and rebase off; head branches deleted on merge | GitHub requires one method. If someone uses the button by mistake, this keeps authorship |
| `main` | A ruleset blocking force-pushes and deletion | `clm land` and the release push `main` as fast-forwards, so neither is affected |
| Security | Private vulnerability reporting and Dependabot alerts on, Dependabot security pull requests off | Its pull requests would be code branches, which `ai-writes-code` leaves to builds, so an alert goes through the backlog |
| Actions | Approval required before workflows run for all outside contributors | No workflow runs on a pull request today; this keeps a new one from running a stranger's code unseen |
| Labels | Add `accepted` | The maintainer's yes to an idea |

Rejected:
- **A ruleset requiring pull requests or restricting updates on `main`.** It would block `clm land` and the release's merge. GitHub's docs limit bypass lists to organizations for branch protection, and users report that "restrict updates" blocks admins even when listed. So the merge button stays available; the guide says not to use it and `clm ledger verify` reports it afterwards.
- **A committed script applying these.** It's more to keep up for one repository whose settings rarely change, and no test could prove it without the network (`yagni`).
