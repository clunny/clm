---
title: The mutation pass can't judge requirements of web.views
kind: bug
components: [web.views]
---

Each of the mutation passes after the lands named left these requirements of `web.views` unjudged, having stopped, reached its limit or failed before its verdict:

- `web.views/answer-buttons`: `bf0d9028`, `88470edf`
- `web.views/architecture-decide-buttons`: `bf0d9028`, `88470edf`
- `web.views/decide-buttons`: `bf0d9028`, `88470edf`
- `web.views/home-waiting-on-you`: `bf0d9028`, `88470edf`
- `web.views/propose-from-backlog`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.
