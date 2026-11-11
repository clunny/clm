---
status: done
from: backlog/clm-init-lists-claude-settings-json-in-the-root-s-paths-when-git
resolves:
  - clm-init-lists-claude-settings-json-in-the-root-s-paths-when-git
---

## Intent

`clm init` tells the human when git ignores `.claude/settings.json`, the file it seeds with the allow rules unattended sessions need, and names the fix. A new requirement, `init/init-names-ignored-settings`, has every run of `clm init`, first or rerun, ask `git check-ignore -q .claude/settings.json` after its `.gitignore` edits and, when git ignores the file, print a next step before the push step, ahead of any missing-rules step, saying so and naming the fix (`.claude/*` with `!.claude/settings.json`, or `git add -f`). `init`'s Design gains an *Ignored settings* paragraph saying how and why.

## Problem

`clm init` writes its allow rules to `.claude/settings.json` and lists that file in the root component's `paths`. Many projects ignore all of `.claude/`. There the file is never committed: the rules exist only on the machine that ran `clm init`, and unattended builds and drafts on any other clone stop at the classifier for the commands the rules were meant to allow. Nothing says so; the human in the item found out only when they continued builds on another machine. No requirement states any check of this, so the fix is a spec change (`code-fix-needs-a-written-rule`).

## Approaches

1. **Report it in the next steps and leave the patterns alone.** Chosen. Git's own answer decides (`reuse-first`, as the `.gitignore` probes already do), and the human picks the fix: which of `.claude/`'s other files a project keeps out is its own choice, and the pattern to rewrite may sit in a parent directory's ignore file or the clone's excludes.
2. **Have `clm init` un-ignore the file by appending `!.claude/settings.json`.** Rejected: git can't re-include a file whose parent directory is excluded, so with `.claude/` ignored the line does nothing, and making it work means rewriting the project's own pattern to `.claude/*`, an edit to a line `clm init` doesn't own, possibly in a file it doesn't write.
3. **Report it from `clm init --check-permissions` too, so the `build` and `triage` skills catch it before launching.** Rejected for now (`yagni`): it changes that command's exit contract, and on the machine that ran `clm init` a rerun of `clm init` already reports it; on another clone the existing check already reports the rules missing.
4. **Drop `.claude/settings.json` from the root's `paths` when it is ignored.** Rejected: the root still owns the file once the human fixes the ignore, and the bootstrap commit's path ownership doesn't depend on an ignored file.

## Outcome

- `init`'s Design: a new *Ignored settings* paragraph after *Next steps.* under **Allow rules for unattended sessions**.
- `init/init-names-ignored-settings`, added, with five scenarios: the directory ignored on a first run, not ignored, re-included by a negation, tracked though ignored, and ignored on a rerun.
- Code (built by `init`): one `check-ignore` probe of `.claude/settings.json` without `--no-index` in `crates/clm-init/src/init.rs`, after the `.gitignore` edits, with the existing `ignored` probe's exit-code handling (that helper always passes `--no-index`, so it takes a flag or a sibling), and one next step ahead of the missing-rules step and the push step; tests in the crate's existing `clm init` test harness, which already isolates git's global and system configuration.

## Constraints

- No edge cases or workarounds are known. A file only this clone's excludes ignore is reported, which is right, since this clone's commits would leave it out.
- `init-root-ownership` and `init-seeds-permissions` stay as written: the root still owns the file, the rules are still seeded, and the missing-rules step stays the last before the push step, the new step going ahead of it.
- No requirement this one relies on is pending, and no capability test sees a difference: none sets up a repository that ignores `.claude/` (builder check). The plugin's `/clm:init` skill shows the next steps verbatim, so it needs nothing.
- The "any other git failure" clause has no scenario; the builder proves it with a unit test of the probe if it can provoke one, as the `.gitignore` probes' symbolic-link case does.

## Open questions

- None. The architecture audit found no broken constraint, gap or contradicted decision; its one finding, that the new step's place clashed with `init-seeds-permissions`' "last step before the push step", is fixed by putting the new step ahead of the missing-rules step.
