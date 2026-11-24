---
title: "The panel's spec runs over its size budget"
kind: idea
components: [plugin.panel]
origin: 20261123-wave-5-plugin-by-the-rule
---

`plugin.panel`'s spec was 12750 tokens against a budget of 12000 before `20261123-wave-5-plugin-by-the-rule`, and 16408 after it, which stated 28 behaviours its code had and no requirement did. Its code is 45382 against 40000. A child component for the build rows, say `plugin.panel.rows`, holding `build-rows`, `build-row-lifecycle` and `build-row-cost` with `plugin-panel/src/agents.ts` and its tests, would bring both halves under the budget (`slim-components`, `node-size-budgets`). It moves those requirements by `progress-follows-moves` and gives the child `depends_on` [effort], which the price agreement needs; the panel keeps the Queue chart and places the rows the child gives. The human may instead keep the panel whole and raise its `size_budget` in its frontmatter, so that decision sits beside the node.
