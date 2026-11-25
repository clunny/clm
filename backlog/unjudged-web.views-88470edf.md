---
title: The mutation pass can't judge requirements of web.views
kind: bug
components: [ci]
---

Each of the mutation passes after the lands named left these requirements of `web.views` unjudged, having stopped, reached its limit or failed before its verdict:

- `web.views/answer-buttons`: `bf0d9028`, `88470edf`
- `web.views/architecture-decide-buttons`: `bf0d9028`, `88470edf`
- `web.views/decide-buttons`: `bf0d9028`, `88470edf`
- `web.views/home-waiting-on-you`: `bf0d9028`, `88470edf`
- `web.views/propose-from-backlog`: `bf0d9028`, `88470edf`

Each pass after the first tried them first, so one that left them was stopped, reached `mutate_minutes`, or met a mutation command that fails on their code: `clm verify`'s stderr for those passes says which. Mend what keeps the pass from its verdict, then delete this item.

What kept the pass from a verdict lies in `ci`, not `web.views`: the pass after `88470edf` ran `.github/scripts/mutate.sh`, whose StrykerJS runs for five `web` blocks (`web/no-install-scripts`, `web/graph-fills-height`, `web/invited-users`, `web/sign-in-early-access`, `web/site-served-directly`) stopped with "No tests were executed" after vitest found no test files related to the mutated files. The script then failed, started no other block and wrote no verdict at all ("0 killed, 0 unproven, 0 not judged, 29 left"), so `web.views`' blocks, later in the plan, never ran. All five of its requirements are current, and its code and tests play no part (see `backlog/one-block-s-strykerjs-failure-leaves-every-other-component-s.md`, reported by `20261124-unjudged-web.worker-88470edf`).
