---
title: "A test in another component's file that cites a removed requirement is listed for deletion by the change that removes it"
date: 2026-11-19
status: accepted
---

Once the root sets `wiring`, a build changes only its own component's files (`own-files-at-the-gate`). A requirement `b/gone` that a change removes may still be cited by a test in a file component `c` owns, where an exemption from `check/tests-cite-their-owner` covers the file or a change gave the file to `c`. `b`'s build may not edit that file, and `c`'s build receives none of `b`'s requirements (`ancestor-scoped-context`), so nobody can re-cite the test where it sits. Left as it was, the requirement stayed `obsolete` on `b`, which the queue offered after every build, and its change never archived.

**The change decides, at design time.** The session that removes a requirement reads the whole repository, and its branch is where the human reviews what goes. Every such test goes on the change's `deleted-tests.txt`, which makes the branch need review (`cuts-are-what-a-program-lists`). Where the behaviour stays stated by another requirement, that requirement's own build proves it in its own files; the copy in `c`'s file is never kept.

**A program holds it.** `check/obsolete-tests-elsewhere` fails `clm check` for a test in a file another component owns that cites a removed requirement and that no `deleted-tests.txt`, live or archived, names. It reads the whole tree, so a repository already in the state gets a failure that names the test and what to do, where a rule at the gate would hold only new branches.

**The owner's build carries it out and judges nothing.** The removed requirement is pending on each other component that owns a file citing it (`check.coverage/pending-report`), and that build deletes the tests the list names and removes markers outside a test. The removed requirement's own component gets its line only once no other component's file cites it: deleting a test breaks nothing, and removing code a test still runs does, so the lines are ordered by which exists, with no new kind of blocker.

**What counts as the requirement's own.** A file a `wiring` glob matches, which every build may change, and a file no component owns. A removed capability keeps the placement `removed-capabilities-build-out` gives it. A project that sets no `wiring` sees none of this.

**What stays.** Code that implemented the removed requirement and that no marker marks is on no build's list, as before; `no-dead-code`'s detectors find it.

Considered:
- **Listing the marker apart on the removing component's prompt, with a question to file.** Built on 2026-11-19 and replaced: the question can block nothing, since the requirement is gone.
- **Letting a question block an obsolete line.** Rejected: it ends the loop by handing the human each test, which is implementation and not intent.
- **Giving the marker to the file's owner to remove or re-cite.** Drafted and not proposed: the owner can't judge a re-cite, which `stale-markers-keep-behind` had rejected for a `behind` marker, and nothing ordered its build against the removing component's.
- **Accepting it while no component holds an exemption.** Rejected by the human on 2026-11-19: a known fault is fixed.
