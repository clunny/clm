---
title: "A host's issues enter the work only as backlog items, through clm intake and the human's yes"
date: 2026-11-04
status: accepted
---

`/clm:triage` takes the host's inbox in before it works the backlog. `clm intake` matches each open issue to the backlog from git: `new` when no item names it, `imported` when an item's `issues` key does, and `resolved` when the item that named it was deleted on `main`, by the change whose `resolves` or `from` names it or by the commit that deleted it. A verifier judges the new issues, the human says yes to each, and the item lands on a code branch, which a branch that only adds items is. From there the issue is ordinary backlog work: `clm triage` groups it, and a draft designs it with the builder check and architecture audit every draft has.

Why a backlog item and not a draft straight from the issue: the issue isn't in git (`git-is-truth`), claims, groups and `resolves` are all keyed on items, and the item's `issues` key is the record that lets `clm intake` tell, later, that the issue is fixed. Before this, issues taken in by hand stayed open after their fix landed, and nothing found duplicates.

Why a command and not the skill: which issue is new, taken in or fixed is a lookup, and a program makes it (`deterministic-enforcement`). `clm triage` stays a reader of git alone; `clm intake` is the one triage command that reads the host, through `host` (`host-adapters`).

Every issue is treated alike, whoever opened it, the maintainer's own `clm defect` reports included (`untrusted-text-read-only-verifier`).

Rejected:
- **The skill matching issues through a `clm contribution list`.** A model would decide which issues are new or closable, as the open-but-fixed issues show it can't be trusted to.
- **Drafting changes straight from issues.** Above.
- **Closing an issue when the land that resolves its item runs.** It puts a host write into `clm land`, which writes nothing to the host today, and closes a stranger's issue with no human's yes. `clm intake`'s `resolved` line lets triage offer the closing instead.
- **Trusting the maintainer's own issues and importing them unasked.** The text still came through the host, and the human chose to see every one.
