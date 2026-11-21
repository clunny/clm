---
title: "Churn reads a hunk's removed and added lines as a file header"
kind: bug
components: [check.health]
---

Found by the code review of `impl/check.health` in wave 2.

`churn(root)` reads a `+++ ` line that directly follows a `--- ` line as a file header, as `check.health`'s Design states. Inside a hunk, a removed line whose text starts `-- ` followed by an added line whose text starts `++ ` shows as `--- <text>` then `+++ <text>`, so the parser takes them for a header: `path` is reset to none (or, for `++ b/x.md`, to a Markdown file), and the markers on the added lines after it, up to the next real header, are not counted. `churn-signal` states that every versioned marker in the added lines of a non-Markdown file counts, so the count comes out low.

Expected: a header is read only between a `diff --git` line and the hunk's first `@@` line (or a hunk's lines are counted off from its `@@` header), so no hunk line is read as a header. The Design's sentence on what a header is changes with it, so the fix needs a spec change.
