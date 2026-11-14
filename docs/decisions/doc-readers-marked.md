---
title: "A test that reads the repository's own doc-only files says so with `clm:reads`"
date: 2026-11-04
status: accepted
---

`code_key` leaves the doc-only paths out (`land-locally` lists them), so a `checked` record for a tree's code key lets a spec land skip `[land] check`. That is safe only while no test reads those paths, and some do: tests read `specs/`, the root documents (`ARCHITECTURE.md`, `CAPABILITIES.md`, `CLAUDE.md`, `README.md`) and the guide's pages under `docs/`. A spec branch whose text broke such a test landed on checks alone and left `main` failing `cargo test` (`backlog/a-spec-branch-s-land-skips-the-code-tests-that-read-spec-text-so`).

A test that reads files of the repository's own tree under doc-only paths now holds a `clm:reads <glob>` marker for them. `clm affected` selects a file whose marker's glob matches a changed path, doc-only paths included, and `clm affected --readers` prints only those files. `clm land`, on a tree whose code key is checked, asks `--readers` and runs `[land] check` with that selection when it names a file, in the land tree under the lock so the target stays warm; with none it skips the check as before.

**Why a marker.** It lives in the test, beside what it describes, as `req:` and `clm:remove-by` do, and it is plain text, so `clm affected` stays free of any test runner or language. A spec land pays only for the tests that read what it changed.

**Rejected.**
- **Counting the read paths as code.** Every spec land changes `specs/`, so every one would run the whole suite and take the land lock, serialising drafts behind builds. `docs/guide/` and `docs/spec-format.md` are doc-only for the same reason, and the tests that read them hold markers (`guide-pages-are-doc-only`).
- **A second check command for spec lands** (`[land] spec_check`), listing the reading tests in `clm.toml`. The list sits away from the tests and drifts as they change, which the marker doesn't.
- **Finding the readers automatically.** Which files a test opens isn't knowable from its text in general without knowing the language and the runner, which `check.affected` doesn't. A reader built without its marker fails as before, on the next full run; the readers this repository has are covered by `clm`'s marker requirement, which covers every crate's tests and is proven on the root through the real `readers`, and by `web.site`'s for the site's tests (`doc-readers-one-rule`).
