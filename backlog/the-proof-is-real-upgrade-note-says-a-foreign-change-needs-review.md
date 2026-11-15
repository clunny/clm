---
title: "The proof-is-real upgrade note says a build's change to another component's file needs review"
kind: bug
components: [upgrade]
origin: 20261115-builds-change-only-their-own-files
---

`crates/clm-upgrade/src/migrations/20261114-proof-is-real/note.md`, not yet in a release, tells an adopter that a branch changing another component's files "needs review". `clm land` refuses such a build branch; no review lets it through (`land/build-holds-own-files`). The note also doesn't say that a project whose tests share files gives them to their components with `tests` globs before it names its `wiring`, which matters once the clause "a file that cites the component" is gone (`own-files-at-the-gate`).

Found by the builder check of `20261115-builds-change-only-their-own-files`. Fix the note before the release that first ships it; after that the migrations lock freezes it (`upgrade/shipped-migrations-frozen`).
