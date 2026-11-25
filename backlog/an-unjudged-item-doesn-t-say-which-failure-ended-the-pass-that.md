---
title: "An unjudged item doesn't say which failure ended the pass that left it"
kind: bug
components: [verify]
origin: 20261124-unjudged-check.coverage-88470edf
---

The pass after `88470edf` ended when `.github/scripts/mutate.sh` exited 1 (`mutate.sh: StrykerJS failed for requirement web/no-install-scripts` and four more `web` blocks), with `0 killed, 0 unproven, 0 not judged, 29 left`. `clm verify` then filed seven `unjudged-<component>-88470edf` items (`check.coverage`, `ci`, `model`, `web`, `web.views`, `web.worker`, `web.worker.replica`), each saying only that the pass "stopped, reached its limit or failed before its verdict" and pointing at "`clm verify`'s stderr for those passes", which `clm verify` keeps nowhere: the stderr of the pass after `bf0d9028` is gone, and that of `88470edf` survives only because a run was redirected by hand to `.clm/verify-run.out`. Triage then drafted six of the items in parallel, each draft hunting one cause that lies in `mutate.sh` (component `ci`) and in none of the other components the items name.

`clm verify` knows the failure when it files: it printed `mutation: .github/scripts/mutate.sh failed with status 1`, and the command's own stderr named the failing blocks. An unjudged item should carry why the pass ended (the command's exit and the lines of its stderr naming blocks, or `stopped`, or `mutate_minutes` reached), and a pass that a tool failure ended should file against the component whose block failed rather than every component whose blocks it never started; or the `mutated` record should keep that reason so the filing can name it. The fix needs a change to `verify/files-unproven` (and perhaps `verify/mutation-pass`), since no requirement states the reason in the body.
