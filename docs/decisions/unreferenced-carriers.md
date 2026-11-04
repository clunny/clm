---
title: "clm unreferenced follows uses through exported macros and renamed re-exports"
date: 2026-10-27
status: accepted
---

`check.unreferenced` counts an item as used when its name appears outside its own library (`public-item-detector`). Two ways for a caller to reach an item never spell the item's name outside it: an exported macro whose body calls `$crate::helper()`, and a renamed re-export `pub use inner::orig as renamed;` whose callers write `renamed`. Both were reported though they are used.

A library's *carriers* now pass uses through: a `#[macro_export]` `macro_rules!` carries every word of its body under the macro's name, and an exactly-`pub` `use` carries each `<path> as <alias>`'s last segment under the alias. A name is used from outside when it is mentioned outside the library or carried by a carrier whose own name is, followed to any depth.

- **It stays a name match.** Carriers are read from the syntax tree the detector already walks; no path is resolved, and words carried by a macro body are matched as loosely as any other mention. Like every mention rule, it can only miss dead code, never report a used item.
- **A carrier counts only when its own name is used outside.** Treating every exported macro body as an outside mention would be simpler, but it would hide the dead helpers of a macro nothing calls; tying the carried words to the macro's use keeps them checked.

Rejected:
- **Listing exported macros and re-exports as items.** It would report a different thing (an unused macro or alias), and `public-item-detector` already chose to report names where they are declared.
- **Counting every word in a library's exported macros as outside mentions.** See above: it hides the helpers of dead macros.
