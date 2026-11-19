---
title: "clm coverage --deleted-tests names base files from the repository root and head files from the CLM root"
kind: bug
components: [check]
origin: 20261119-the-land-and-clm
---

Step: drafting `20261119-the-land-and-clm`, whose builder check read `clm_check::lists::deleted_tests_lines` (`crates/clm-check/src/lists.rs`).

What happens: it reads the base with `clm_git::tree_blobs`, which lists with `git ls-tree --full-tree`, so its paths start at the repository root, and the head with `read_files`, whose `git ls-files` paths start at the CLM root. When the CLM root is a subdirectory, every base file's path carries the prefix and no head file's does. Counts by bare name still balance, but `deleted_tests`' rename match looks in "the head's text of its file", which never exists, so a test renamed in its own file citing every requirement it cited is listed as gone by `clm coverage --deleted-tests` while `clm land` (which strips `rev-parse --show-prefix`) lands it.

Expected: both sides named from the CLM root, as `clm land` names them, so the two give one verdict in a repository whose CLM root is a subdirectory too.
