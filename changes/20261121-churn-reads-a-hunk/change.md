---
status: proposed
resolves: [churn-reads-a-hunk-s-removed-and-added-lines-as-a-file-header]
---

## Intent

`clm health`'s churn signal reads a patch's file headers only before a hunk, so a removed line starting `-- ` followed by an added line starting `++ ` inside a hunk no longer reads as a header, and the markers on the added lines after them are counted.
