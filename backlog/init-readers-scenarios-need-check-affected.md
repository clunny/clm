---
title: "init's doc-readers-marked scenarios can't be proven through readers"
kind: question
components: [init, check.affected]
---

The scenarios of `init/doc-readers-marked` say `readers` selects `crates/clm-init/src/guide.rs` and the others for a changed doc. The spec-auditor found no `init` test runs `clm-affected`'s `readers` on them (`clm/doc-readers-marked` does so for its own). A test in `clm-init` needs a dev-dependency on `clm-affected`, but `init` doesn't depend on `check.affected`, so `check/undeclared-imports` fails it, though the requirement has `#### Needs check.affected/affected-readers`.

Needed: a spec change giving component `init` the dependency on `check.affected`, or moving these scenarios' proof to a test under `clm`'s paths. Until then the citing test checks the markers with its own glob matching.
