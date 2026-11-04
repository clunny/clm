---
title: "A Rust file's inline test module counts where its markers cite; the code above it counts toward its owner"
date: 2026-10-27
status: accepted
---

`tests-count-where-cited` counts a file holding `req:` markers toward the nodes they cite, not toward the node whose `paths` hold it. A Rust source file with a `#[cfg(test)]` module is one file, so when its inline tests cite another node (a parent, for a child crate's file), the file's production code moved with them, and the owning node's count lost it (`backlog/size-counts-inline-tests-with-their-source`).

Now a `.rs` file with a line that is exactly `#[cfg(test)]` is two parts for `clm size`: from the first such line to the end, and what comes before. Each part is counted as a file is: toward the nodes its markers cite, or, with no markers, toward the node whose `paths` hold the file. Rust convention puts the test module last, so the line splits source from tests without a parser, and the measure stays deterministic and cheap.

Rejected:
- **Counting every file under `paths` toward its owner as well as toward the nodes it cites.** It would put `crates/clm/tests/*.rs` back on the root, the miscount `tests-count-where-cited` fixed.
- **Parsing the module's extent** with tree-sitter. `clm size` would gain a C dependency for bytes that the simple rule already gets nearly right; a file that puts code after its test module counts that code as test, which is the rare case.
- **Other languages.** TypeScript and Python keep tests in their own files by convention, so no part rule is needed for them.
