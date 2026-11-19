---
title: "clm run still sets apart an obsolete requirement's markers in other components' files, which no rule states"
kind: deferred
components: [orchestrator.run, check.coverage]
---

`orchestrator.run/pending-prompt` no longer states that a build lists apart, once the root sets `wiring`, the markers of its own `obsolete` requirement that sit in a file another component owns, with the tests holding them. `clm-run` still does it: the obsolete branch's use of `Item::foreign` and `Item::foreign_tests` in `crates/clm-run/src/lib.rs` (`job()` and `prompt()`), which a small branch marks `clm:remove-by 20261119-an-obsolete-marker-moves-in-three-steps` once that change is on `main`. It stays until `check.coverage/pending-report` is current, since until then `pending` still places such a marker on the requirement's own component (`a-wait-names-a-line`).

The build of `orchestrator.run/pending-prompt` that follows `check.coverage`'s removes that code and its fixed sentences, and deletes `crates/clm-run/tests/prompt.rs::an_obsolete_test_in_another_components_file_is_listed_apart`, which `changes/20261119-an-obsolete-marker-moves-in-three-steps/deleted-tests.txt` names. `Item::foreign` stays for a `behind` marker. The build deletes this item with `Resolves: backlog/clm-run-lists-an-obsolete-requirement-s-foreign-markers-no-rule-states`.
