---
title: "Flaky panel test fails a land removing its temp directory"
kind: bug
components: [plugin.panel]
---

A build run's first `clm land` failed at the check script's `capabilities/panel` step: the panel suite's `watch-machine-load` test failed with ENOTEMPTY while removing its temporary directory. The test lies outside the component being built. A retry of `clm land` with no changes passed, so the failure is intermittent.

Expected: the panel suite cleans up its temporary directories reliably, so a land fails only on a real defect (`land-locally`: `clm land` runs the check script on this machine).

The run did not report it itself, though `defect-reports` asks it to, since a check misfire is a CLM defect.
