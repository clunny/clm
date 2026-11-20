---
component: orchestrator.run
---

## MODIFIED Requirements

### Requirement: pending-prompt <!-- base: d236f8667196b1e0 -->
The `clm run` prompt SHALL list the component's pending requirements, most urgent first, each with its ID, version and state; for a `behind` requirement, the diff from the text at each version its markers cite other than the current one to the current text, taken from the history of the component's `SPEC.md`, or a line saying the earlier text wasn't found, and the locations of the markers that cite it at another version or without one, saying to re-cite each one after checking that its test proves the current text (`path-ownership`, `stale-markers-keep-behind`), wherever its file sits, another component's `paths` included, while the root sets no `wiring`; once the root sets `wiring` (`model/wiring-globs`), a marker in a file another component owns SHALL instead be listed apart, naming that owner and saying that `clm land` refuses this build's edit to the file (`land/build-holds-own-files`), that `clm check` reports the marker unless an exemption covers its file (`check/tests-cite-their-owner`), and to leave it and add a `question` item that blocks the requirement; an `obsolete` requirement whose ID names another component, or one the spec graph doesn't hold, and that is pending on this component for the markers in its files (`check.coverage/pending-report`) SHALL be listed with those markers, saying that it was not this component's and which changes removed it, and, in place of the two lists below: each test an approved list names, as a test to delete; each test whose requirement `check.coverage`'s `moved_to` takes to a requirement of this component, as a test to re-cite to that requirement; each marker outside a test, or on a test that also cites a requirement the spec holds, as a marker to take out; and any other test as one to leave, never to re-cite, since this build's bundle doesn't hold another component's requirements, adding a `question` item whose `blocks` names the removed requirement (`model/parse-backlog`), as `check/obsolete-tests-elsewhere` makes the removing change list such a test wherever it applies. The prompt SHALL NOT say to remove code for such a requirement, and when the spec graph holds the component its ID names and no file of that component cites it, SHALL say that the code it needed is that component's to remove and to add a `deferred` item naming that component and the requirement unless the backlog holds one, except for a removed capability (`check.coverage/removed-capabilities`), whose tests are all it has; and for an `obsolete` one, the locations of its markers, saying to remove the code they mark that no other requirement needs, and its tests in two lists: the tests among the markers that an approved list names (`land/deleted-tests-approved`), read with `gate`'s `list_lines` and compared by `check.coverage`'s `bare_name`, as tests to delete, and every other test, as tests `clm land` refuses this build to lose, saying to re-cite each to the requirement that now states what it proves and, where none does, to leave it and the code it runs and add a `question` item whose `blocks` names the removed requirement (`model/parse-backlog`), so its line waits until the human's answer, a spec change that lists the test in a `deleted-tests.txt` or states what it proves, is applied. The approved lists SHALL be read from the tree the other lists are taken from. It SHALL leave blocked requirements out of the list and name them with what blocks each: the backlog items, or the pending requirement they need, and for an `obsolete` requirement that waits on its own ID (`check.coverage/pending-report`), in place of that ID, the components its other lines are pending on (`check.coverage/wait-lines`), sorted by ID, as the components whose files still cite it, and SHALL NOT name the requirement's own ID as what blocks it. It SHALL take the lists, the earlier texts and the context bundle from the tree `git merge-tree --write-tree origin/main impl/<component>` writes once `clm run` has fetched, never from the checkout it runs in, so that a requirement made current on `origin/main` since a resumed branch was cut neither shows as pending nor blocks another; when that merge conflicts, it SHALL take them from `origin/main`'s tree and say in the prompt that they were read from `origin/main` because the branch conflicts with it, and that `clm pending <component>` after the first `clm rebase` is the list to build from. For a `behind` requirement the prompt SHALL also list, as tests to delete and never to re-cite, each test in a file the build may change that cites the requirement, at any version, and that an approved list names, read and compared as for an `obsolete` requirement's tests, since a change lists a test to delete though its requirement stays, such as a duplicate or the test of a sentence the requirement lost (`cuts-are-what-a-program-lists`); such a test's marker SHALL be left out of the markers to re-cite.

#### Scenario: behind and obsolete
- GIVEN component `b` whose requirement `b/x` is cited at version `11111111`, which `specs/b/SPEC.md` had at an earlier commit, and is now at `22222222`; whose requirement `b/y` is uncited; and a marker at `t.rs:9` citing `b/gone`, which a live change removed
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` as `behind` with a diff from the earlier text to the current, `b/y` as `unbuilt`, and `b/gone` as `obsolete` at `t.rs:9`

#### Scenario: an obsolete requirement's tests
- GIVEN component `b`, a live change that removed `b/gone` and whose `deleted-tests.txt` holds `DELETE t.rs::old_rule`, and tests `old_rule` and `kept_rule` in `t.rs`, each citing `b/gone`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/gone` as `obsolete`, names `t.rs::old_rule` as a test to delete, and names `t.rs::kept_rule` as a test `clm land` refuses the build to lose, to re-cite or else to leave, with the code it runs, and add a `question` item whose `blocks` names `b/gone`

#### Scenario: an obsolete test in a file the build may not change
- GIVEN the root's `wiring: []`, no component `g`, a live change `20260101-x` that removed `g/gone`, and a test `old_rule` citing `g/gone` alone in `crates/c/t.rs`, which component `c` owns
- WHEN `clm run c` runs with `DELETE crates/c/t.rs::old_rule` in that change's `deleted-tests.txt`, and again without that line
- THEN the first prompt lists `g/gone` as `obsolete`, says it was not `c`'s and that `20260101-x` removed it, and names `crates/c/t.rs::old_rule` as a test to delete; the second names it as a test to leave, with a `question` item whose `blocks` names `g/gone`; neither says to re-cite it, to remove code for it, or to add a `deferred` item, `g` being no component

#### Scenario: an obsolete requirement cited from another component's file
- GIVEN the root's `wiring: []`, components `b` and `c` with `paths` `["crates/b/**"]` and `["crates/c/**"]`, a live change `20260101-x` that removed `b/gone` and whose `deleted-tests.txt` holds `DELETE crates/c/t.rs::old_rule`, a test `old_rule` citing `b/gone` alone in `crates/c/t.rs`, and a test `kept_rule` citing it in `crates/b/t.rs`
- WHEN `clm run b` runs, and `clm run c` runs
- THEN `b`'s prompt names `b/gone` among the blocked requirements, as waiting on `c`, whose files still cite it, and not as blocked by `b/gone`; and `c`'s prompt lists `b/gone` as `obsolete`, says it was not `c`'s, and names `crates/c/t.rs::old_rule` as a test to delete

#### Scenario: another component's removed capability
- GIVEN a root with `wiring: []`, components `capability` and `c`, a live change `20260101-x` with `removes_capabilities: [gone]` whose `deleted-tests.txt` holds `DELETE crates/c/t.rs::old_cap`, a test `old_cap` citing `capability/gone` alone in `crates/c/t.rs`, no file of `capability` citing it, and `pending` placing `capability/gone` on `c`
- WHEN the prompt for `c` is written from that pending list
- THEN it lists `capability/gone` as `obsolete`, says it was not `c`'s and that `20260101-x` removed it, names `crates/c/t.rs::old_cap` as a test to delete, and doesn't say to add a `deferred` item

#### Scenario: a test whose requirement moved to this component
- GIVEN the same, the change adding `c/gone`, the move of `g/gone`, `c/gone` current, and no `deleted-tests.txt`
- WHEN `clm run c` runs
- THEN the prompt names `crates/c/t.rs::old_rule` as a test to re-cite to `c/gone`

#### Scenario: another component's requirement whose own files don't cite it
- GIVEN a root with no `wiring`, components `b` and `c`, a live change that removed `b/gone`, and `pending` placing `b/gone` on `c` for a marker outside a test at `crates/c/lib.rs:4`, no file of `b` citing it
- WHEN the prompt for `c` is written from that pending list
- THEN it names `crates/c/lib.rs:4` as a marker to take out, and says the code `b/gone` needed is `b`'s to remove and to add a `deferred` item naming `b` and `b/gone` unless the backlog holds one

#### Scenario: stale marker in another component's files
- GIVEN a root that sets no `wiring`, and component `b` whose requirement `b/x` is at version `22222222`, cited at `crates/b/t.rs:3` at `22222222` and at `crates/c/t.rs:8` at `11111111`, which `specs/b/SPEC.md` had at an earlier commit
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` as `behind` with a diff from the `11111111` text to the current, and names `crates/c/t.rs:8` as a marker to re-cite, and not `crates/b/t.rs:3`

#### Scenario: stale marker in a file the build may not change
- GIVEN the same, with the root's `wiring: []` and `crates/c/t.rs` owned by component `c`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` as `behind`, names `crates/c/t.rs:8` apart as a marker in `c`'s file that `clm land` refuses this build to change, says to add a `question` item that blocks `b/x`, and doesn't say to re-cite it

#### Scenario: blocked requirement left out
- GIVEN component `b` with pending requirements `b/x` and `b/y`, `b/y` in the `blocks` of `backlog/q.md`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` to build, and names `b/y` and `backlog/q` as blocked

#### Scenario: needing requirement left out
- GIVEN component `b` with pending requirements `b/x` and `b/y`, `b/y` needing uncited `c/z`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/x` to build, and names `b/y` as blocked by `c/z`

#### Scenario: resumed branch behind main
- GIVEN an earlier build's local branch `impl/b` containing `origin/impl/b`, with a commit `origin/main` lacks, cut when `b/y` needed uncited `c/z`, and a commit on `origin/main` since then adding a test that cites `c/z` at its current version; and a checkout `clm run` runs in that is at the older commit of `main`
- WHEN `clm run b` runs
- THEN the prompt it writes lists `b/y` to build and doesn't name it as blocked

#### Scenario: resumed branch that conflicts with main
- GIVEN the resumed branch of "resumed branch behind main", whose commit and a later commit on `origin/main` change the same line of one file
- WHEN `clm run b` runs
- THEN it exits 0, and the prompt it writes lists `b/y` to build, says its lists were read from `origin/main` because the branch conflicts with it, and names `clm pending b` after the first `clm rebase`

#### Scenario: a listed test of a requirement that stays
- GIVEN requirement `a/x` at a new version, tests `keeps` and `dup` in `a`'s file `t.rs`, each citing it at its old one, and a live change whose `deleted-tests.txt` holds `DELETE t.rs::dup`
- WHEN `clm run a` runs
- THEN the prompt lists `a/x` as `behind`, `dup` as a test to delete, and `keeps`'s marker, and not `dup`'s, as one to re-cite

#### Needs
- model/parse-backlog
- land/deleted-tests-approved
- clm/renamed-commands
- check/obsolete-tests-elsewhere
- check.coverage/move-targets
- check.coverage/pending-report
- check.coverage/wait-lines
- check.coverage/state-names
