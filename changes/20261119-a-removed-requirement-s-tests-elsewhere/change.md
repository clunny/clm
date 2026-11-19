---
status: applied
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
- **Taken: the removed requirement's line on its own component is blocked while another component holds a line for it.** Deleting a test breaks nothing; removing code another component's test still runs does. `clm pending` prints the wait as `blocked` by the requirement's own ID, which the queue already reads as a requirement that is pending somewhere, so it needs no new kind of blocker and the line stays in view. Hiding the line instead, as first drafted, would have let `clm rebase` set a component `active` with obsolete markers still in its files.
- **Taken: a test whose requirement moved to the file's own component is re-cited, not listed.** That build does hold the requirement it moved to, and deleting proof of behaviour that is still stated would ask the human to approve a deletion nobody wants (`cuts-are-what-a-program-lists`). The check leaves such a test alone and the prompt names it to re-cite.
- **Taken: a removed component's tests follow the same rule.** They already count on their file's owner (`node-removal`), whose build `clm land` now refuses to let delete an unlisted test, so they need the list as much.
- **Taken: a test that also cites a requirement the spec holds only loses its marker.** It stays a citing test, so nothing is deleted and nothing needs a list.
- **Taken: when no file of the removed requirement's own component cites it, the owner's prompt says to file a `deferred` item for that component's code.** No marker is left to carry the work to it, and a `deferred` item is how code work no requirement makes pending is carried.
- **Taken: `orchestrator.run` builds before `check.coverage`** (`expand-contract`). `crates/clm-run/tests/run.rs` asserts today's placement and `check.coverage`'s build may not edit it. `pending-prompt` is worded for a requirement `pending-report` already places on a file's owner, a removed component's, so it is true before the placement widens and after; `pending-report` needs it. Between the two builds, the removing component's prompt lists a test in another component's file among its own, which the land refuses; no component here holds the exemption that case needs.
- **Taken: a file a `wiring` glob matches, and a file no component owns, count as the removed requirement's own component's.** Every build may change the first, and nobody else has the second.
- **Not taken here: a removed capability.** It keeps its own placement (`check.coverage/removed-capabilities`), which has the same fault when another component's file cites it; `backlog/an-obsolete-requirement-nobody-can-clear-keeps-its-component-ready` carries it.
- **Not taken: a rule at the gate only.** It would hold new branches and leave a repository already in the state looping. As a check of the whole tree, the state is a named failure that says what to do.
- **Not taken: an exemption from the new check.** It is the state an exemption leads to that the check ends.

## Outcome

- `check/obsolete-tests-elsewhere` (new): with `wiring` set, a test in a file another component owns that cites a removed requirement must be named by a `deleted-tests.txt`.
- `check.coverage/pending-report`: such a requirement is pending on each owner of a file that cites it, and its line on its own component is blocked while one of those is left.
- `orchestrator.run/pending-prompt`: the owner's prompt lists another component's obsolete requirement with the tests to delete, to re-cite after a move, and the markers to take out; the clause that listed it apart on the removing component's prompt goes.
- `check.coverage/move-targets` (new): the lookup of where a move took a removed requirement, which was private to the crate.
- `view/blockers-view`: a component whose obsolete requirement waits on other components' lines is shown under those components.
- `orchestrator`'s Design and six decisions say where an obsolete marker counts.

## Constraints

- A project that sets no `wiring` sees no change.
- No file, key or format a released project holds changes, and `wiring` is in no published release, so no project meets the new check on upgrading.

## What the requirements no longer state

- `orchestrator.run/pending-prompt` loses the clause that an obsolete requirement's marker in a file another component owns "SHALL be listed apart the same way once the root sets `wiring`, naming that owner and saying that `clm land` refuses this build's edit to the file and to leave it and add a `question` item, which names the file and blocks nothing, the requirement being gone, and SHALL be in neither of the two lists below nor among the markers whose code the prompt says to remove". The marker is on its owner's prompt instead. Its scenario "an obsolete test in a file the build may not change" keeps its name; it no longer says the test is named "neither as a test to delete nor as one to keep", and shows it on the owner's prompt as a test to delete.
- `check.coverage/pending-report` no longer has an obsolete requirement pending on its own component alone: with `wiring` set and another component's file citing it, it is pending there first, and has no line on its own component when none of that component's files cites it.

No requirement and no scenario is removed.

## What the builds do

`check.coverage` twice: first `move-targets`, which needs nothing and which the check and the prompt both read, then `check`, then `orchestrator.run`, then `check.coverage` again for `pending-report`, then `view`, each by its `#### Needs`.

- `orchestrator.run`'s build rewrites, under their names, the two tests of the scenario "an obsolete test in a file the build may not change" (`crates/clm-run/tests/run.rs` and `prompt.rs`), and removes the code of the listing apart for an obsolete marker, which the requirement no longer states. No test is deleted.
- The same build takes out of `scenario_stale_marker_in_a_file_the_build_may_not_change` and `a_stale_marker_in_a_wiring_file_is_re_cited`, in `crates/clm-run/tests/run.rs`, their assertions on `b/gone`'s marker in `crates/c/gone.rs`, or gives those fixtures no obsolete marker in `c`'s file: both tests are about a `behind` marker, and where that obsolete marker is listed changes again when `check.coverage` builds `pending-report`, in a file that build may not edit.
- `check.coverage`'s second build puts the wait on the own ID into `waits_on` from the lines other components hold after the split by owner, never from the set of pending IDs, in which the line's own ID always is; and it drops a move blocker on a line's own component before it adds that wait.

## Open questions

- **An obsolete requirement nobody can clear keeps its component ready.** The same loop exists wherever a build can neither delete nor re-cite a removed requirement's test: a test in the requirement's own files that no list names and nothing states any more, and a removed capability's test in another component's file. The question the build files can block nothing. `backlog/an-obsolete-requirement-nobody-can-clear-keeps-its-component-ready` is a `bug`; whether the build loop launches such a component again and again was not measured.
