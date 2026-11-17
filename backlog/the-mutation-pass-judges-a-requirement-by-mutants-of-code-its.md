---
title: "The mutation pass judges a requirement by mutants of code its tests never reach"
kind: bug
components: [verify, ci]
---

`verify/mutation-pass` lists every file the requirement's component owns as its `code`, and `ci/mutate-script`, when the diff gives no mutant or none is caught, tries the first `limit` (20) mutants of those files in `cargo mutants --list` order. For a requirement whose tests reach only part of the component, those are mutants of other code, which no test citing the requirement could notice, so the requirement is filed unproven though its tests are sound.

Seen on `backlog/unproven-config-eb2b740e` (`config/unreferenced-detector@f0c53e15`): the first 20 mutants of `crates/clm-config/src/` are `Display` impls and getters (`Defects`, `Host`, `Approval`, `Land::cap_gb`, `Settings::list`, ...). The requirement's code is data, `clm.toml`'s `unreferenced-public-items` detector, which no Rust mutation tool changes; the Rust its tests reach is `detectors()`, whose `replace detectors -> Ok(vec![])` mutant `unreferenced_detector_is_named_here` catches, and nine hand-made breaks of the `clm.toml` detector each fail a named citing test. `backlog/unproven-clm-eb2b740e` (`clm/doc-readers-marked`, a rule about which tests carry `clm:reads` markers) looks like the same case.

The build deleted the item as already sound; the next pass that finds its tests changed would file it again. What's needed is a spec change: choose the further mutants among the code the citing tests reach (the coverage the pass already writes could say which), and judge a requirement whose rule lives in a data or text file as not-judged rather than unproven.
