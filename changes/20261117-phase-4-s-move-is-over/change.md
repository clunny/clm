---
status: applied
resolves: [routes-only-states-help-for-clap-s-own-help-and-no-exception-for-unreadable-input, own-files-does-not-say-how-a-capability-citation-is-read, tests-cite-their-owner-judges-wiring-and-source-files-as-tests]
---

## Intent

Phase 4 of the plan in `backlog/a-passing-check-proves-less-than-the-spec-claims.md`, step 7, the first closing branch. `gate/own-files` is built without the clause "a file that cites the component", so the move of every test into its component's files is over. This change takes out of the specs and the guide what still describes the move as under way, and rewords three requirements where their builds found the text and the code apart. The human answered each of the three on 2026-11-17. One behaviour changes, in `clm check`: the wording of one violation line.

## Problem

- **Three requirements say other than the code does**, each found by a build's review. `clm/routes-only` asks `--help` of the parser's own `help` and allows the root no error of its own. `gate/own-files` doesn't say that a capability test's citation is read as a marker. `check/tests-cite-their-owner` judges the root's wiring files, which every build may edit, and its message calls every file a test.
- **The specs and the guide still describe the move.** The root's Design has two paragraphs on how a build moves a test; five Designs name a `moved-tests.txt` of a change that is archived; the guide keeps the exception "A test that cites the component", which no longer exists; and three Designs name files that have moved.
- **Six requirements of `view` and `check.coverage` need requirements of `view.repo` and `check`** only because `20261117-clauses-only-a-command-shows` had to order its move.

## Approaches

For each reword, the text follows the code (taken) or the code follows the text. The decisions `commands-live-in-their-crates` and `own-files-at-the-gate` say why the text moves in each case.

The item that asked for this branch also asked for a `tests` glob on the root's test files and on the four suite directories. Not taken: the suites are already in their components' `paths`, and since `check/tests-cite-their-owner` judges every owned file once the root names its `wiring`, a glob would add nothing.

## Outcome

- `clm/routes-only`: the root prints an error of its own only for a standard input or a current directory the process can't read; "every command routes" leaves out the parser's `help`.
- `gate/own-files`: a capability test's citation is read as a marker, by its whole ID.
- `check/tests-cite-their-owner`: a file the root's `wiring` globs match isn't judged unless a `tests` glob owns it; the message reads "the file is `a`'s; move the test to `b`'s files".
- `view`'s `status-report`, `view-command`, `spec-view` and `view-library`, and `check.coverage`'s `pending-report` and `status-report`, lose the needs that ordered the move. Their text is otherwise unchanged.
- Designs: the root's two paragraphs become one, **Where tests sit**; `land`, `land.approval`, `web`, `web.views` and `web.worker` lose their sentence on `moved-tests.txt`; `ci`, `config`, `plugin.agents` and `check.unreferenced` name files as they are. `check`'s Design still gives `crates/clm/tests/run.rs` as its example of a test file that counts toward the component it cites; it is left for `20261115-check-passes-on-no-coverage`, whose delta is written against that Design.
- The guide loses the exception "A test that cites the component" and says a project gives its tests to their components before it names its `wiring`.

## Constraints

- **Each reworded requirement goes `behind`** and is brought up by a build that re-cites its tests: `clm`, `gate`, `check`, `view` and `check.coverage`. Only `check`'s has code to change, the wiring files and the message.
- **Left for phase 5's regrouping**, since each reword would cost a build for no change in behaviour: the 23 `commands-in-crate` requirements, and requirements of `check.coverage`, `view`, `view.repo`, `init` and `upgrade`, which say where a test "sat" or "is no longer left".
- **Left as code fixes**, in `backlog/the-propose-skill-and-the-run-prompt-still-say-a-test-may-sit-in-another-component-s-files`, which is what remains of the item that asked for this branch: one sentence of the propose skill and one of the run prompt's fixed text. The plan item's phase 5 holds the list of what is left for the regrouping.
- `clm/routes-only` drops its needs on `view` and `check.coverage`, and `gate/own-files` its needs on `clm/routes-only` and `check/tests-cite-their-owner`: each only ordered the move, and each would now chain four builds that only re-cite.
- The message's wording changes for a project that sets `tests` globs too. It is a violation line a person reads, and no file or format a project holds, so it ships no migration.

## Open questions

None.
