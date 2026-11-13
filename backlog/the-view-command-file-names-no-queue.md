---
title: "The /clm:view fallback command file lists no queue name and still shows Flight for no name"
kind: bug
components: [plugin]
---

## Intent

`plugin/commands/view.md`, which the model reads when `clm-panel` isn't loaded, names `status`, `spec`, `flight` and `blockers` and shows `clm view flight` for no name. The panel now answers `queue` too (`plugin.panel/view-command-in-chat`) and opens on Queue, so without the panel `/clm:view queue` is called an unknown name. The file is `plugin`'s, so a `plugin` build adds `queue` to its names and shows `clm view queue` for no name.
