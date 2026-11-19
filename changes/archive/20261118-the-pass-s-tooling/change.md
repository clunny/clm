---
status: done
resolves: [a-build-run-as-a-subagent-leaves-no-effort-record-for-its-branch]
---

## Intent

Programs now list what a spec branch removes and refuse a code branch that deletes a test nobody approved, so `removal-needs-a-yes` is checked in part before the 51 components are pruned. It is step 2 of phase 5 of `backlog/a-passing-check-proves-less-than-the-spec-claims`, the tooling: four advisory `clm health` signals that count what the rule for a requirement forbids, the gate's reading of a cut, `clm land`'s check of deleted tests, and the spec-slimmer rewritten to the rule. It also has a build's model work recorded against its own branch, an exception to the freeze the human gave on 2026-11-18, since the plan wants what each component's pass costs.

## Problem

- **Nothing checks a removal.** `gate/needs-review`'s own scenario lets a branch that removes a requirement land on checks. Replayed over the last 150 spec lands, 26 removed a requirement, a scenario or a SHALL sentence; nine were ordinary changes that dropped exactly one scenario and landed with nobody's yes.
- **Nothing checks a deleted test.** Of the 163 `impl/*` lands since `8fe48394`, 14 (9%) lost a citing test by name, 20 tests, about half of them a build renaming or replacing a test. The build prompt tells a build to remove every test of an `obsolete` requirement, which is wrong when the requirement was merged into another and its tests should be re-cited.
- **The rule for a requirement has no count.** `requirements-state-what-the-contract-shows` was measured once by nine readers whose rows are gone. A wave that prunes a group of components has to measure it again by hand.
- **The spec-slimmer predates the rule.** It proposes cuts by its own four kinds and returns one list, so a reworded requirement and a cut one reach the human alike.
- **A build's cost is not recorded.** A build run as a subagent writes transcript entries whose `gitBranch` is the launching session's, `main`, though it worked on `impl/<component>` in a worktree of its own, so the ledger holds no `effort` record for the build (`backlog/a-build-run-as-a-subagent-leaves-no-effort-record-for-its-branch`). The pilot's cost was added up by hand.
- **The gate and the Worker disagree on an exemption.** The gate asks review for an added or widened exemption, but by a function of its own that reads git; `needs_review_of`, which the Worker calls, doesn't know the rule, so the web review says such a branch needs no review. `gate/needs-review-of` says the two give one answer.

## Approaches

- **Taken: a cut is read from the two spec graphs and from a list the change declares.** `clm-gate`'s `cuts` compares the requirements at the merge base with the head's. A requirement that is gone is a cut unless each of its scenarios is *moved*; a scenario whose name is gone from its requirement is a cut unless it is moved; a requirement that states fewer sentences is a cut; and each line a branch adds to a change's `deleted-tests.txt` is a cut. A spec branch with any cut needs review.
- **Taken: a scenario is moved one for one, within the diff.** A gone scenario is moved when the head holds a scenario of that name under another requirement where the base held none, and each such place takes one gone scenario. Matching a name against the whole head would never list a name many requirements share: 15 requirements hold a scenario named "covered by the scan". So a renamed scenario is listed, a scenario moved to another component is not when both deltas are in one branch, and of two same-named scenarios merged into one requirement one is listed.
- **Taken: a requirement that states fewer sentences is listed, a sentence that places a test left out of the count.** It is the one part of "a SHALL sentence a requirement loses needs the human's yes" a program can see, and it would have caught the pilot's first draft, which moved two library offerings out of their requirements. A sentence holding one of a fixed list of phrases that place a test isn't counted, since the human agreed on 2026-11-18 that such a sentence goes with no separate yes. What it leaves a reader: a reword that keeps the count, and a sentence of behaviour worded like one that places a test.
- **Taken: the tests a change deletes are a list the change declares, `changes/<id>/deleted-tests.txt`.** The plan first named `clm coverage --obsolete-tests`. That list holds every test of a merged requirement, which a build re-cites, and misses a duplicate test whose requirement stays. A declared list errs safely both ways: a line too many is read by the human, and a line too few has `clm land` refuse the build, which then keeps the test.
- **Taken: an approved list is one on the base the land lands on.** A list reaches `main` only through the gate, which asks review for each line a branch adds. `clm land` reads the lists of live and archived changes from the fetched base, never from the branch, so a build can't approve its own deletion; the archive is read because a change can be archived before a duplicate's test is deleted.
- **Taken: every land but a `spec/*` branch's is held to the lists, not only `impl/*`.** A small code branch deleted a citing test in the replay too. The check runs before the gate, so it goes by the branch's name as `spec-branch-holds-spec-paths` does, and reads no kind.
- **Taken: a test is gone when the tree holds fewer citing tests of its name.** A test moved to another file under its name is not gone; 905 of the 924 tests `clm coverage --deleted-tests` listed in the replay were moves. A test that loses its last marker is gone, since it proves nothing any more. A marker in a test's title is left out of the name, so re-citing doesn't rename it.
- **Taken: four health signals, a line per finding.** The plan said a count per component. A line per finding gives the count, as each line starts with the component's spec path, and gives a wave the rows it would otherwise measure by hand.
- **Taken: the gate's exemption rule moves into a function of two graphs, `widened_exemptions`,** which the Worker calls beside `cuts`, so the two give one answer again.
- **Taken: the effort of a subagent's transcript goes to the branch checked out where it ran.** The entries' `cwd` is the subagent's worktree, and `clm land` runs there with the build's branch checked out.
- **Not taken: failing `clm check` on any of the four counts.** Each is a heuristic (`requirements-state-what-the-contract-shows`).
- **Not taken: matching a renamed scenario by its unchanged text.** A rename with the same text is rare, and one line on the list is its whole cost.
- **Not taken: a requirement for the spec-slimmer's prompt.** Its text is source (`prompt-text-is-source`). The rewrite is `backlog/the-spec-slimmer-predates-the-rule-for-a-requirement`, a `deferred` item fixed under `clm claim plugin.agents --fix`.
- **Not taken: changing the slimmer's `model`.** `plugin/model-names` keeps `sonnet` in the file. For phase 5's waves the launching session names the build model, which the plan item says.

## Outcome

- `clm gate` prints one `gate/cut` line for each cut after the kind line, and a spec branch with any prints `spec review`. A line a branch adds to a `deleted-tests.txt` that names no citing test is a violation.
- `clm pr open` and `clm pr body` end a spec branch's body with a `## Cuts` section holding those lines.
- The Worker's review carries the lines as `cuts`, and its `review` is true for a branch with a cut or a widened exemption.
- `clm land` and `clm land --check-only` refuse a code branch that loses a citing test no approved list names.
- The build prompt tells a build which of an obsolete requirement's tests an approved list names, to delete, and which to re-cite or leave.
- The web review lists the cuts above the branch's components.
- `clm health` reports `outside-contract`, `test-placement`, `design-exit` and `uncited-test`.
- `clm coverage --deleted-tests` no longer lists a moved test.
- `clm land` records a build subagent's work against the build's branch.
- `ARCHITECTURE.md`'s `removal-needs-a-yes` says what is checked and what stays the session's.

## Constraints

- `clm land` gains only checks that take milliseconds: the deleted tests are read from the files the diff changes.
- The health signals gate nothing (`health-advises`).
- No file, key or format a released project holds changes. `changes/<id>/deleted-tests.txt` is new and optional; the guide's spec-format page describes it (`method-in-the-plugin`).

## What the requirements no longer state

- `gate/needs-review` loses "Nothing else SHALL make a branch need review" in its old reach: a cut now does, and the sentence is kept with the cut named before it. Its scenario "nothing else" no longer removes a requirement in its GIVEN; the name is kept.
- `orchestrator.run/pending-prompt` no longer says to remove every test an obsolete requirement's markers mark: it lists the tests an approved list names, to delete, and the others, to keep. It still says to remove the code.
- `host/pr-body` loses "changing nothing else" of the body it sets: on a spec branch with cuts the body ends with the section. It still changes nothing else of the pull request.
- `check.coverage/test-lists` no longer has `deleted_tests` list a test moved to another file under its name.

No requirement and no scenario is removed.

## What the builds do, and their order

`effort`, then `check.coverage`, then `gate`, then `check.health`, `host`, `land` and `web.worker.review`, then `orchestrator.run` and `web.views`, each by its `#### Needs`. `plugin.agents`' prompt and the propose and health skills' text are `deferred` items.

- `gate`'s build rewrites `contract_change_needs_no_review` in `crates/clm-gate/tests/gate.rs`, whose fixture removes a requirement and expects `spec`.
- `check.coverage`'s build rewrites `deleted_tests_notice_a_missing_file_and_a_moved_test`, which expects a moved test to be listed.
- `orchestrator.run`'s build rewrites the fixed sentence of the build prompt that says to remove the tests of an `obsolete` requirement, beside the lists the requirement states.
- `web.worker.review`'s build names the cache key anew, `compare5`.

## Open questions

- **The review audit judges earlier lands by the new rule.** `clm ledger verify --review` recomputes each signed land with the gate as it is, so in a signed repository a land that removed a requirement before this rule is reported as understated. This repository is unsigned. `backlog/the-review-audit-judges-earlier-lands-by-today-s-rule-for-review` is a `bug` on `ledger`, due before a release; it needs a design of its own, a land audited by the rule in force when it landed.
- **A subagent that worked on two branches in one worktree** has all its work counted for the one checked out when the record is made, and a record made while a build's worktree is still detached counts those entries for both branches. No build does the first, and the second is a build's first minute.
- **The skills don't yet show the cut lines when they ask for an approval.** The pull request and the web review do. `backlog/the-propose-and-health-skills-predate-the-gate-s-list-of-cuts` holds the text.
