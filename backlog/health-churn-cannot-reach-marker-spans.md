---
title: "check.health can't call marker_spans, which churn-signal now reads markers with"
kind: question
components: [check.health, check.coverage, check]
origin: 20261114-proof-is-real
blocks: [check.health/churn-signal]
choices:
  - "Add check.coverage to depends_on"
  - "Re-export it from check"
---

`check.health/churn-signal` now counts the markers `marker_spans` finds, whether or not they sit in a test. `marker_spans` lives in `clm-coverage` (`check.coverage/marker-spans`). `check.health` depends on `check`, not `check.coverage`, and `clm-check` re-exports no `marker_spans`, so `crates/clm-health` can't call it without an import its `depends_on` doesn't allow (`undeclared-imports`).

**The question for the human:** which edit lets it reach `marker_spans`?

- Add `check.coverage` to `check.health`'s `depends_on`, as `rename` does.
- Have `check` re-export `marker_spans` beside `markers_in`, which its Design lists.

Either is a spec change (a `depends_on` entry, or a line of `check`'s Design and a requirement carrying it); `churn-signal` stays unbuilt until it lands, and its tests keep citing the old version, so the component stays `behind`.

Recommended: the re-export. `check`'s Design already has dependents call `clm-coverage`'s library through `clm-check` (`check-coverage-split`), so it adds no second import path. Only `churn-signal` waits; the component's other requirements are current.
