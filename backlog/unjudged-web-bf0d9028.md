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
