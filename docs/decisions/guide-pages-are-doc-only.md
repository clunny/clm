---
title: "The guide's pages are doc-only; the tests that read them say so with `clm:reads`"
date: 2026-11-08
status: accepted
---

`clm guide` compiles `docs/guide/` and `docs/spec-format.md` into the binary and tests read them, which argues for counting them as code. But no component's `paths` hold them, so counted as code `check.affected` answers every edit to one with `all <file>`, and a spec branch that edits a guide page, as the propose skill asks whenever a change adds a settings key, a glossary term or guidance, runs every test: about 20 minutes on one land (`backlog/a-guide-page-edit-selects-every-test`).

So every path under `docs/` is doc-only (`ledger/doc-only`). The tests that read the guide's pages, from the tree, through `clm guide` or through the site built from them, hold `clm:reads` markers (`doc-readers-marked`), so a land that edits a page runs those tests and nothing else, as one that edits `specs/` or `ARCHITECTURE.md` does. The marker requirements of `clm` and `web.site` mark those readers (`doc-readers-one-rule`).

**Why.** The tests a guide edit can break are exactly the ones that read the guide: the compiled table's completeness and order, the links and entry-page rules, the glossary and settings coverage, the retired terms, `clm guide`'s output and the site's build. A marker in each names that, and `doc-readers-marked` already made the same trade for `specs/` and the root documents. A broken page that stops `clm-init` building (a page that isn't UTF-8, a `docs/guide/spec-format.md`) still fails the land, since `clm-init`'s own tests, which build it, read the whole guide and are selected by any page. The change is also right for adopters, whose `docs/guide/` is ordinary prose that nothing compiles.

**Rejected.**
- **Give `init` the guide in its `paths`**, as the item suggested. A guide edit would select `init` and its dependants, and `check` and the root depend on `init` and `capability` on the root, so nearly the whole Rust suite and the capability suites would still run. It would also add the guide's roughly 31,000 tokens to `init`'s code size, which is already over budget, though a build of `init` reads none of them.
- **An `[affected]` mapping from paths to components**, a new setting for one directory, where the marker already does the job (`reuse-first`).
