---
title: "An obsolete marker in another component's file is nobody's to remove"
kind: question
components: [check.coverage, orchestrator.run, model]
choices:
  - "Accept it: no exemption, no case"
  - "Let a question block an obsolete line"
  - "Owner step, with its three gaps closed"
---

When a change removes requirement `b/gone` and a test in a file component `c` owns still cites it, `check.coverage/pending-report` keeps `b/gone` `obsolete` on `b`. Once the root sets `wiring`, `clm land` refuses `b`'s build an edit to `c`'s file, so the build prompt lists the marker apart and tells the build to leave it and add a `question` item (`orchestrator.run/pending-prompt`). That item can block nothing, since `model/parse-backlog` rejects a `blocks` entry naming a requirement the spec doesn't hold. So `b` stays pending and ready, the queue offers it again, and the removing change never archives. When an approved `deleted-tests.txt` names the test, the approval sits in that question and nothing acts on it.

It arises only in a file an exemption from `check/tests-cite-their-owner` covers, since `clm check` otherwise fails a test that cites another component. This repository holds no exemption (`clm health` on 2026-11-19: `health/exemption: 0 exemptions`), so it can't happen here today.

The human first chose to list the marker apart (built), then, on 2026-11-19, to count it as pending on the file's owner. That was drafted and its architecture audit found three gaps, so it was not proposed:

- **The removing component's code.** With no line on `b`, nothing tells `b`'s build to remove what `b/gone` needed, and `c`'s build may not (`no-dead-code`).
- **No order between the two builds.** `b` and `c` would each hold a line for one requirement, both ready at once; `b` removing code `c`'s test still runs is held only by `contract-removal-needs`' fallback.
- **The owner can't judge a re-cite.** `c`'s bundle doesn't hold `b`'s requirements (`ancestor-scoped-context`); `stale-markers-keep-behind` rejected the same step for a `behind` marker for that reason. It also contradicts `moved-markers-wait`, `removed-requirements-build-out` and `contract-removal-needs`, which give the markers to the removed requirement's own component, and leaves `check.coverage/removed-capabilities` and `orchestrator`'s Design saying otherwise.

Its builder check found three more choices the owner step would have to make: a marker left in a child's file by a nested split would wait on a requirement of its own component, which `orchestrator`'s Design says never happens; a file a `wiring` glob matches is every build's to change, so its owner is not the only one who can; and a removed capability's markers follow another requirement, `check.coverage/removed-capabilities`.

The choices:

- **Accept it** while no component holds an exemption: the route to one already needs the human's review (`gate/needs-review`), which is where the cost can be weighed. Delete this item, and record the acceptance beside `temporary-exemptions`.
- **Let a question block an obsolete line**: `model/parse-backlog` accepts in `blocks` a requirement a live change removed, so the question the build files holds `b` back and reaches the human, who approves the deletion or has the test moved. Smallest change that ends the loop; the human still acts on each case.
- **The owner step**, with a line kept on `b` for its code that waits on `c`'s, and the owner's prompt saying it deletes only what an approved list names and otherwise files the question. The largest: it touches `check.coverage`, `orchestrator.run`, `orchestrator` and three decisions.
