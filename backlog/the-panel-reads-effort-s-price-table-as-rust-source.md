---
title: "The panel reads effort's price table as Rust source"
kind: idea
components: [effort, plugin.panel]
---

The panel's estimate of a run's cost must equal what `effort/prices` gives for the same model and tokens (`plugin.panel/build-row-cost`). The panel ships alone in `plugin-panel/` and can't read `crates/` at run time, so it carries its own copy of the table, and the only check that the two agree is a panel test that parses `crates/clm-effort/src/prices.rs` as text. That ties effort's Rust source to its present shape (effort's Design tells its build to keep each row as `("<pattern>", [<rates>])` and the fallback as `const SONNET: Rates = [...]`), so a change to how effort writes its table can break the panel's suite with no requirement changing.

A price table both read would end that: the rows in a data file of effort's (for example `crates/clm-effort/prices.json`, which effort embeds with `include_str!` and whose keys a requirement may state as a file format), and the panel's test reading that file, or the panel's copy generated from it. It needs an effort change (its `prices` requirement and Design) and a panel change (its test and Design), in steps: effort's file first, then the panel's reader, then effort's Design drops the sentence that keeps `prices.rs`'s shape.
