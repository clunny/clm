---
title: "A split's cut leaves imports running one way, checked with `clm survey --nodes` before it is offered or written"
date: 2026-10-31
status: accepted
---

`nested-splits` says how a node is split (the carved-out part becomes its child) and that `paths` stay disjoint. It says nothing about which way imports may run across the cut, and every child in this repository so far was its own crate, so the gap never showed. A triage session then recommended, and the human chose, splits that carved a child out of some files of one crate (`web.worker.review` as four modules of `crates/clm-web`). Such a split can't pass `clm check`: the crate root's `mod` lines and the router's calls are import edges from the parent's files to the child's, and the child's `use crate::...` lines are edges back, so `check/undeclared-imports` fails one way and, once the child is in the parent's `depends_on`, `model/dependency-dag` reports the cycle the other. The draft had to leave the item, which cost a draft session and a second triage round before the human chose a cut that works.

So a split's cut must leave imports running one way: the child below the parent, or what both halves share in a layer of its own, a node below both. Inside one crate or package, files can stay where they are only when the carved-out ones use none of the rest (`view.repo`, `repo.rs` and `at.rs` of `clm-view`, passes); a carved-out part that uses the crate's other modules, as most do, goes into a crate or package of its own, with what both halves share in a layer below both or in the child.

The check already exists (`reuse-first`): `clm survey --nodes <file>` reads a node map and reports each `survey/undeclared-edge` and `survey/cycle` from the same import edges `clm check` uses. A cut is written as a map of the node's halves, each with the `paths` and `depends_on` the split gives it, with `outside: ["**"]` so the rest of the code is neither reported uncovered nor judged; edges to files outside every node are skipped. It runs against the tree as it is, before any file moves, and that is enough: whether the files can move into a crate of their own is exactly whether their imports already run one way. Tried on this repository: the four-module `web.worker.review` cut reports `survey/undeclared-edge` lines (`approve.rs` imports `access.rs`, `gh.rs`, `http.rs`...), and the `view.repo` cut exits 0. `propose` runs it before writing a split's deltas, and `triage` before recommending or offering a split to the human, offering only cuts it accepts.

Rejected:
- **Leave it to `clm check` on the draft.** It catches the cut, but only after the human has chosen it and a draft session has been spent on it.
- **Allow cycles between a parent and its child.** `depends_on` is a DAG so that the queue and builds have an order; a parent and child that import each other are one unit to build, which is what a node is.
- **A new command for checking a cut.** `clm survey --nodes` already does it.
