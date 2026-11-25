---
status: applied
resolves: [unjudged-web.worker.replica-88470edf]
---

## Intent

This change deletes `backlog/unjudged-web.worker.replica-88470edf.md` as a symptom of `backlog/unjudged-web-bf0d9028.md`, and adds to that item what the deleted one showed.

Nothing in `web.worker.replica` keeps the mutation pass from judging `web.worker.replica/alarm-batches` and `web.worker.replica/times-as-instants`. The pass after `88470edf` never started their blocks. `clm verify` carries what the last pass left to the front of the next, so the five `web` requirements that `unjudged-web-bf0d9028` names went first (`graph-fills-height`, `invited-users`, `no-install-scripts`, `sign-in-early-access`, `site-served-directly`). StrykerJS failed on each with "No tests were executed", because vitest found no test related to the mutated files. `mutate.sh` starts no block once one has failed (`ci/mutate-script`), so the pass wrote no verdict: 0 killed, 0 unproven, 0 not judged, 29 left. The replica's two requirements were among the 29.

The fix lies in those `web` blocks, or in the rule that one block's failed tool ends the pass. It does not lie in this component's code or tests, and the item that names the cause is already open. A second item for the same cause would ask a second draft to design the same mend. `unjudged-web-bf0d9028` now records that its failures starve every pass and names the two requirements. Once it is mended, the next pass tries the two requirements first, since they are still carried. If a pass leaves them again, `clm verify` files a new item for them (`files-unproven`), so deleting this one loses nothing.

## Open questions

None.
