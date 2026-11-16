---
title: "rename's Design names a gate requirement that no longer exists"
kind: bug
components: [rename]
---

`specs/rename/SPEC.md` (Design, **Tests.**) says a test that runs the `clm` binary sits in `crates/clm/tests/rename.rs`, "as `gate`'s do (`binary-tests-apart`)". Change `20261115-builds-change-only-their-own-files` removed `gate/binary-tests-apart`, and `gate`'s Design now says all its tests are its crate's (`commands-in-crate`). The paragraph cites a removed requirement and a placement that no longer holds; a spec branch should reword it to follow `commands-live-in-their-crates` and `rename/commands-in-crate`. Found by the code review of `impl/gate`.
