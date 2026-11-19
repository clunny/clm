---
status: proposed
resolves: [an-obsolete-marker-in-another-component-s-file-is-nobody-s-to-remove]
---

## Intent

When a change removes a requirement that a test in another component's file cites, what happens to that test is decided in the change, checked by `clm check`, and carried out by the build of the component that owns the file, which judges nothing. It answers `backlog/an-obsolete-marker-in-another-component-s-file-is-nobody-s-to-remove`, as the human chose on 2026-11-19: the correct fix, not acceptance, though no component of this repository holds the exemption the case needs.

## Problem

Once the root sets `wiring`, `clm land` refuses a build's edit to another component's file. A removed requirement `b/gone` cited by a test in `c`'s file stays `obsolete` on `b` alone. `b`'s build is told to leave the marker and file a question, which can block nothing since the requirement is gone, and `c`'s build is never shown it. So `b` is offered after every build, the removing change never archives, and a deletion the human approved in a `deleted-tests.txt` sits unacted on. It arises where an exemption from `check/tests-cite-their-owner` covers the file, or where a change gives a file to another component.

Three earlier answers failed, each for a reason this design takes in:

- Listing the marker apart on `b`'s prompt (built) leaves nobody to remove it.
- A question that blocks the obsolete line ends the loop and hands the human each test, which is implementation, not intent.
- Giving the marker to the file's owner, as first drafted, asks `c`'s build to judge a re-cite against requirements its bundle doesn't hold (`ancestor-scoped-context`), which `stale-markers-keep-behind` rejected for a `behind` marker; it also ordered nothing between two builds.

## Approaches

- **Taken: the change that removes the requirement lists every such test for deletion, and `clm check` fails when one isn't listed.** The session that removes a requirement reads the whole repository and its branch is where the human reviews what goes. A test of `b`'s requirement in `c`'s file can't be re-cited where it sits by anyone: `b`'s build may not edit the file and `c`'s can't judge it. So the copy in `c`'s file always goes, on the list the human approves (`gate/cuts`), and where the behaviour stays stated by another requirement, that requirement's own build proves it in its own files.
- **Taken: the owner's build deletes what the list names.** With every such test listed, the owner has nothing to judge, so `stale-markers-keep-behind`'s objection doesn't apply: it deletes named tests and removes markers that sit outside a test.
- **Taken: the removed requirement is pending on its own component only once no other component's file cites it.** Deleting a test breaks nothing; removing code another component's test still runs does. Sequencing the two lines by presence needs no new kind of blocker: the own component's line appears when the others are gone.
- **Taken: a file a `wiring` glob matches, and a file no component owns, count as the removed requirement's own component's.** Every build may change the first, and nobody else has the second.
- **Taken: a removed capability keeps its own placement** (`check.coverage/removed-capabilities`).
- **Not taken: a rule at the gate only.** It would hold new branches and leave a repository already in the state looping. As a check of the whole tree, the state is a named failure that says what to do.
- **Not taken: an exemption from the new check.** It is the state an exemption leads to that the check ends.

## Outcome

- `check/obsolete-tests-elsewhere` (new): with `wiring` set, a test in a file another component owns that cites a removed requirement must be named by a `deleted-tests.txt`.
- `check.coverage/pending-report`: such a requirement is pending on each owner of a file that cites it, and on its own component once no other component's file does.
- `orchestrator.run/pending-prompt`: the owner's prompt lists the other component's obsolete requirement with the tests to delete; the clause that listed it apart on the removing component's prompt goes.
- `orchestrator`'s Design and five decisions say where an obsolete marker counts.

## Constraints

- A project that sets no `wiring` sees no change.
- Code that implemented a removed requirement and that no marker marks is on no build's list, before this change and after it; `no-dead-code`'s detectors are what find it.
- A project that sets `wiring`, holds an exemption from `check/tests-cite-their-owner` and has such a test today fails `clm check` on upgrading until a spec branch lists the test. No file, key or format it holds changes.

## What the requirements no longer state

- `orchestrator.run/pending-prompt` loses the clause that an obsolete requirement's marker in a file another component owns "SHALL be listed apart the same way once the root sets `wiring`, naming that owner and saying that `clm land` refuses this build's edit to the file and to leave it and add a `question` item, which names the file and blocks nothing, the requirement being gone, and SHALL be in neither of the two lists below nor among the markers whose code the prompt says to remove". The marker is on its owner's prompt instead. Its scenario "an obsolete test in a file the build may not change" keeps its name; it no longer says the test is named "neither as a test to delete nor as one to keep", and shows it on the owner's prompt as a test to delete.
- `check.coverage/pending-report` no longer has an obsolete requirement always pending on its own component: with `wiring` set and another component's file citing it, it is pending there first.

No requirement and no scenario is removed.

## What the builds do

`check.coverage`, then `check` and `orchestrator.run`. `orchestrator.run`'s build removes the code and the test of the listing apart for an obsolete marker (`foreign_tests` and its text), which the modified requirement no longer states.
