---
status: applied
resolves:
  - a-guide-page-edit-selects-every-test
---

## Intent

A land that edits a guide page runs the tests that read the guide, not every test. Every path under `docs/` becomes doc-only for `ledger`'s code key, the guide's pages and `docs/spec-format.md` included, and each test that reads those pages, from the tree, through `clm guide` or through the site built from them, holds a `clm:reads` marker, as the readers of `specs/` and the root documents already do (`doc-readers-marked`, `guide-pages-are-doc-only`).

## Problem

`ledger`'s `doc_only` leaves `docs/guide/` and `docs/spec-format.md` off its list, because `clm guide` compiles them in and tests read them, so they count as code. No component's `paths` hold them, and `check.affected` answers a changed file with no owner with `all <file>`. A spec branch that edits a guide page, as the propose skill asks whenever a change adds a settings key, a glossary term or guidance, therefore runs every test at its land: the whole Rust suite and every browser and capability suite, about 20 minutes on the land that reported it.

## Approaches

- **Make the guide doc-only and mark its readers (chosen).** `doc_only` names every path under `docs/`. `check.affected` already drops doc-only paths and selects the files whose `clm:reads` globs match them, and `clm land` already runs those readers on a tree whose code was checked, so nothing in either changes. The tests a guide edit can break are the ones that read it: `clm-init`'s table, link and entry-page tests, the root's glossary, guide and retired-terms tests, `config`'s settings-guide test (marked already), and the site's build tests and the capability test that opens the quickstart. A page that stops `clm-init` building still fails the land, since `clm-init`'s own guide tests mark `docs/guide/**` and build it. It reuses `doc-readers-marked` (`reuse-first`) and is right for adopters too, whose `docs/guide/` is prose nothing compiles.
- **Give `init` the guide in its `paths`**, as the item suggested. A guide edit would select `init` and its dependants: `check` and the root depend on `init`, and `capability` on the root, so nearly every Rust test and the capability suites would still run, and the site's tests, which read the guide, wouldn't, since `web.site` depends on nothing. It would also add the guide's roughly 31,000 tokens to `init`'s code size, already over budget, though a build of `init` reads none of them. Rejected.
- **A setting mapping unowned paths to components.** A new key for one directory, where the marker already says which tests read it. Rejected (`yagni`).

## Design

- `ledger/doc-only` and `ledger/code-key` take `docs/guide/` and `docs/spec-format.md` into the doc-only list, and `ledger`'s Design says why. `legacy_doc_only` stays as it is: it keys the old lands, and no change to the live list re-checks them. The first land after the build computes a new code key for `main`, finds no `checked` record and runs the full check once.
- `ledger/doc-only` can't wait on the marker requirements: they need `check.affected/affected-readers`, which needs `ledger/doc-only`, so a need would close a cycle, and `web.site` is optional while `ledger` is required. The four builds run side by side instead. Should `ledger`'s land before a marker build, a guide-only land in between skips that component's guide tests, and a test it would have failed fails on the next full run (a code land that selects it, or `[verify]`), the risk `doc-readers-marked` already takes for a reader built without its marker. Marking a reader early costs nothing, since a guide path counts as code until `ledger`'s build lands.
- `init/doc-readers-marked` widens from `crates/clm-init/tests/` to every file of `crates/clm-init/` holding a test, since the guide's readers there are unit-test modules under `src/` (`src/guide.rs`, `src/guide/entry_pages.rs`). A marker in a source file makes `ci-check.sh` run the crate's `--lib` tests, which is what those are.
- `clm/doc-readers-marked` names the guide's pages and `docs/spec-format.md`, and a test that reads them through a command (`clm guide`, or `clm health`, which searches `docs/guide/` for decision names) as well as one that opens them.
- `web.site/doc-readers-marked` is new: every site test that builds the site with no `GUIDE_DIR`, or reads the repository's own guide, marks it (`site`, `analytics`, `github`, `blog`, `guide` and `seo` today); the scenarios name two of them. A selected `web/site/` file already makes the check script run the web and site suites.
- `ledger/doc-only`'s build changes the tests that held the guide as code: `crates/clm-ledger/tests/doc_only.rs` and `code_key.rs`, and `crates/clm/tests/ledger.rs`, whose `legacy_key` fixture (citing `ledger/verify`) tells the two lists apart by `docs/spec-format.md`, which both now name; it takes a path only the live list names, or none. `web.worker.platform`'s builds change `crates/clm-web-platform/tests/keys.rs`.
- The enforcement tests `crates/clm/tests/doc_readers.rs` and `crates/clm-init/tests/doc_readers.rs` learn the guide's paths, and `init`'s scans `src/` as well as `tests/`. A test that prints a compiled-in page (`clm guide quickstart`) marks that page's path.
- `capabilities/web/global-setup.ts` builds the site from the repository's guide for every web capability test. Its marker (`clm:reads docs/guide/**`, `docs/spec-format.md`) selects the web capability suite on a guide edit and changes nothing the test observes, so it is harness-only work on `capability`'s files, not part of any requirement's build (`path-ownership`): it lands on a small branch of its own from `main` (`small-change-no-claim`), done in this draft.
- `docs/guide/ci-and-deployment.md`, which tells adopters which paths a land treats as prose, drops its exception for the guide (`method-in-the-plugin`).
- `web.worker.platform/code-key-doc-only` and `code-key-as-ledger` keep their rule, calling `doc_only`; only their scenarios, which said the guide was code, change, and they need `ledger/doc-only`.
- `check.affected` changes nothing: it reads `doc_only`, so its selection follows.

Known edge cases: a test that reads the guide without a marker goes unrun on a guide-only land and fails on the next full run, as `doc-readers-marked` already accepts for the other doc-only paths; the builds of the three marker requirements search their components' tests for such readers.

## Open questions
- The guide page `ci-and-deployment` now says a land treats `docs/guide/` as prose, which holds only once `ledger/doc-only` is built; until then a guide-only land still runs every test, so the page errs only toward running more.
- The audit's findings were taken: the capability test's marker left `web.site`'s requirement (`path-ownership`, `capability-tests-follow-rules`), the guide page above was edited, and `guide-table-at-build-time` was amended.
