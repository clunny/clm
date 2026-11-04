---
status: done
resolves:
  - dev-install-s-move-off-a-directory-marketplace-leaves-the-clone
---

## Intent

`scripts/dev-install.sh` on a clone set up the old way, whose `clm` marketplace is a directory record, runs `claude plugin marketplace remove clm` in the main worktree, and Claude Code then empties the `clm` entries of the tracked `.claude/settings.json` there. The checkout is left dirty, and committing it would stop fresh clones being offered the plugin (`dogfood-from-main`, `dev-install-scoped`). This change states that the setup and its removal leave every tracked file of the clone as it was, and has each `claude plugin marketplace remove` run in `/`, outside every project, where the removal has no project settings to edit.

## Problem

`clm/dev-install` step 5 says what the script runs to move a directory record back to GitHub, but not where, and the script runs every `claude` command in the main worktree. No requirement says the setup leaves tracked files alone, so the fix needs a written rule (`code-fix-needs-a-written-rule`). `dev-install-remove` runs `claude plugin marketplace remove clm-dev` in the main worktree too; the tracked settings name no `clm-dev` entry today, so nothing is lost there yet, but the rule should hold for both.

## What we measured

On Claude Code 2.1.318, under a throwaway `CLAUDE_CONFIG_DIR`, in a scratch git repository holding a copy of the tracked `.claude/settings.json` (which enables `clm@clm` and `clm-panel@clm` and lists `extraKnownMarketplaces.clm` as the `github` source `clunny/clm`), with `clm` added as a directory marketplace at that repository and `clm@clm` installed:

- `claude plugin marketplace remove clm` run in the repository rewrote its `.claude/settings.json`, leaving `"enabledPlugins": {}` and `"extraKnownMarketplaces": {}`, as well as the user settings. This reproduces the item's report and confirms its inferred cause.
- The same removal run in `/` left the repository's `.claude/settings.json` byte for byte as it was, and emptied the same entries in the user settings, uninstalling `clm@clm` as before.
- `claude plugin marketplace add clunny/clm` run in `/` added the GitHub marketplace as it does elsewhere.

Not measured: whether a removal run in a project also edits that project's `.claude/settings.local.json`. The setup rewrites the `clm` entries of that file itself in step 7, and the removal in `--remove` rewrites them before step 5 runs, so neither depends on it.

## Approaches

1. **Run the removal outside every project** (chosen). One `cd /` on each `claude plugin marketplace remove`. Claude Code then has no project settings to edit, so nothing tracked changes and nothing has to be put back. The user-level record, the only thing the removal is for, goes as before.
2. **Restore the tracked file after the removal**, as the item suggested: `git restore .claude/settings.json` when it was clean beforehand. It repairs damage the script caused rather than not causing it, needs a "was it clean" check so it never discards a human's edit, and covers only the one file Claude Code is known to touch today.
3. **Say what changed and leave it to the human.** Leaves the checkout dirty on every old-style clone and invites the wrong commit.

## Outcome

- `clm/dev-install` step 5 runs `claude plugin marketplace remove clm` in `/`, and the requirement states that the setup leaves every tracked file as it was. A new scenario has the stub `claude` empty the entries in `<cwd>/.claude/settings.json` on `plugin marketplace remove`, as the real one does, and checks the tracked tree is clean afterwards.
- `clm/dev-install-remove` runs `claude plugin marketplace remove clm-dev` in `/` as well, with the same rule and a scenario covering both removals.
- `CONTRIBUTING.md`'s list of what the setup changes needs no edit: the tracked settings are no longer among them.

## Constraints

- `claude plugin uninstall <name>@clm-dev --scope local` and `claude plugin install ... --scope local` stay in the main worktree, since local scope belongs to the project they run in.
- `/` is a POSIX path; the script is a POSIX `sh` script for Unix-like systems already.

## Edge cases and workarounds

- A setup that exits in step 6 after step 5 ran in `/` leaves `settings.local.json`'s old `extraKnownMarketplaces.clm` directory entry until the script runs again, which step 6's message already asks for.

## Open questions

None.
