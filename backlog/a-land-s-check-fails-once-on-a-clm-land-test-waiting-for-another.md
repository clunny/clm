---
title: "A land's check fails once on a clm land test waiting for another land's check"
kind: bug
components: [clm]
---

Step: `clm land` in an unattended `clm run` build, while other builds of the same session were landing in parallel on the same clone.
What happened: the first land failed in the clm crate's land test for a land held through the gate on the merge, with a message about waiting for another land's check. A plain retry, with nothing changed, passed.
Expected: the land tests are isolated from other lands running on the same machine, so parallel builds (builds-in-subagents) don't fail each other's checks.
Rule involved: land-locally and builds-in-subagents; parallel builds land concurrently on one clone, so a test that is sensitive to that makes lands fail at random.
