---
priority: P1
status: done
resolves: [clm-upgrade-apply-writes-upgraded-between-a-table-s-comment-and]
---

## Intent

Make `clm upgrade --apply` and `--decline` add a new top-level line to `clm.toml` above the comment that leads into the first table header, never between that comment and its header. Today `upgrade/upgrade-apply` says only "before the first table header", so a `clm.toml` such as this repository's, whose `[land]` has a comment directly above it, gets `upgraded = "<id>"` between the comment and `[land]`: the comment then seems to describe `upgraded`, and the session has to move the line by hand. `--decline` writes `declined_migrations` through the same function, so it does the same. This resolves the bug `clm-upgrade-apply-writes-upgraded-between-a-table-s-comment-and`.

## Problem

`set_top_level` in `crates/clm-upgrade/src/commands.rs` already steps back over the blank lines above the first header, but not over comments. The requirement as written is met by today's placement, so the fix restores no behaviour a rule states: it needs the rule to say where the line goes (`code-fix-needs-a-written-rule`).

## Approaches

- **Above the header's lead-in** (chosen): step back from the first table header over the run of comment lines directly above it, then over the blank lines above that run, and insert there, followed by a blank line when none follows. A comment directly above a header is taken as that header's, which is how `clm init` and this repository write `clm.toml`. It is a small change to the one function both flags call.
- **Parse comments with a format-preserving TOML editor** (`toml_edit`): it attaches a comment to the item after it as "decor", so it would place the key correctly too, but it pulls in a dependency to replace a twenty-line function that already works line by line, and a line-level edit is what keeps every other line byte for byte (`yagni`, `reuse-first` weighed: the existing function is the reuse).
- **Append at the end of the top-level block, after the last top-level key**: wrong where the top-level block ends in a comment for the header, the same bug.

## Outcome

`upgrade/upgrade-apply` says the new line goes above the first table header's lead-in (the comment lines directly above it, then the blank lines above them) and is followed by a blank line when none follows it there, with a scenario `a commented header`. `upgrade/upgrade-note-and-decline` says `declined_migrations` is replaced or added where `--apply` adds `upgraded`, changing no other line, with a scenario `declining under a commented header`. `upgrade`'s Design gains **A new top-level line**, which states the rule once for both keys. The build changes `set_top_level` and its tests.

## Edge cases

- A comment run that reaches the top of the file with no blank line before the first header can't be told from that header's own comment, so the new line goes first in the file, above it. A file comment followed by a blank line keeps its place, since the lead-in stops at the comment run directly above the header.
- Only the comment run directly above the header is its lead-in. In `# a`, a blank line, `# b`, `[land]`, the line goes directly under `# a`, before the blank line, so `# b` stays with `[land]`; `# a`, set off from the header by a blank line, is taken to describe what comes before it.

## Constraints

Every line of `clm.toml` other than the one written stays as it is, and the result parses as TOML with the key at top level.
