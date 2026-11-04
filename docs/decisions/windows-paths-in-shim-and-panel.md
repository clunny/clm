---
title: "The shim keeps MSYS path conversion; the panel runs the shim through the Git Bash git names"
date: 2026-10-27
status: superseded
superseded_by: no-windows-support
---

`windows-binaries` runs `clm.exe` from the `sh` shim under Git Bash. Two places where a POSIX path meets a native Windows process were left unchecked (`20261027-release-reach`): MSYS's conversion of arguments and environment values when Git Bash execs `clm.exe`, and the panel, a hooks module in Node, starting the `sh` shim.

**The shim leaves MSYS's argument conversion on.** Git Bash rewrites an argument that looks like an absolute POSIX path (`/tmp/body.md`, `/c/Users/u/x`) into the Windows path it names (`C:/Users/u/AppData/Local/Temp/body.md`, `C:/Users/u/x`) when it execs a native program. `clm` takes such paths (`--body-file`, a scratchpad file), and `clm.exe` can open only the converted form, so `MSYS2_ARG_CONV_EXCL='*'` or `MSYS_NO_PATHCONV=1` would break them. The cost is that a text argument starting with `/`, such as a title `/clm:view fails`, may be rewritten too; skills and prompts pass long text through files, and the rest is accepted until a Windows user reports it.

**`CLM_PLUGIN_ROOT` gets no `cygpath -m`.** The shim exported it so `clm run` could pass `--plugin-dir` to the sessions it launched; nothing in `clm` read it any more, and `20261028-drop-clm-plugin-root` removed the export, so there is nothing to convert.

**The panel runs the shim through Git Bash's `bash.exe`, found from `git --exec-path`.** Node's spawn can't start an extensionless `sh` script on Windows. A bare `bash` on PATH is often absent (Git for Windows puts only `Git\cmd` on PATH by default) and, where WSL is installed, is `C:\Windows\System32\bash.exe`, which would run the shim in Linux. `git` is on PATH wherever Claude Code's Bash tool works, and `git --exec-path` prints `<Git>/<mingw64|clangarm64>/libexec/git-core`, three levels below the Git root whose `bin/bash.exe` Claude Code itself uses. The panel recognises Windows by a drive-letter `$.plugin.root`, since a hooks module has no platform call, and normalises `\` to `/`, which Windows accepts, so its path rules stay one set of string functions.

Rejected: a `clm.cmd` or PowerShell shim beside the `sh` one (two shims to keep in step, as `windows-binaries` rejected); having the panel run the cached `clm.exe` itself (it would duplicate the shim's version and cache rules, and can't read `XDG_CACHE_HOME` or `HOME`); `MSYS2_ARG_CONV_EXCL='*'` (breaks every absolute path argument to protect rare text ones).

None of this has run on a Windows machine yet, and that is accepted until after CLM goes public (decided 2026-11-03, `backlog/windows-check-after-going-public`): the check runs once then, or Windows support is cut if keeping it proves costly. The check: on a Windows machine with Git for Windows and Claude Code, in a CLM repository, have the Bash tool run `clm status` and `clm page open ... --body-file <an absolute path>`, and run `/clm:view` with `clm-panel` installed. Report what fails with `clm defect`, for instance `git --exec-path` printing a form the panel doesn't expect, `$.plugin.root` not starting with a drive letter, or the shim, started by the panel through `Git/bin/bash.exe`, missing `uname`, `tar`, `sha256sum` or `gh` on its `PATH`, or `HOME`.
