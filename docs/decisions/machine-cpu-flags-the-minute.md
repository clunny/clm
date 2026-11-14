---
title: "The CPU turns amber on a minute's queueing and rose once it has held for five"
date: 2026-11-12
status: accepted
---

`clm machine` judges the CPU on both load averages over the logical cores: `warn` when the 1-minute ratio is at least 1.25, `critical` when the 5-minute ratio is at least 1.25, `critical` taking precedence, `ok` below both. It settles the CPU thresholds of `machine-load-measured-by-clm`, which first judged on the 5-minute ratio alone, amber from 1.25 and rose from 2.

A parallel cargo build fills every core, so a ratio near 1 is a machine at work, and 1.25 is where work starts queueing for the cores. The human found the old thresholds flagged too late: a minute of heavy queueing showed nothing, and queueing that held for minutes stayed amber up to twice the cores. The 1-minute average flags queueing as it begins, amber, since it may pass; the 5-minute one turns it rose once it has held long enough that builds are slowed by it. Lowering the 5-minute thresholds alone would still lag a minute's queueing by minutes.
