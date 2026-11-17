---
title: "Root defect tests still cite plugin.hooks defect-backlog at an old version"
kind: bug
components: [clm]
---

Change `20261117-scenarios-only-a-process-shows` split `plugin.hooks/defect-backlog`'s two mixed scenarios: plugin.hooks keeps the half it proves in `crates/clm-hooks/tests/defect.rs` (now built at 06439020: `backlog_on_a_spec_branch`, `backlog_node_a_live_change_creates`), and `clm/check-across-components` takes the `clm check` clause. But `crates/clm/tests/defect.rs` (`backlog_on_a_spec_branch`, `backlog_node_a_live_change_creates`, lines 80 and 131) still cites `req:plugin.hooks/defect-backlog@064ef91e`. The plugin.hooks build can't take those markers off: the file is the root's, and `land/build-holds-own-files` refuses it ("a marker gives a build no file"), yet its build prompt lists those lines to re-cite. Until the root's build removes the two markers, `clm pending plugin.hooks` keeps listing defect-backlog as `behind`. Fix: drop the two plugin.hooks markers from the root's test (it stays cited by `clm/check-across-components`); and the build prompt shouldn't ask a build to re-cite markers in files it may not change.
