---
title: "`machine-report` states a total of 0 as the code doesn't judge it"
kind: question
components: [machine]
blocks: [machine/machine-report]
choices:
  - "Change the code to the text"
  - "Reword the text to the code"
---

`machine/machine-report` (`ec4a0cdd`) states two things about a reading whose total is 0 that the code doesn't do:

- "Memory SHALL be judged … `unknown` for a reading whose total is 0, whose numbers are printed all the same." `crates/clm-machine/src/system.rs` turns a memory total of 0 from `sysinfo` into no memory reading (`(total > 0).then(...)`), so `clm machine` prints `mem  -  unknown` with `total` and `available` `null`, never `mem  0B available of 0B  unknown`. `judge` and `text` do as the text says for a reading given to them by hand.
- "With no `peak` it SHALL be `critical` below 5% of total and `warn` below 10%, a disk whose total is 0 being `critical`." `judge_disk` divides by `space.total.max(1)`, so a disk reading with total 0 and any bytes available is `ok` or `warn`; only total 0 with 0 available is `critical`. `statvfs` can't give that reading, but `judge` is the pure function the text states.

Found by the spec audit of `machine`'s wave-1 build (`20261119-wave-1-by-the-rule`), which changes no behaviour but the share's syntax, so the code stays as it is and `machine-report` stays behind, its tests citing `5f920ce4`. Either the code changes to the text (a memory total of 0 is a reading, printed; a disk total of 0 is `critical` whatever is available), or the text says that a total of 0 from the system is no reading and drops the disk clause.
