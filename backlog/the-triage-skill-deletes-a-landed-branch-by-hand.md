---
title: "The triage skill deletes a landed branch by hand"
kind: deferred
components: [plugin]
origin: 20261123-wave-5-plugin-by-the-rule
---

`plugin/skills/triage/SKILL.md` step 2a (line 27) says to "delete the pushed branch after with `clm branch delete`", which `clm land` now does itself (`land/landed-branch-cleanup`). Replace it with: `clm land` deletes the pushed branch from `origin`. Prompt text; resolve with the trailer.
