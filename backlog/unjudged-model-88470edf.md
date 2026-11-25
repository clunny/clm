---
title: The mutation pass can't judge requirements of model
kind: bug
components: [ci]
---

Each of the mutation passes after the lands named left these requirements of `model` unjudged, having stopped, reached its limit or failed before its verdict:

- `model/move-records`: `bf0d9028`, `88470edf`
- `model/parse-changes`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.

What kept the pass from a verdict lies in `ci`, not `model`: the pass after `88470edf` ran `.github/scripts/mutate.sh`, whose StrykerJS runs for five `web` blocks (`web/no-install-scripts`, `web/graph-fills-height`, `web/invited-users`, `web/sign-in-early-access`, `web/site-served-directly`) stopped with "No tests were executed" after vitest found no test files related to the mutated files. The script then failed (`mutate.sh failed with status 1`), started no other block, and wrote no verdict at all ("0 killed, 0 unproven, 0 not judged, 29 left"), so `model`'s blocks, later in the plan, never ran. `model`'s code and tests play no part.
