---
title: "scan for Budgets collides with clm_check::Budgets in main.rs"
kind: workaround
components: [plugin.hooks.guard]
requirement: plugin.hooks.guard/hook-budgets@0a955c81
---

The scenario "nothing shortens the hook's budgets" says no file under crates/clm/src names Budgets, but crates/clm/src/main.rs already names clm_check::Budgets (component_size). The test scans for guard_within, and for Budgets only on a line naming clm_guard, so it proves the intent without a false failure. The scenario should say clm_guard's Budgets.
