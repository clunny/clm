---
title: "The glossary still lists clm runner, and the root's paths still name its crate"
kind: deferred
components: [clm]
origin: 20261113-phase-2-delete
---

`20261113-phase-2-delete` removes `clm runner`. `docs/guide/glossary.md` still defines it in two lines (the "Run / runner" entry and the list of working commands). They stay until the root's build has removed the command: `clm/glossary-covers-terms` fails a land while `clm --help` lists a command the glossary lacks, and the glossary is a spec path, which the build's branch can't edit.

The root's `paths` also hold `crates/clm-runner/**`, added by that change so the crate has an owner until the root's build deletes it. Once the crate is gone the glob matches nothing.

Once `clm --help` no longer lists `runner` and `crates/clm-runner/` is gone, one spec branch removes the two glossary mentions, takes the glob out of the root's `paths` with a delta, and deletes this item.
