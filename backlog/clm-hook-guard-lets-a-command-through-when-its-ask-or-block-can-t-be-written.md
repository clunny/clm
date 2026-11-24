---
title: "clm hook guard lets a command through when its ask or block can't be written"
kind: bug
components: [clm]
---

`clm-guard`'s call returns 2 when it can't write an ask or a block (`plugin.hooks.guard/commands-in-crate`), but the binary doesn't carry that to the hook's caller. The root's `Printed` writer (crates/clm/src/main.rs) panics on a failed write (`clm/closed-stdout`), so a block whose stderr can't be written exits 101, which no agent harness reads as a block. An ask's JSON has no trailing newline, so it sits in the line-buffered stdout, and the failed flush at exit is dropped: `clm hook guard` on `clm approve spec/x` with stdout on /dev/full, or closed, exits 0, and `clm approve` runs with no prompt (`approval-policy`). Probes: ask with stdout to /dev/full → 0; stdout closed → 0; `git push origin HEAD:main` with stderr to /dev/full → 101; a Codex ask with stderr to /dev/full → 101.
