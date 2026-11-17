---
title: "The mutation pass files unproven on a rule whose code is data in other components' files"
kind: bug
components: [verify]
---

`verify/mutation-pass` has filed `unproven` on `clm/doc-readers-marked` twice: `backlog/unproven-clm-eed96364` (at `@3125f2de`) and `backlog/unproven-clm-09d807cc` (at `@a5e3c957`, after the land `09d807cc`). Both times a build judged the tests sound and deleted the item with a `Resolves:` trailer (`0dff6fcd`, and `0144210f` on `impl/clm`). The second filing was due by the spec, since `14f04f5a` and `65e7b1dd` changed `crates/clm/tests/doc_readers.rs` in more than its markers, so it is not a lost judgement; it will happen again whenever those tests change.

The cause: the rule's code is data, the `clm:reads` markers in test files of many crates (`clm-init`, `clm-model`, `clm-context`, `clm-config` and the root's), which the plan's `code` lines (the files `clm` owns) mostly don't hold, and which a mutation tool of Rust code doesn't mutate even where they are listed, since they are comments. The tests run `clm-affected`'s `readers`, none of the root's code, so the pass's rule "a requirement whose tests run none of its code, as an empty test, still reads as unproven" files them, and only the clause about tests that "run lines of its code but none a mutant changes" was written for data rules (`config/unreferenced-detector`). The build that deleted `09d807cc` ran 22 hand mutants of the markers the scenarios name (deleting one marker, or narrowing a directory glob to one page); each failed a named test of `crates/clm/tests/doc_readers.rs`.

Each filing costs a build run and a land of no code. The pass needs a way to tell a rule whose code is data from an empty test, for instance by leaving unjudged a requirement whose citing tests read files outside the component's code (a `clm:reads` marker names them), rather than filing it. Related: `backlog/the-build-prompt-and-skills-still-tell-sessions-what-programs-now-refuse.md` records how a build settles such an item.
