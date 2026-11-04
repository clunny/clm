---
title: "Rename the data fields that still say node: the views' JSON, the proposal API and clm affected's lines"
kind: idea
components: [view, web, web.worker, plugin.panel, ledger, check.affected, ci, land]
---

`component-words` keeps the word node in the data CLM's own programs exchange, since renaming one is a contract between programs, not a word swap: the views' JSON (`nodes`, `node`, `pending_nodes`, the blockers' kind `node`), which the web app and the panel read; the web app's proposal API and the ledger's `proposed` records (`nodes`); `effort`'s report fields; and `clm affected`'s `node <id>` lines, which `.github/scripts/ci-check.sh` and `clm land` parse.

Each could move to the new word with an expand step: the producer writes both names, the readers move, then the old name goes (`expand-contract`). Worth it only if someone reads these fields by hand; until then the cost is a release-spanning migration for no reader. Weigh it once the term swap (`rename-node-to-component-term-swap`) has landed.
