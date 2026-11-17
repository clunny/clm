---
title: "The mutation pass files unproven again on a requirement its tests were judged sound for"
kind: bug
components: [verify]
---

`verify/mutation-pass` has now filed `unproven` on `clm/doc-readers-marked` twice: `backlog/unproven-clm-eed96364` (at `@3125f2de`) and `backlog/unproven-clm-09d807cc` (at `@a5e3c957`, after the land `09d807cc`). Each time a build judged the tests sound and deleted the item with a `Resolves:` trailer (`0dff6fcd`, and this build's commit on `impl/clm`), and the next land of `clm` brings it back.

The cause: the rule's code is data, the `clm:reads` markers in test files across many crates, while the pass mutates the component's own code (`crates/clm/src/**`), which no test of the rule reads. So every mutant survives by construction, and the pass reads the survivors as unproven tests. This build ran 22 hand mutants of the markers the scenarios name (deleting one marker, or narrowing a directory glob to one page); every one fails a named test of `crates/clm/tests/doc_readers.rs`.

Each recurrence costs a build run and a land of no code. The pass should either remember a judgement made with a `Resolves:` trailer until the requirement's version or its citing tests change, or not judge a requirement none of whose citing tests can observe the code it mutates (for example when the tests read no file the mutants change).
