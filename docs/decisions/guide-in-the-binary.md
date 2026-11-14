---
title: "The guide stays in docs/guide/ and ships inside the clm binary, printed by clm guide"
date: 2026-10-27
status: accepted
---

`method-in-the-plugin` needs the guide in front of every adopter's session, offline and at the version they run. A skill that points at `docs/guide/shaping-the-graph.md` names a path that exists only in this repository; an adopter's session has no such file, and the installed plugin is `plugin/` alone.

The guide stays where it is, `docs/guide/`, the one source. `clm-init` compiles every page into the binary with `include_str!`, as it already does for its templates, and `clm guide` prints the index (each page's name and title) and `clm guide <page>` prints one page. The plugin's shim runs the binary of the plugin's own version, so a skill that says "run `clm guide shaping-the-graph`" gets the page that matches the skills it is reading, with no network and no host. One page, `constraints`, is generated from the carried entries of CLM's `ARCHITECTURE.md` rather than read from `docs/guide/` (`carried-constraints`). `clm guide` is the one way the pages are served to a session; the public site is built from the same files (`guide-from-docs`). `ledger` counts `docs/guide/` as doc-only: the tests that read the pages, from the tree or through `clm guide`, hold `clm:reads` markers, so a guide edit runs those tests (`guide-pages-are-doc-only`).

`init` owns the command: the guide is adoption material, and `clm-init` already embeds files the same way.

Rejected:
- **Move the guide into `plugin/guide/`.** A skill can't reliably name its plugin's install directory from a session's Bash calls, the site would build from the plugin's tree, and this repository's own sessions would read the guide at a path no one expects.
- **Link to the hosted guide (GitHub or the web app).** Needs the network and a host (`host-adapters`), reads `main` rather than the version that runs, and makes the web app or GitHub a dependency of learning the method (`web-optional`).
- **Copy the pages into the plugin at release.** Two copies to keep equal, which `regenerate-dont-merge` would need a check for; the binary already ships with every plugin version.
