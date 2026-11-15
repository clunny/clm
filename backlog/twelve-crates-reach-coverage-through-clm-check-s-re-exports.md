---
title: "Twelve crates reach coverage through clm-check's re-exports"
kind: deferred
components: [clm, check.affected, check.health, orchestrator.checkpoint, effort, land, orchestrator, replica, orchestrator.run, sync, view, view.repo]
origin: 20261115-one-route-to-coverage
---

`check.coverage` was carved out of `check` on 2026-10-27, and `clm-check` re-exported its library so that no reader had to change (`check-coverage-split`). Measured on `main` at `4f96740b`, twelve crates still name a re-exported item as `clm_check::...`: `clm`, `clm-affected`, `clm-checkpoint`, `clm-effort`, `clm-health`, `clm-land`, `clm-orchestrator`, `clm-replica`, `clm-run`, `clm-sync`, `clm-view` and `clm-view-repo`. Five import `clm-coverage` itself, two of them (`clm-affected`, `clm-health`) by both routes. So a build of those twelve is given `check`'s Contract for calls that `check.coverage`'s Contract describes.

Each of the twelve components now depends on `check.coverage`. The work, one component at a time, each under its own `clm claim <component> --fix`, the last deleting this item: in the component's crate, add `clm-coverage` to the manifest where it is missing, and import from `clm_coverage` each of these that the code or its tests name through `clm_check`: `ChangeProgress`, `Coverage`, `DeletedTest`, `Marker`, `ObsoleteTest`, `Pending`, `Row`, `STATE_NAMES`, `State`, `StatusReport`, `TestSpan`, `Uncounted`, `change_progress`, `complete_changes`, `deleted_tests`, `markers_in`, `markers_outside_tests`, `matches` (also named `path_matches`), `obsolete_tests`, `owner`, `pending`, `removed`, `report`, `status_report`, `tests_in`, `uncounted`, `Scan` and `scan`. Drop the crate's `clm-check` dependency when nothing else uses it, and tell the human so the component's `depends_on` can follow in a spec change. `clm-check`'s own re-exports stay: `20261115-check-passes-on-no-coverage` removes them.

Due now. It changes nothing a requirement states.
