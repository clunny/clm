---
title: "Every change says what is significant, and a rubric checked by the gate makes sure nothing flagged is left out"
date: 2026-10-22
status: superseded
superseded_by: yes-no-review
---

A reviewer approves intent, not every line. So each `change.md` starts with `## Intent`, a few sentences, and `## Significant`, one bullet per significant item with why it matters. This is what the session shows the human for approval, what the web UI shows first, and what the PR body repeats. It lives in `change.md`, not only in the PR body, so it is in git, covered by the approval fingerprint (editing it drops an approval) and checkable locally.

The rubric is mechanical, computed by `clm significant` from the diff:
- `ARCHITECTURE.md` and `CLAUDE.md`;
- nodes added or removed, and `depends_on` changes;
- Contract changes;
- modified or removed requirements;
- every change in a node marked `significant: true`, for trust-boundary nodes such as `gate` and `ci`;
- added, modified or deleted decisions.

The gate (`gate/significant`) blocks a PR until each flagged item is cited in a `## Significant` section. It checks that each is cited, not what is said about it.

The author adds what the rubric can't see: anything outward-facing or hard to undo, new work or steps for the human, ordering risks, and what is deferred.

Rejected:
- **A generated inventory of every change in the PR body.** It duplicates the diff.
- **Prose alone.** A summary Claude writes could leave out the one thing that matters, which would amount to Claude approving its own change.
