---
title: "No test runs `clm fingerprint`, the command"
kind: question
components: [gate.fingerprint, clm]
choices:
  - "Give the root a requirement for the command's printing"
  - "Let gate.fingerprint's tests run the clm binary"
---

`gate.fingerprint/fingerprint` says `clm fingerprint --base <ref>` SHALL print what `fingerprint(repo, base)` returns, `empty` included, and the scenarios "nothing differs" and "main ahead of the fork point" are worded against the command. Every test of them calls the library's `fingerprint()`: none runs the command, so breaking its arm in `crates/clm/src/main.rs` (what it prints, the stream, the exit status or the directory it passes as `repo`) fails nothing.

`gate.fingerprint`'s Contract gives the command's arguments, stream and exit status to the root (`crates/clm` routes it and prints what `fingerprint` returns), and `clm-fingerprint`'s tests can't run the `clm` binary, whose crate isn't theirs. A spec change should say where that test lives: a requirement of the root's that states the command's printing, or the scenarios reworded against `fingerprint()` alone.

Found by the code review and the spec audit of `gate.fingerprint`'s wave-1 build (`20261119-wave-1-by-the-rule`), which changes no behaviour.
