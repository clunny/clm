---
title: "The guide never says a test reading the project's own files needs a clm:reads marker"
kind: bug
components: [init]
---

Step: an adopting project writes a test that reads its own `ARCHITECTURE.md`, a file under `specs/`, or its `clm.toml`.

What happens: `clm land` counts a tree whose code key is checked as checked, and runs only the tests whose `clm:reads` markers name a file the branch changes (`land/selected-tests`, `doc-readers-marked`). After `20261107-an-upgrade-branch-that`, a land whose `clm.toml` changed only in comments or upgrade notes counts as checked too (`land/settings-unchanged-is-checked`). A test with no marker is then skipped, and the land can leave `main` failing. The rule is stated only in this repository's `CLAUDE.md`, decisions and specs, never under `docs/guide/` or in the plugin's skills.

Expected: a guide page (the one on `clm land` and `clm affected`, or the CI page) says that a test reading the project's own doc-only files or its `clm.toml` holds a `clm:reads <glob>` marker, as `method-in-the-plugin` asks. Found by the architecture audit of `20261107-an-upgrade-branch-that`.
