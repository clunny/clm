---
status: applied
resolves: [claude-md-edit-plus-its-test-classify-mixed]
---

# CLAUDE.md edit plus its test: already fixed

## Intent

Deletes `backlog/claude-md-edit-plus-its-test-classify-mixed`, which is already fixed on `main` by the route the item itself proposed. The `CLAUDE.md` edit landed on its own branch through the small-change route (`claude-md-clm-branch-push`, commit f67d236, "Name clm branch push at the remaining push steps in CLAUDE.md"). Then the test landed on `impl/clm` (commit c20756b, "Test CLAUDE.md pushes through clm branch push"): `crates/clm/tests/claude_md.rs` cites `clm/claude-md-pushes-with-clm` at its current version, and `clm pending clm` no longer lists it.

The rest of the item is intended behaviour. `gate/classify` counts `CLAUDE.md` as a spec path, so an edit to it plus code makes a branch `mixed`. That split is what `CLAUDE.md`'s doc-only route states: a hand edit to `CLAUDE.md` lands on its own branch, and the code that tests it follows on a build. So nothing in `gate` changes.
