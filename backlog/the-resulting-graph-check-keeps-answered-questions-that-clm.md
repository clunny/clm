---
title: "The resulting-graph check keeps answered questions that clm apply deletes"
kind: bug
components: [apply]
origin: 20261113-builder-check-text-lives
---

`clm check` runs `apply/resulting-graph` on a proposed change before `clm apply`. That check models the graph the change would leave, but it keeps the `kind: question` items in the change's `resolves`, which `clm apply` deletes (`apply-deletes-answered-questions`). So when a question item's `blocks` names a requirement the change removes, the check fails with `model/parse-backlog: blocks names <req>, which isn't a requirement or one a live change adds`, although after apply the item is gone. Seen drafting `20261113-builder-check-text-lives`: the question `builder-check-text-lives-in-the-propose-skill-file` blocked `plugin/builder-check`, which the change removes. The workaround was to drop the item's `blocks` line on the spec branch before applying. The resulting graph should leave out the question items in `resolves`, as apply does.
