---
title: The mutation pass can't judge requirements of check.coverage
kind: bug
components: [check.coverage]
---

Each of the mutation passes after the lands named left these requirements of `check.coverage` unjudged, having stopped, reached its limit or failed before its verdict:

- `check.coverage/pending-report`: `bf0d9028`, `88470edf`
- `check.coverage/status-report`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.
