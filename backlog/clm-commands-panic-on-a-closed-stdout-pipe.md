---
title: "clm commands panic on a closed stdout pipe"
kind: bug
components: [clm]
---

Piping a clm command that prints a lot into a reader that exits early, as `clm context <component> | head` does, ends the command with a Rust panic, "failed printing to stdout: Broken pipe", and a backtrace note, instead of exiting quietly as command-line tools do when their reader goes away. Sessions pipe clm's long output through head routinely, so the panic text lands in their output and reads like a failure of the command. Expected: a closed stdout ends the command without a panic.
