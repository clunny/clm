---
title: "The replica keeps marker counts a change to the scanner has made stale"
kind: bug
components: [replica, web.worker.replica]
origin: 20261124-a-marker-above-a
---

The replica holds each file's markers as `clm_coverage::markers_in` counted them when the file was fetched (`Held.markers`, `crates/clm-replica/src/lib.rs`), and a `Tip` change rescans only the files it carries (`replica`'s Design, "Marker index"). When a release changes which markers count, as `20261124-a-marker-above-a` does for a marker on a `describe` block's or a `Test` class's own lines, a file the replica already holds keeps the old count until the file itself changes or the store is rebuilt, so the web app's coverage disagrees with `clm coverage` for the same commit (`replica/snapshot-matches-clm`).

Expected: a replica built by an older scanner is rescanned, or rebuilt, once the Worker runs a newer one, for example by holding the scanner's version beside the store, as wave 6 gives `web.worker.replica`'s `Meta` a version whose absence drops the store.
