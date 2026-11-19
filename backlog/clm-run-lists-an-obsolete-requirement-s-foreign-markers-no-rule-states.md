---
title: "clm run still sets apart an obsolete requirement's markers in other components' files, which no rule states"
kind: deferred
components: [orchestrator.run, check.coverage]
---

`orchestrator.run/pending-prompt@54c39a40` dropped the sentence that made a build list apart, once the root sets `wiring`, the markers of its own `obsolete` requirement that sit in a file another component owns, together with the tests holding them, and add a `question` item that blocks nothing. `check.coverage/pending-report@ebf1b50f` puts such markers on the owning component's line instead. That requirement is still `behind`, so `pending` still places them on the requirement's own component.

The build of `orchestrator.run` kept that code: the obsolete branch's use of `Item::foreign` and `Item::foreign_tests` in `crates/clm-run/src/lib.rs` (`job()` and `prompt()`). Without it, a build would be told to delete or re-cite tests in a file `clm land` refuses it to edit. The tests `an_obsolete_test_in_another_components_file_is_listed_apart` in `crates/clm-run/tests/prompt.rs` and `crates/clm-run/tests/run.rs` stay too. They still cite `pending-prompt`, because `clm land` refuses to let a build lose a citing test.

Once `check.coverage/pending-report` is current, `pending` won't place such a marker on the requirement's own component. Then remove that code and both tests, with a `deleted-tests.txt` that names the tests.
