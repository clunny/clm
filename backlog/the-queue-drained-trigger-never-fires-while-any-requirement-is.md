---
title: "The queue-drained trigger never fires while any requirement is blocked"
kind: bug
components: [verify]
---

Step: `clm verify --due`, asked by the build loop after its last run returned, in a project whose `[verify] triggers` holds `queue-drained`.

What happened: the build queue was empty of work (nothing ready, no claim live), and `clm verify --due --dry-run` answered `not-due no trigger holds`. The trigger holds only when no requirement is pending, and a requirement blocked by a backlog item or by a need is pending. The project held requirements blocked on purpose, waiting on a human's answer and on later work, so the trigger had not been true since they were blocked and could not become true until they are built. Every verification came from the `every` trigger or was run by hand, and a finished build's lands went unverified, and its tests unjudged by the mutation pass, until enough further lands had happened.

Expected: the trigger holds when the queue has nothing left that a build could take: no requirement is ready, as `clm next` computes it, whether or not blocked ones remain. A requirement waiting on a human or on other work is not work the loop is about to do.

Why it matters more now: the mutation pass runs only after a verification, so with this trigger dead a build's new tests wait for up to `every.lands` further lands before anything judges them.

Rule involved: `verify/verify-due` ("`queue-drained` when no requirement in `origin/main`'s tree is pending"), `verify-after-land`. It needs a spec change: the requirement as written states the behaviour seen.
