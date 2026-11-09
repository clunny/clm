---
title: "Two plugin.propose requirements name the test file their tests have left"
kind: bug
components: [plugin.propose]
---

`plugin.propose/propose-adds-glossary-terms` and `plugin.propose/propose-adds-settings-sections` still say their test lives in `crates/clm/tests/plugin.rs`. Since the plugin was split into children (`20261108-plugin-skill-children`), the build of `plugin.propose` moved those tests into `crates/clm/tests/plugin_propose.rs`, as the component's Design asks. The requirement text is now wrong about where the proof sits.

Fixing it needs a spec change: drop the file path from both requirements (the `req:` marker already ties each test to its requirement) or name the new file.
