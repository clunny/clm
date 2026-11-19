---
title: "An obsolete requirement nobody can clear keeps its component ready"
kind: bug
components: [model, check.coverage, orchestrator.run]
origin: 20261119-a-removed-requirement-s-tests-elsewhere
---

Since `land/deleted-tests-approved`, a build deletes a removed requirement's test only when an approved `deleted-tests.txt` names it, and otherwise re-cites it. Where it can do neither, the build prompt tells it to leave the test and add a `question` item. `model/parse-backlog` rejects a `blocks` entry naming a requirement the spec doesn't hold, so the question holds nothing back: the requirement stays `obsolete` and unblocked, its component stays ready, and the removing change never archives until a spec change lists the test.

`20261119-a-removed-requirement-s-tests-elsewhere` closes one such case at design time, a test in another component's file. Two are left:

- a test in the requirement's own files that no list names and that no requirement states any more, when the change that removed the requirement forgot the line. A human's yes is rightly needed there, since deleting it is a cut;
- a removed capability's test in a file another component owns: `check.coverage/removed-capabilities` places it on the component that builds the capability tests, whose build `clm land` refuses the edit, and `check/obsolete-tests-elsewhere` leaves capabilities out.

Not measured: whether `/clm:build` launches such a component again after each build that changes nothing, or only shows it as ready. A fix probably lets a `question` item's `blocks` name a requirement a live change removed, so the line waits on the human, and gives a removed capability the placement a removed requirement now has. It needs a spec change.
