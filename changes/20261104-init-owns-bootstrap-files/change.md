---
status: applied
---

# Own the files the bootstrap commits

## Intent

`clm init` builds the root's `outside` from the files the index holds when it runs, but the init skill's bootstrap commit then commits everything in the work tree. A file that is new and untracked at that moment, such as the `.claude/settings.json` Claude Code writes when the CLM plugins are enabled for the project, lands in no component's `paths` and no `outside` glob, so the bootstrap fails `check/unowned-files`. This change has `clm init` build `outside` from the files the bootstrap will commit: those the index holds and those untracked that the `.gitignore` it leaves doesn't ignore. It comes from issue #24 (clunny/clm), accepted by the maintainer.

## Problem

`init/init-root-ownership` promises that the committed result of `clm init` passes path ownership, but it reads only the index. The init skill's step 5 commits "everything so far", so any untracked file present at `clm init` time breaks that promise. `.claude/settings.json` is the common case, since enabling the plugins per project writes it, but `.mcp.json`, editor settings or a file the human just created hit the same failure. The same happens when a human runs `clm init` by hand and then commits with `git add -A`.

## Approaches

- **List tracked and untracked files that aren't ignored (chosen).** `git ls-files --cached --others --exclude-standard`, read after `clm init` has written its `.gitignore` lines, so `.clm/`, `clm.local.toml` and `.claude/worktrees/` stay out. It covers every file the bootstrap commits, names no agent's files, and works for the skill and for a `clm init` run by hand.
- **Treat `.claude/` as a spec path, like `CLAUDE.md`.** Rejected: `.claude/settings.json` sets hooks and permissions, so a change to it is code the gate must classify as code. CLM's own repository owns it in the root component's `paths`.
- **Put `.claude/settings.json` in the root's `paths`.** Rejected: it names one agent's file, and misses `.mcp.json` and the rest.
- **Have the skill stage everything before `clm init`.** Rejected: it fixes only the skill's path and leaves `clm init` by hand broken, and the requirement would still read only the index.

## Outcome

- A repository holding an untracked `.claude/settings.json` when `clm init` runs gets `".claude/**"` in the root's `outside`, so the bootstrap commit passes `check/unowned-files`, and the first spec graph adopts it as it adopts the rest of the code.
- An untracked file the `.gitignore` ignores, as `clm init` leaves it, is never listed: a `.clm/` scratch file present before `clm init` writes the ignore line doesn't land in `outside`.
- A file the human then leaves out of the bootstrap commit makes its glob match nothing, which `check/unowned-files` reports with the line that says to remove the glob. That is the right report: the root names code the commit doesn't hold.

## Constraints

- `clm survey` and `clm survey --components` keep reading the index alone (`tracked-files-in-git-layer`); only `clm init`'s `outside` reads the untracked files.
- The listing goes through `clm-git`, as every other git call in `clm-init` does.
