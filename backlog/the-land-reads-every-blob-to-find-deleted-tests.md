---
title: "The land's deleted-test check loads every blob of both trees, binaries included"
kind: idea
components: [land]
origin: 20261119-the-land-and-clm
---

Step: building `land/deleted-tests-approved@3f3cdc80`, whose code review flagged the cost.

What happens: `crates/clm-land/src/deleted_tests.rs` now reads every file of the merge base's and the head's trees under the CLM root, as the requirement states, through `clm_git::tree_blobs`, which holds the whole `cat-file --batch` answer in memory before the UTF-8 filter drops what isn't text. Only files whose language `check.coverage`'s `tests_in` reads (Rust, script, Python) can hold a test, so in a project with large committed binaries every land and `--check-only` run reads and buffers them twice for nothing, and any branch whose diff touches no such file still reads both trees.

Expected: the check reads only the files that can hold a test, and skips both trees when the diff touches none, with the same verdict. That needs `check.coverage` to say which paths `tests_in` reads (its families are private today), so the land's filter can't drift from the parser, and a requirement wording change in `land/deleted-tests-approved` ("every file of the two trees").
