---
title: "clm rename breaks five of its own requirements at the edges"
kind: bug
components: [rename]
origin: 20261119-wave-1-by-the-rule
---

Found by the second builder check of wave 1 of phase 5 (2026-11-19), each read from the code and none run. The requirements were left as they are.

- `crates/clm-rename/src/build.rs:56` and `:179-200`: an I/O failure after `clm_apply::apply` has run, or in the archive step, leaves `specs/` and the change's status written, against `rename/refusals` "writing nothing" (with `backlog/an-i-o-failure-while-clm-apply-writes-leaves-the-files-already`).
- `build.rs:244-263`: the marker rewrite edits every file the coverage scan reads, non-Markdown files under `changes/archive/` and `docs/decisions/` among them, against `rename/references` "SHALL edit no file under" them.
- `build.rs:489`, `:682` and `text.rs:75-84`: the frontmatter key `successor`, which holds a change ID, is rewritten as a reference, and a block-style YAML list never is, against `rename/references`.
- `build.rs:335`: `rename/flag-rerecord` matches the text `requirement: <id>@<version>` literally, so a quoted or differently spaced value keeps its old version.
- `build.rs:359-396`: `rename/other-deltas` rebases an element of a component that exists only after the rename, which it says is judged against the graph before it.
- `text.rs:116-125`: a line starting `#` inside a code fence is read as a heading, and the `#### Needs` state it sets outlasts the fence.
