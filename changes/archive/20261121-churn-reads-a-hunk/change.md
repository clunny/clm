---
status: done
resolves: [churn-reads-a-hunk-s-removed-and-added-lines-as-a-file-header]
---

## Intent

`clm health`'s churn signal reads a patch's file headers only before a hunk, so a removed line starting `-- ` followed by an added line starting `++ ` inside a hunk no longer reads as a header, and the markers on the added lines after them are counted.

## Problem

`check.health`'s Design says a `+++ ` line that directly follows a `--- ` line is a file header, and `churn(root)` reads the patch log that way. Inside a hunk, a removed line whose text starts `-- ` followed by an added line whose text starts `++ ` shows as `--- <text>` then `+++ <text>`, so the parser takes them for a header: the file is reset to none, or, for `++ b/x.md`, to a Markdown file, and the markers on the added lines after it, up to the next real header, aren't counted. `churn-signal` says every versioned marker in the added lines of a non-Markdown file counts, so the count comes out low. The Design states the faulty rule, so the fix changes it, and a scenario makes the requirement pending so a build fixes the code and proves it.

## Approaches

- **Taken: a header runs from a `diff --git ` line to the first `@@` line.** git's patch for each file starts with `diff --git `, and its hunks with `@@`; every hunk line starts with ` `, `+`, `-` or `\`, so neither line can be met inside a hunk. Two states in the existing loop, no new parsing. A `diff --git ` line also leaves no file named, so a binary file or a mode change, which has no `+++ ` line, never inherits the previous file.
- **Not taken: count each hunk's lines off from its `@@ -a,b +c,d @@` header.** Exact too, but it parses the counts and tracks two budgets per hunk for nothing the simpler bound misses.
- **Not taken: accept it.** The count is a health signal and the case is rare, but the requirement as written promises every marker, and a known-wrong count is cheap to make right.

## Outcome

- `check.health` Design, the `churn` bullet: a file's header runs from its `diff --git ` line to its first `@@` line; only there does a `+++ ` line name the file; within the hunks every `+` line is an added line whatever follows.
- `check.health/churn-signal`: new scenario "a hunk line that reads like a header". The statement is unchanged.

## Constraints

- The log's shape is already pinned by the `-c` settings and `--no-` options the Design lists; `diff --git ` and `@@` lines are part of that pinned shape.

## Edge cases and workarounds

- None known. A merge commit still shows no patch, as the Design already says.

## What the requirements no longer state

Nothing: no sentence holding `SHALL` is removed.
