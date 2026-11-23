---
title: "The replica's snapshot leaves out a component's binary assets"
kind: deferred
components: [replica, web.worker.replica]
origin: 20261122-wave-4-loop-by-the-rule
---

`clm` lists every file under `specs/<c>/assets/` in the component's `assets`, with its hash; the replica keeps no bytes for a binary blob (`File` carries `text` only), so a binary asset is missing from its snapshot (`replica/snapshot-matches-clm` states text assets only). Mending it needs the blob's bytes on `File`, a type `clm-web-replica` builds by literal: in steps in wave 6, a constructor first, then the field, then the requirement widened to every asset.
