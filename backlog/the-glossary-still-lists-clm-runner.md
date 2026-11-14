---
title: "The glossary still lists clm runner"
kind: deferred
components: [clm]
origin: 20261113-phase-2-delete
---

`20261113-phase-2-delete` removes `clm runner`. `docs/guide/glossary.md` still defines it in two lines (the "Run / runner" entry and the list of working commands). They stay until the root's build has removed the command: `clm/glossary-covers-terms` fails a land while `clm --help` lists a command the glossary lacks, and the glossary is a spec path, which the build's branch can't edit.

Once `clm --help` no longer lists `runner`, a docs-only branch removes the two mentions and deletes this item.
