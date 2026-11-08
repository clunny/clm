---
title: "clm upgrade --apply writes upgraded between a table's comment and its header"
kind: bug
components: [upgrade]
---

Step: `clm upgrade --apply` in a project whose clm.toml has no top-level `upgraded` line and puts a comment line directly above its first table header (here `[land]`).

What happened: the stamp `upgraded = "<id>"` went in on the line just above the header, between the comment and the header it describes. The comment then reads as if it describes `upgraded`, and the header loses its comment. The session had to move the line by hand before committing.

Expected: the new top-level line goes above the comment block (and blank line) that leads into the first table header, so each comment stays attached to the line it describes.

Rule involved: upgrade's `--apply` requirement, which says to add `upgraded` "before the first table header" and change no other line. The current placement meets the letter of that but cuts a comment off from its header. `--decline` writes `declined_migrations` the same way, so it has the same problem.
