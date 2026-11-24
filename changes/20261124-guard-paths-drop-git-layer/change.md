---
status: proposed
resolves: [plugin-hooks-guard-s-paths-still-name-a-test-file-its-build-deletes]
---

## Intent

`plugin.hooks.guard`'s `paths` drops `crates/clm-guard/tests/git_layer.rs`. Wave 5's build of the guard deleted that file with the test `deleted-tests.txt` named. The entry was kept on wave 5's spec branch only because dropping it while the file still existed would have left the file in no component's `paths` (`check/unowned-files`).

## Outcome

- The guard's frontmatter lists its four remaining globs. No requirement, scenario, Contract or Design changes, and nothing is cut.
- `backlog/plugin-hooks-guard-s-paths-still-name-a-test-file-its-build-deletes` goes with this change.
