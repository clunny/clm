---
title: "The cuts' graphs load the human layers, which the Design doesn't say"
kind: workaround
components: [gate]
requirement: gate/cuts@e97edf05
---

gate's Design says both graphs for `cuts` are "loaded as the code-fix check loads the base's": `specs/` and the live `change.md` files. Loaded that way, every rule whose `serves` names an architecture or capability entry fails `model/serves`, so the specs "load with a violation" at both sides and every spec branch that touches `specs/` in this repository would print the unloaded line and need review. The build loads `ARCHITECTURE.md` and `CAPABILITIES.md` beside them (crates/clm-gate/src/lib.rs, `load_graph_at(.., true)`), proven by `the_graphs_hold_the_human_layers`. The Design sentence should name the two files.
