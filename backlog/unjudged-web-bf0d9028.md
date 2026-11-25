---
title: The mutation pass can't judge requirements of web
kind: bug
components: [web]
---

Each of the mutation passes after the lands named left these requirements of `web` unjudged, having stopped, reached its limit or failed before its verdict:

- `web/graph-fills-height`: `6d6325c6`, `bf0d9028`
- `web/invited-users`: `6d6325c6`, `bf0d9028`
- `web/no-install-scripts`: `6d6325c6`, `bf0d9028`
- `web/sign-in-early-access`: `6d6325c6`, `bf0d9028`
- `web/site-served-directly`: `6d6325c6`, `bf0d9028`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.

These five fail every pass that carries them, and starve it: in the pass after `88470edf`, StrykerJS stopped on each ("No tests were executed": vitest found no test related to the mutated files), and `mutate.sh` starts no block once one fails. Carried first, they took the first job slots, so that pass wrote no verdict at all (0 killed, 0 unproven, 0 not judged, 29 left). Among the 29 were `web.worker.replica/alarm-batches` and `web.worker.replica/times-as-instants`, whose item (`unjudged-web.worker.replica-88470edf`) was deleted as a symptom of this one. The mend may lie in these requirements' tests or in `ci/mutate-script`'s rule that one block's failed tool ends the pass.
