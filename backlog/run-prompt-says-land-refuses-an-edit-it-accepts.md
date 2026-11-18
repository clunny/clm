---
title: "The run prompt says `clm land` refuses an edit to a stale marker's file that land accepts"
kind: question
components: [orchestrator.run, land, gate, check]
choices:
  - "Prompt follows the gate"
  - "Gate refuses such edits"
---

`orchestrator.run/pending-prompt` (at `209c6f80`) says that, once the root sets `wiring`, a
`behind` requirement's stale marker in a file another component owns is listed apart, saying
`clm land` refuses this build's edit to the file (`land/build-holds-own-files`) and `clm check`
reports the marker unless an exemption covers it (`check/tests-cite-their-owner`), and to add a
`question` item that blocks the requirement. The build of `orchestrator.run` built it as written.

But the gate's `foreign_files` (`gate/own-files`), which `clm land` uses, lets `impl/<c>`
change a file of another component that cites `req:<c>/` before or after the change, and a
modified capability test the component serves. A file holding a stale `req:b/x@...` marker
always cites `req:b/`, so land accepts the very re-cite the prompt tells the build not to make,
and the build files a blocking question nobody needed. `check/tests-cite-their-owner` also
reads only files a component's `tests` globs match, so a marker in a file owned only through
`paths` is never reported, which the prompt says it is.

One of the two rules has to move: either `pending-prompt` lists apart only the files land
would refuse (ideally by one shared predicate in `clm_gate`, rather than `clm-run`'s copy of
the ownership test, which also reads the merged tree's graph where land reads the merge
base's), or the gate stops exempting a foreign file for citing the build's own requirement.
Found by `/code-review` on `impl/orchestrator.run`.
