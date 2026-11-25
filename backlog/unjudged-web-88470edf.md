---
title: The mutation pass can't judge requirements of web
kind: bug
components: [web]
---

Each of the mutation passes after the lands named left these requirements of `web` unjudged, having stopped, reached its limit or failed before its verdict:

- `web/answer-from-app`: `bf0d9028`, `88470edf`
- `web/approve-by-branch`: `bf0d9028`, `88470edf`
- `web/home-first`: `bf0d9028`, `88470edf`
- `web/honest-errors`: `bf0d9028`, `88470edf`
- `web/mobile-layout`: `bf0d9028`, `88470edf`
- `web/worker-deploy`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.
