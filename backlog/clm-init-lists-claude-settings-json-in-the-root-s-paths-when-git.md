---
title: "clm init lists .claude/settings.json in the root's paths when git ignores it"
kind: bug
components: [init]
---

`clm init` writes allow rules to `.claude/settings.json` and adds that path to the root component's `paths`. In a repository whose `.gitignore` ignores `.claude/`, the file is never tracked: the root owns a path no tracked file matches, and the allow rules unattended sessions need exist only on the machine that ran init. Nothing says so; it was found only when the human continued builds on another machine.

Expected: `clm init` checks the path with `git check-ignore` and, when it is ignored, says so and names the fix (for example `.claude/*` with `!.claude/settings.json`), rather than leaving the rules untracked silently.
