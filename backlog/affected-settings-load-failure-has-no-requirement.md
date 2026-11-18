---
title: "No requirement states that clm affected exits 1 when its settings can't be loaded"
kind: question
components: [check.affected]
---

`check.affected`'s Contract and Design say `clm affected` exits 1, naming the file and key, when the `[affected]` settings in `clm.toml` can't be loaded, and the code does so, but no requirement states it: `affected-base` names only a graph that can't be loaded, and `affected-all` says nothing about a setting that fails to load. The test `settings_that_cant_load_exit_1` in `crates/clm-affected/tests/command.rs` checked it while citing `affected-all`, which doesn't state it, so its marker was taken off (`code-fix-needs-a-written-rule`).

A spec change should add the clause, most naturally to `affected-base` ("and 1 naming the file and key when the settings can't be loaded") with a scenario, so its build can cite the test again. Found by the spec-auditor on `impl/check.affected`.
