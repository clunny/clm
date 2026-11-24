---
title: "The panel's suite asserts the marketplace and clm's hooks file until plugin's suite does"
kind: deferred
components: [plugin.panel, plugin]
origin: 20261123-wave-5-plugin-by-the-rule
---

`20261123-wave-5-plugin-by-the-rule` moved the marketplace clause (`clm` then `clm-panel`) and the clause that `plugin/hooks/hooks.json` names no hooks module from `plugin.panel/panel-package` to `plugin/package` and `plugin/hooks-wired`. `plugin.panel`'s build landed before `plugin`'s suite asserted them, so `crates/clm/tests/plugin_panel_suite/main.rs::panel_hooks_module` keeps both asserts in the meantime, so neither goes unchecked.

Once `crates/clm/tests/plugin_suite/main.rs` asserts that the marketplace lists exactly `clm` (`./plugin`) then `clm-panel` (`./plugin-panel`), and that `plugin/hooks/hooks.json` holds no `modules`, `plugin.panel`'s next build drops the two asserts from `panel_hooks_module` and deletes this item with a `Resolves:` trailer (`deferred-work-resolves-in-code`).
