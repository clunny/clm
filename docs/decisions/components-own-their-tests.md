---
title: "A component owns the tests that prove it, through tests globs, and reaches the entry point through test_depends_on"
date: 2026-11-12
status: accepted
---

A test proves a component's requirement wherever it sits (`tests-count-where-cited`), but `path-ownership` went by `paths`, so a test kept in another component's files could be re-cited but never written there. Two projects adopting CLM met the same wall in their first build sessions. Their existing suites shared a harness, and the tests imported the app's entry point, which sits above every feature, so recovery gave the whole suite to one component, listing its files one by one. From then on every build that needed a test either wrote into the suite's files, against `path-ownership`, or created a file no component owned, which only a spec edit could claim and which `gate/classify` refused beside code as `mixed`. Builds stopped, and some got around the checks: a test's import moved inside a function, a request to rewrite a branch's history, a glob that hid the breach. This repository has the same shape: `crates/clm/tests/*.rs` belongs to the root because Cargo gives the binary only to the root package's tests, while each file proves another component.

The upward import is the real cause, and build systems have long had the answer: a test-only dependency (Cargo's dev-dependencies, Maven's test scope). So:

- **`tests` globs** name a component's test files, which it owns wherever they sit. Inside another component's `paths` they win, so a suite directory can belong to a harness component while each feature owns `tests/<feature>/**` or `tests/test_<feature>*.py` in it. Two components' `tests` never overlap (`check/paths-disjoint`). A new test file under the component's own glob is owned the moment it is written, with no spec edit.
- **`test_depends_on`** names the components the test files may import beyond `depends_on`, such as the one that holds the entry point, the binary or a shared harness. `undeclared-imports` judges a test file against both lists, and production code against `depends_on` alone. It forms no cycle, since nothing imports a test.
- **A test file a `tests` glob owns cites only its owner** (`check/tests-cite-their-owner`), so ownership and citation agree and no two builds edit one test file. A file only `paths` own isn't judged, so a project moves to `tests` a glob at a time.
- **Recovery writes `tests` globs**, by directory or file-name pattern, never file lists, and prefers directory globs in `paths` too (`plugin.init/init-skill-gives-tests-their-component`).

Rejected:
- **A "holding component" that builds may write into and that may only shrink**, as one of the reporting projects proposed. It names the problem but keeps the breach: builds still edit a file they don't own, and two builds can edit the same file. And "only shrinks" contradicts "builds add to it".
- **Exclusion globs in `paths`** (`!tests/a/**`). They express the carve-out from the owner's side, so every new feature edits the harness's frontmatter, and the feature's own frontmatter doesn't say what it owns.
- **A further `path-ownership` exception for tests added to a shared suite file.** Exceptions had grown from one to five in two weeks, each covering only its case (`redesign-before-workaround`).
- **Letting test files import anything.** A build's context holds what its component depends on (`ancestor-scoped-context`). A declared test dependency says which Contracts its tests may lean on.
- **Clean Architecture as a principle of every CLM project.** It would let most tests run against fakes with no entry point, but CLM already enforces the dependency rule that matters (`depends_on` and `undeclared-imports`), end-to-end tests still need the composition root, and its layering is indirection that a small CLI or front end doesn't need. A project may still adopt it in its own `ARCHITECTURE.md`.

This repository's own `crates/clm/tests/` keeps relying on the transitional re-citing exception until its files get `tests` globs (`backlog/clm-s-cli-tests-belong-to-the-root`).
