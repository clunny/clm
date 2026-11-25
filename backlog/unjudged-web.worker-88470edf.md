---
title: The mutation pass can't judge requirements of web.worker
kind: bug
components: [web.worker]
---

Each of the mutation passes after the lands named left these requirements of `web.worker` unjudged, having stopped, reached its limit or failed before its verdict:

- `web.worker/architecture-decide`: `bf0d9028`, `88470edf`
- `web.worker/decide`: `bf0d9028`, `88470edf`
- `web.worker/proposals`: `bf0d9028`, `88470edf`
- `web.worker/views-from-replica`: `bf0d9028`, `88470edf`
- `web.worker/webhook-refresh`: `bf0d9028`, `88470edf`
- `web.worker/writes-to-object`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.
