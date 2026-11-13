---
status: applied
---

## Intent

Carve the pages of the web app out of `web` into a child, `web.views`, so the rethink in `20261112-web-on-the-move` builds its new pages in a component one build session can hold (`slim-components`, `nested-splits`). It changes no behaviour: the 28 requirements about what a page shows move word for word, and `web` keeps the shell (routing, loading and refresh, offline, the live socket's wiring, styles, deploy).

## Problem

`web` is over both budgets: 118,757 code tokens against 105,000 and 20,797 spec tokens against 16,000. Its Design held that splitting a view out would leave the child reaching into the shell's state, a cycle. The code no longer does that: every view module imports only leaf helpers (`dom`, `types`, `forms`, `markdown`, `time`, `diff`, `route`), and the one exception, `capabilities.ts` importing `SignedOut` from `api.ts`, goes away when `api.ts` moves with the views. `clm survey` over the cut (views and helpers to `web.views`, the rest to `web`) exits 0.

## Approaches

- **A. Grow `web` in place.** No split; the rethink adds an inbox, three detail pages and graph changes to a component already 13% over its code budget and 30% over its spec budget. Not chosen.
- **B. Carve the views into `web.views` (chosen).** The views and the helpers they share move under `web/src/views/`; `web` imports them, never the reverse. Each later build of either half holds a smaller spec and less code.

## Outcome

- `web.views` at `specs/web/views/SPEC.md`, `paths: ["web/src/views/**"]`, holding the moved requirements: `full-document`, `graph-view`, `implementation-view`, `activity-feed`, `backlog-view`, `blockers-view`, `queue-states`, `review-intent-first`, `spec-branch-review`, `approve-button`, `pr-document`, `backlog-taken`, `inspector-pending`, `signed-approvals`, `propose-from-backlog`, `comment-on-review`, `request-changes`, `capabilities-list`, `capability-page`, `decide-buttons`, `architecture-list`, `architecture-page`, `architecture-decide-buttons`, `says-architectural-constraint`, `says-component`, `oversized-component`, `says-new-state-words`, `zoom-at-pointer`. Its build moves the modules and `web/test/fixtures.ts` into `web/src/views/` and re-points the markers, `web`'s end-to-end tests' among them (`moved-markers-wait`).
- `web` keeps the shell's requirements, depends on `web.views`, and its budget drops to 12,000 spec and 80,000 code tokens.

## Constraints

- The move is word for word: a moved requirement's text doesn't change here. `20261112-web-on-the-move` modifies some of them afterwards.
- `web/src/styles.css` stays `web`'s.
