---
title: "Build prompt asks to re-cite markers on tests whose scenario moved to another requirement"
kind: bug
components: [orchestrator.run]
---

The build prompt for check.unreferenced listed crates/clm/tests/unreferenced.rs:49 and :64 under "Re-cite each marker that cites it at another version", for check.unreferenced/report moving 27b2d762 -> 849ec044. The change 20261117-scenarios-only-a-process-shows moved exactly those two tests' scenarios out of report and into clm/root-from-cwd and clm/check-across-components. Re-citing them, as told, would credit report with scenarios it no longer states, leave a check.unreferenced marker in a root test file against clm/routes-only, and was flagged by both /code-review and the spec-auditor. The build took the markers off instead.
The prompt's re-cite list comes from the markers alone. When a test's scenario has moved to another requirement (the root's here), the prompt should say to take the marker off, or at least that "after checking that its test proves the current text" can end in removal, not only re-citing.
