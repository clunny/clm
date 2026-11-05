---
title: "A test can't hold a requirement ID as plain text without the marker scanner reading it as a citation"
kind: bug
components: [check.coverage, check]
issues: [92]
---

A test that searches other files for citations of a requirement holds the marker prefix, a colon and `<component>/<slug>` as a search string. The marker scanner reads it as a citation without a version, which keeps the requirement `behind` and fails the check. The only way round it is to split the string so the literal never appears whole, which obscures the test. That workaround is written only in this repository's `CLAUDE.md` (a gotcha about `dangling-markers`); `docs/spec-format.md` and `docs/guide/` don't tell adopters about it.

Expected: a way to write such a string that the scanner doesn't count, such as an escape or an ignore comment it honours, or at least a documented rule in `docs/spec-format.md` and the guide. Skipping unversioned forms when the file also holds versioned markers is ruled out: `stale-markers-keep-behind` keeps a marker without a version `behind` even beside one at the current version.
