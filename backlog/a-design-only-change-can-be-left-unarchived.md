---
title: "A design-only change can be left unarchived, and a branch named impl/archive reads as a claim"
kind: idea
components: [check, orchestrator]
---

Two outcomes that only skill text guards against, since the requirements pinning that text (`plugin.propose/propose-archives-design-only`, `plugin.triage/triage-archives-complete`) were scans of prompt text and are deleted in `20261113-phase-2-delete`.

- A change whose deltas add, modify and remove no requirement has nothing to build, so no build archives it; its spec branch must archive it (`archive-in-spec-pr`). `clm check` could refuse such a change left `applied`.
- An archive branch is `impl/archive/<id>`. A branch named exactly `impl/archive` would read as a claim on a component `archive`. The queue could ignore, or `clm branch push` refuse, an `impl/*` branch that names no component.
