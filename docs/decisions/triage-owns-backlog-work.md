---
title: "Triage works every backlog item but ideas; propose suggests only ideas"
date: 2026-11-01
status: accepted
---

`clm triage` used to give `deferred` and `workaround` items no line, so the only place they surfaced was `/clm:propose`'s suggestions when the human named no intent, beside the ideas. That mixed two jobs: `propose` turns a new intent or an idea into a design, while `triage` works the backlog down. A workaround flag or a deferred item offered as a fresh intent read as new work, and the human had to sort them by hand.

So `clm triage` prints a `deferred` line for each untaken deferred item and a `workaround` line for each untaken workaround flag, and the `triage` skill offers them as two menu choices, *Deferred* and *Workaround*. Neither is drafted on its own. Each is shown to the human one at a time with a recommendation, and the human has it drafted, kept or dropped; a drafted or dropped item goes to `clm draft --answers`, the path an answered `question` takes, so the human's call is quoted in the change. This keeps `workaround-flags` (a flag is evidence, counted by `clm health`, never drafted as work by itself) and `deferred-work-resolves-in-code` (only the human judges a deferred item's condition): the human's answer is what drafts it.

`/clm:propose` with no intent suggests only `idea` items and points the human at `/clm:triage` for the rest. An item of any kind the human names is still taken, since turning it away would only send them elsewhere.

Bugs lead the list: `group` lines sort first, the changes' and the human's lines after them, then ideas, `deferred` next to last and `workaround` last, in `clm triage`'s output and the menu alike. The menu still asks the *Approvals* before any draft is launched. `/clm:build` drafts the bug groups too, as part of its drain; every other kind stays `triage`'s (`build-drafts-bugs`).

Refines `workaround-flags` and `deferred-work-resolves-in-code`.

Rejected:
- **Group workaround and deferred items with the bugs and draft them unasked.** A flag is evidence, not a defect, and a deferred item's condition is the human's to judge; a draft can't ask.
- **Keep them under `propose`.** `propose` would stay a second, partial triage.
