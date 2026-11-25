---
status: applied
resolves: [a-marker-above-a-describe-block-counts-once-for-coverage-while]
---

## Intent

A `req:` marker above a TypeScript `describe` block was read two ways. The coverage counted it as one citation by the block, while `clm health`'s `uncited-test` and the land's `deleted_tests` read each test's own lines and found the tests inside citing nothing: for `web`, 112 of 119 `uncited-test` rows were such tests, and any one of them could be deleted with no `deleted-tests.txt` line though it was the block's proof (`backlog/a-marker-above-a-describe-block-counts-once-for-coverage-while`).

Now a marker counts only for the test whose lines hold it, the comment directly above the test included, never on a group's own lines (decision `a-marker-counts-for-its-own-test`, superseding `markers-count-in-tests`, whose other parts it restates). The coverage then reads a test's citations from the same lines `uncited_tests` and `deleted_tests` already read, so a test inside a group is cited exactly when its own lines cite something, `clm health` says so, and the land refuses its unlisted deletion.

## Approaches

- **A group's marker counts for no test (chosen).** `markers_in` keeps only the markers on a test's lines; a marker on a group's own lines joins `outside` and shows in `clm coverage`'s `uncounted` line, as a file-header marker does. `uncited_tests`, `deleted_tests` and `obsolete_tests` already leave groups out, so their rule doesn't change: only `markers-in-tests`, `test-spans`' outside list, two Contract sentences and one Design paragraph do. This is the rule wave 6 already gave `web`, `web.views` and `web.client-live` by hand (one marker on each test, none on a `describe`), made the rule everywhere.
- **A group's marker cites every test inside it.** Keeps the 68 markers above blocks counting, but `deleted_tests`' counting of gained, stripped and renamed tests would read inherited markers throughout, one marker would stand for many tests each needing its own list line, and the mutation pass would judge a block's marker by the block. Rejected: more code in the longest rule `check.coverage` holds, to keep a reading every other reader already left.
- **Failing `clm check` on a marker above a group.** Rejected as `markers-count-in-tests` rejected failing on a marker outside a test: a green project turns red on upgrading.

## Outcome

- `check.coverage/markers-in-tests`: only markers on a test's lines count, never one on a group's own lines outside its tests; the coverage, `uncited_tests` and `deleted_tests` read a test's citations from the same lines. Two scenarios added: *a marker above a group* and *a marker on each test of a group*.
- `check.coverage/test-spans`: `markers_outside_tests` returns the markers on no test's lines, a group's own lines included, so `uncounted` lists a group's marker.
- `check.coverage`'s Contract and Design, and `check`'s Contract, say the same.
- `init/migration-group-markers` (added): an `advised` note migration, `20261124-a-marker-above-a`, tells a project set up before what stops counting and how to move its markers, as `20261114-proof-is-real` did for the last tightening (`upgrades-carry-migrations`). A migration can't edit a project's tests, so it is a note. It is `init`'s first migration of its own (`migrations-live-with-init`), so `init/all-migrations`' scenario *the list today* names it.
- `check.health` changes nothing: `uncited-test` already reads `uncited_tests`, which reads a test's own lines, so once the coverage agrees its rows are true.
- Hand edits: the decision, `ARCHITECTURE.md`'s `proven-by-tests` names it in place of the superseded one, and `docs/spec-format.md` and `docs/guide/how-it-works.md` state the rule.

## Edge cases and workarounds

- On `main` today 69 markers in `web/` sit on or above a `describe` or `test.describe` (`web/e2e/*.spec.ts`, `web/src/**/*.test.ts`), and none elsewhere. Wave 6's builds of `web`, `web.views` and `web.client-live` move every one onto its tests. If `check.coverage` is built first, those markers show as `uncounted` until those builds land. 63 of them cite requirements already pending under wave 6; six are the only citations of requirements `current` today, which then read `unbuilt` until their component's wave-6 build re-cites them: `web/links-to-site` (`web/e2e/app.spec.ts:1018`), `web.views/backlog-item-page`, `web.views/backlog-taken`, `web.views/oversized-component`, `web.views/edge-direction` and `web.views/inspector-pending`. Both components are pending under wave 6 anyway, and their builds re-cite every test, so this queues no build that wasn't queued. No `#### Needs` orders it: `check.coverage` is required and those components are optional (`requirement-needs`), and `check.coverage`'s build can't move markers in their files (`path-ownership`).
- Readers of the counted markers shift with them, as `check.coverage`'s Design ("Who reads which markers") says: `check/skipped-tests` no longer reports a skipped test in a file whose only markers are on a group's lines (its own clause: a file whose every marker stands outside a test isn't reported), and `check/tests-cite-their-owner` no longer reports a group's marker, which shows as `uncounted` instead; a group's marker citing a removed requirement is `uncounted`, not `obsolete`, as a file-header marker is today. None is on `main`. Readers that already left groups out (the mutation pass's innermost test, the gate's test names, the build prompt's test lists, `deleted_tests`, `uncited_tests`) see nothing new.
- The Worker's replica keeps each file's markers as counted when it was fetched, so after a release a file it already holds keeps counting a group's marker until it changes: filed as `backlog/the-replica-keeps-marker-counts-a-scanner-change-has-made-stale`, which holds for any change to the scanner.
- A marker on a group's code outside its tests (a `beforeEach`) stops counting too, as the rule says; none is on `main`.

## What the requirements no longer state

- `check.coverage/markers-in-tests`: "only the markers on a line of a test or a group of tests" loses "or a group of tests": a marker on a group's own lines no longer counts.
- `check.coverage/test-spans`, scenario *a group above its tests*: its outcome changes under its name, `markers_outside_tests` now returning `a/h` and `a/g` where it returned `a/h` alone.
- `check.coverage/test-spans`, scenario *a Playwright group*: its outcome changes under its name, `markers_outside_tests` now returning `a/x` where it returned nothing.
- `init/all-migrations`, scenario *the list today*: its outcome changes under its name, the list now ending with `init`'s first migration, `20261124-a-marker-above-a`, where it held `upgrade`'s thirteen alone.

## Open questions

None. The architecture audit's one gap, what an adopter sees on upgrade (`upgrades-carry-migrations`), is met by `init/migration-group-markers`.
