---
title: "Windows gets one x86_64 binary, run by the shim under Git Bash"
date: 2026-10-27
status: superseded
superseded_by: no-windows-support
---

Claude Code on native Windows runs the Bash tool, and plugin hooks written as shell commands, through Git Bash when Git for Windows is installed (Claude Code's setup and hooks documentation, read 2026-10-27). The plugin's `bin/clm` is a POSIX `sh` script, so under Git Bash it runs as it does elsewhere, given a Windows binary to download. The release therefore adds `x86_64-pc-windows-msvc`, built on `windows-latest` with the MSVC tools the image ships.

- One target only. Windows on Arm runs x86_64 binaries under emulation, so the shim sends `aarch64` Git Bash to the x86_64 asset too; an `aarch64-pc-windows-msvc` build would need a cross C toolchain for the tree-sitter grammars and isn't worth it until someone asks.
- The asset is `clm-x86_64-pc-windows-msvc.tar.gz` holding `clm.exe`, not a `.zip`: Git Bash's `tar` reads it, so the shim keeps one unpacking path and the pack step is the same script on every target.
- The shim recognises Git Bash by `uname -s` starting with `MINGW` or `MSYS`. Cygwin isn't mapped: Claude Code doesn't use it.
- Updated 2026-10-27 (`20261027-interactive-panel`): it also recognises MSYS2's UCRT64, CLANG64 and CLANGARM64 shells, whose `uname -s` names their environment (`UCRT64_NT-…`, `CLANG64_NT-…`, `CLANGARM64_NT-…`) rather than starting `MINGW`, and which run the same native `.exe`. They are matched by those three prefixes, not by `*_NT-*`, which would also take Cygwin's `CYGWIN_NT-…`.
- Without Git Bash, Claude Code falls back to PowerShell, where a `sh` script doesn't run. That case stays unsupported; Claude Code's own troubleshooting tells the user to install Git for Windows.
- `clm-init` set the check script's mode with `std::os::unix`, which doesn't build for Windows; it now does so only on Unix and, elsewhere, tells the human to record the mode with `git update-index --chmod=+x` (`init/windows-build`).

Rejected: a PowerShell shim beside the `sh` one (two shims to keep in step, for a setup Claude Code itself discourages); `x86_64-pc-windows-gnu` (needs MinGW on the runner, while MSVC is already there).
