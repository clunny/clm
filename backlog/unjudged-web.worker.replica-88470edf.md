---
title: The mutation pass can't judge requirements of web.worker.replica
kind: bug
components: [web.worker.replica]
---

Each of the mutation passes after the lands named left these requirements of `web.worker.replica` unjudged, having stopped, reached its limit or failed before its verdict:

- `web.worker.replica/alarm-batches`: `bf0d9028`, `88470edf`
- `web.worker.replica/times-as-instants`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.
