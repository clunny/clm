---
title: "The proof-is-real upgrade note doesn't say that naming wiring also switches on a check"
kind: deferred
components: [upgrade]
origin: 20261117-tests-sit-with-the-component-they-cite
---

`crates/clm-upgrade/src/migrations/20261114-proof-is-real/note.md`, not yet in a release, tells a project that naming the root's `wiring` switches on the land's refusal of a build that changes another component's files. Once `check/tests-cite-their-owner` is built as `20261117-tests-sit-with-the-component-they-cite` states it, naming `wiring` also has `clm check` report a test marker that cites a component other than its file's owner, in every file a component owns. A project that names `wiring` while its tests still share files then fails `clm check`, and the note doesn't warn of it.

Due when `check/tests-cite-their-owner` is current and before the next release: add the sentence to the note, and to the test of its text in `crates/clm-upgrade/src/shipped_tests.rs`. The note states no behaviour of `upgrade`'s that a requirement would have to change, so this is a fix under `clm claim upgrade --fix`.
