---
title: "`clm land` names an empty owner for a file outside the CLM root"
kind: bug
components: [land]
---

Since `gate/paths-under-the-root`, `clm_gate::foreign_files` returns a file a build changes outside the CLM root (named `../<path>`) with an empty owner. `land/build-holds-own-files` formats every foreign file the same way (`crates/clm-land/src/own_files.rs`), so `clm land impl/a` on a branch that changes `../b` prints:

```
../b: land/build-holds-own-files: owned by ``; impl/a changes its own files, the wiring and a capability test it serves. Build this in ``, or leave the requirement unbuilt and add a question item that blocks it
```

The refusal itself is right; the message points the session to an empty component. For an empty owner it should say the file is outside the CLM root, which no build of the root's components may change.

Found by the code review of `impl/gate`.
