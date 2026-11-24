---
title: "view.repo's status(root, budgets) has no caller once the session-start hook uses clm status's call"
kind: deferred
components: [view.repo]
origin: 20261123-wave-5-hooks-by-the-rule
---

`clm_view_repo::status(root, budgets)` exists only for `clm-hooks`' session-start hook, and it writes what `clm status` prints beside its summary to the process's own standard error rather than a stream it is given. Wave 5 (`20261123-wave-5-hooks-by-the-rule`) moves the hook onto `status_command`, passing the hook call's error stream and the budgets its caller gives (`plugin.hooks/session-status`, `plugin.hooks/commands-in-crate`). Once that build of `plugin.hooks` has landed, nothing calls `status`, so it is dead (`no-dead-code`).

Take this up after `plugin.hooks/session-status` is current on `main`. Remove `status` from `clm-view-repo` (its private `status_to` stays for `status_command`), and on a spec branch remove the Contract's words for it: "`status(root, budgets)`" among the calls that load the graph, and "`status` writes it to the process's standard error". No requirement of `view.repo` names `status`, so no requirement sentence goes and the removal needs no yes from the human. If one is found to, that sentence's removal needs the human's yes. Resolve this item in the commit that removes the call with the trailer `Resolves: backlog/view-repo-s-status-call-has-no-caller-once-the-session-start-hook-uses-clm-status-s-call`.
