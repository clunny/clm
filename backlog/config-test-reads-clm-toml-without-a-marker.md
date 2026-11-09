---
title: "config's verify-here test reads the repository's clm.toml without a reads marker"
kind: bug
components: [config]
---

`crates/clm-config/tests/config.rs`'s `this_repository_verifies_the_whole_suite_after_lands` reads this repository's own `clm.toml` (`../../clm.toml`), but the file holds no `clm:reads clm.toml` marker, so a land that changes only `clm.toml`'s comments or upgrade notes, which counts as checked (`land/settings-unchanged-is-checked`), doesn't run it. `clm/doc-readers-marked`, as `20261109-init-readers-scenarios-need` widens it to every crate's tests, states the marker. The fix is the marker line in that file, which lies in `config`'s `paths`, so `clm`'s build can't add it (`path-ownership`). Found by that change's builder check.
