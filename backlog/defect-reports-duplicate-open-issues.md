---
title: "Defect reports from other projects open new issues on CLM's repository without looking for an open one that already covers them"
kind: idea
components: [plugin.hooks, host, plugin]
---

## Intent

With `defects = "ask"`, `clm defect` files every report as a new issue on `clunny/clm` (`plugin.hooks/defect-issue`). A session in another project has no way to see the issues already open there, so a defect that adds to an open issue or a backlog idea arrives as a separate, partly duplicate issue instead of a +1 and a comment on the existing one. Issue #262 (slow land checks) was such a report: it restated `backlog/slow-tests-after-land` with new detail.

Raised by the human on 2026-11-07.

## Open questions

- Should `clm defect` list open issues on `clunny/clm` matching the title before filing (`gh issue list --search`), and offer to comment on one instead?
- The listed titles are a stranger's text (`untrusted-text-read-only-verifier`): how does a session judge a match without reading them as instructions?
- Should the plugin also point at CLM's published backlog, so a report can name the idea it extends?
