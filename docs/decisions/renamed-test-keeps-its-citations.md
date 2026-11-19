---
title: "A test renamed in its own file that cites everything it cited is not a deleted test"
date: 2026-11-18
status: accepted
---

`cuts-are-what-a-program-lists` had `clm land` refuse a renamed test, on the reasoning that a build keeps a test's name. The first build to meet it (`impl/host`, renaming two tests whose names stated an exit status the requirement had changed) could not keep a true name: a code branch can't add a line to a `deleted-tests.txt`, so the only ways through were a spec branch for a rename or a name that says something false. It took the false name and a comment explaining it.

**What counts as a rename** (`check.coverage/test-lists`). A test `deleted_tests` would return is matched instead, one for one, with a marker-holding test at the head of the same file whose bare name no marker-holding test at the base carries, and whose markers cite every requirement the gone test's cited, versions left out. A matched test is renamed, not gone.

**Why that loses nothing a removal yes protects.** The check is about names already: a build may rewrite a test's body under its old name, keeping its markers, and the land has never refused it. A new name in the same file with at least the same citations gives no more room than that. Whether the test still proves what it cites is the judge audit's and the mutation pass's question, as it is for a test edited in place.

**What is still gone.** A test renamed and moved to another file in one branch, one renamed that cites fewer requirements, one whose new name some test at the base already carries, and every gone test beyond the new names its file gains. Each of these errs toward refusing: the build keeps the test, or the change lists it.

Considered:
- **A rename recorded by the build, such as a commit trailer.** Rejected: it is the session's word where a program can check the same thing from the two trees.
- **Matching a new name anywhere in the tree.** Rejected: the case met was a rename in place, and a match across files would let any test elsewhere that cites the same requirement stand for a deleted one.
- **Comparing the test's body.** Rejected: a rename that comes with a change to what the test asserts, the case that met the rule, would never match.
