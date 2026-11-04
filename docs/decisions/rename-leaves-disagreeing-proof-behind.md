---
title: "clm rename leaves a reworded requirement behind when its component's code still names the ID it rewrote"
date: 2026-11-03
status: accepted
---

`rename-by-replay` re-cites a reworded requirement's markers at its new version, so a rename leaves no build behind it for marker strings. That holds while the rewording changes nothing the code says. It fails when `references` rewrote an ID in the requirement's text that the code also names: in `20261103-rename-node-to-component-2`, renaming `architecture/slim-nodes` to `slim-components` re-cited `plugin/propose-nests-splits`, whose text then said the propose skill cites `slim-components`, while the skill and its test still said `slim-nodes`. The marker claimed a proof the code no longer gave, and nothing was left pending to fix it.

So `clm rename` keeps such a marker at its old version when a `still names` leftover of an ID or slug it rewrote in that requirement's text lies in the requirement's component's files (by `paths`, `check.coverage`'s `owner`) or in a file holding one of the requirement's markers. The requirement goes `behind`, and its component's build brings the code to the new name. Every other reworded requirement is re-cited as before. The leftovers report also names a renamed requirement's or entry's old slug written as a code span's whole text, the form CLM's prose cites an entry by, when no other rule or decision shares it, as `references` rewrites it in spec text.

The rule is a function of the base, like every other edit `rename` makes, so the replay still reproduces the branch.

Rejected:
- **Only reporting the leftover.** The `still names` line is advice the author may miss; the marker stays a false proof on `main`, which `proven-by-tests` forbids.
- **Leaving every reworded requirement behind.** That is `rename-by-replay`'s rejected "leaving markers to the builds": a build per component for words the code never said.
- **Holding a marker back on a term's leftover too.** A word in code can be a sense the swap didn't mean (`component-words` keeps several), so a term swap would send whole components back to build over comments. An old ID names one rule, so its mention disagrees for certain; the author judges a term's from the `still names` lines.
- **Rewriting the code's mentions.** Editing skills and code is the build's work, outside spec text (`rename-by-replay`'s scope).
- **Searching every whole-word mention of the old slug.** A one-word slug (`finish`) would report every use of the word; a code span is how CLM cites a rule by its bare name.
