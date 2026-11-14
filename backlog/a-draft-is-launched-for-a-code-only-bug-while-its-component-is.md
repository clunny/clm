---
title: "A draft is launched for a code-only bug while its component is claimed by a build, and ends with its fix claim refused"
kind: bug
components: [orchestrator.triage]
origin: 20261113-no-test-reads-what-2
---

`clm draft` launched a draft for `no-test-reads-what-clm-upgrade-lock-appends`, a `kind: bug` item whose body says its fix is code-only and lies in `clm`. The draft's only route for such a bug is `clm claim clm --fix backlog/<slug>`, which was refused: "clm claim: `clm` is claimed: `origin/impl/clm` has a live commit", a build's claim made minutes before. The draft could fix nothing and left the item.

This is the second draft the same day to end that way for the same item: `20261113-no-test-reads-what` was refused the same claim for the same reason, and `20261113-no-test-reads-what-2` repeated it. Each cost a whole session, a spec branch, a pull request and a land that changed nothing about the item, and the next triage will group and draft it again for as long as a build holds `impl/clm`, which has pending requirements most of the time.

Nothing tells triage or the draft, before the session is spent, that a bug's components are all claimed, and nothing records on the item that it waits on a claim, so the same item is drafted again and again. The draft prompt's check against `origin/main` covers an item already fixed, not one that can't be fixed yet.
