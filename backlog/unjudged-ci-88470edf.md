---
title: The mutation pass can't judge requirements of ci
kind: bug
components: [ci]
---

Each of the mutation passes after the lands named left these requirements of `ci` unjudged, having stopped, reached its limit or failed before its verdict:

- `ci/mutate-script`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.
